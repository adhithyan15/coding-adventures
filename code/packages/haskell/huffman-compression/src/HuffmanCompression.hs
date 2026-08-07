module HuffmanCompression
    ( compress
    , decompress
    ) where

import Data.Bits ((.|.), setBit, shiftL, shiftR, testBit)
import qualified Data.ByteString as BS
import Data.Char (intToDigit)
import Data.List (find, foldl', sortOn)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import Data.Word (Word8, Word32)
import qualified HuffmanTree
import Numeric (showIntAtBase)

maxCodeLength :: Int
maxCodeLength = 16

alphabetSize :: Word32
alphabetSize = 256

-- | Compress bytes into the self-contained CMP04 wire format.
compress :: BS.ByteString -> Either String BS.ByteString
compress input
    | BS.null input = Right (encodeWord32 0 <> encodeWord32 0)
    | toInteger (BS.length input) > toInteger (maxBound :: Word32) =
        Left "input is too large for the CMP04 uint32 length field"
    | otherwise = do
        tree <- HuffmanTree.build weightPairs
        let canonical = HuffmanTree.canonicalCodeTable tree
            entries =
                sortOn
                    (\(byte, codeLength) -> (codeLength, byte))
                    [ (fromIntegral symbol, length code)
                    | (symbol, code) <- Map.toList canonical
                    ]
        case find ((> maxCodeLength) . snd) entries of
            Just (byte, codeLength) ->
                Left
                    ( "Huffman code for symbol "
                        ++ show byte
                        ++ " exceeds the 16-bit CMP04 limit: "
                        ++ show codeLength
                    )
            Nothing -> do
                bitString <- encodeInput canonical (BS.unpack input)
                let header =
                        encodeWord32 (fromIntegral (BS.length input))
                            <> encodeWord32 (fromIntegral (length entries))
                    lengthTable =
                        BS.pack
                            (concatMap (\(byte, codeLength) -> [byte, fromIntegral codeLength]) entries)
                Right (header <> lengthTable <> packBitsLsbFirst bitString)
  where
    frequencies =
        BS.foldl'
            (\counts byte -> Map.insertWith (+) byte 1 counts)
            Map.empty
            input
    weightPairs =
        [ HuffmanTree.WeightPair (fromIntegral byte) count
        | (byte, count) <- Map.toList frequencies
        ]

-- | Decompress one CMP04 payload, rejecting malformed headers and code tables.
decompress :: BS.ByteString -> Either String BS.ByteString
decompress input
    | BS.length input < 8 = Left "truncated CMP04 header: expected at least 8 bytes"
    | otherwise = do
        originalLength <- decodeWord32 input 0
        symbolCount <- decodeWord32 input 4
        if symbolCount > alphabetSize
            then Left "symbol count exceeds the 256-byte alphabet"
            else do
                let count = fromIntegral symbolCount
                    tableEnd = 8 + 2 * count
                if BS.length input < tableEnd
                    then Left "truncated CMP04 code-length table"
                    else do
                        let entries =
                                [ (BS.index input (8 + 2 * index), fromIntegral (BS.index input (9 + 2 * index)))
                                | index <- [0 .. count - 1]
                                ]
                        validateHeader originalLength symbolCount
                        if originalLength == 0
                            then Right BS.empty
                            else do
                                decodeTable <- canonicalDecodeTable entries
                                let maxLength = maximum (map snd entries)
                                    bits = unpackBitsLsbFirst (BS.drop tableEnd input)
                                decoded <- decodeSymbols originalLength maxLength decodeTable bits
                                Right (BS.pack decoded)

validateHeader :: Word32 -> Word32 -> Either String ()
validateHeader originalLength symbolCount
    | originalLength == 0 && symbolCount /= 0 =
        Left "zero-length payload must have a zero symbol count"
    | originalLength /= 0 && symbolCount == 0 =
        Left "non-empty payload must have at least one symbol"
    | otherwise = Right ()

canonicalDecodeTable :: [(Word8, Int)] -> Either String (Map String Word8)
canonicalDecodeTable [] = Left "non-empty payload has an empty code-length table"
canonicalDecodeTable entries@((_, firstLength) : _) = do
    case find ((== 0) . snd) entries of
        Just (byte, _) -> Left ("code length is zero for symbol " ++ show byte)
        Nothing -> Right ()
    case find ((> maxCodeLength) . snd) entries of
        Just (byte, codeLength) ->
            Left
                ( "code length exceeds 16 bits for symbol "
                    ++ show byte
                    ++ ": "
                    ++ show codeLength
                )
        Nothing -> Right ()
    let symbols = map fst entries
    if Set.size (Set.fromList symbols) /= length symbols
        then Left "duplicate symbol in CMP04 code-length table"
        else Right ()
    if entries /= sortOn (\(byte, codeLength) -> (codeLength, byte)) entries
        then Left "CMP04 code-length table is not sorted by length and symbol"
        else do
            codes <- assignCanonicalCodes 0 firstLength entries
            Right (Map.fromList [(bits, byte) | (byte, bits) <- codes])

assignCanonicalCodes :: Integer -> Int -> [(Word8, Int)] -> Either String [(Word8, String)]
assignCanonicalCodes _ _ [] = Right []
assignCanonicalCodes current previousLength ((byte, codeLength) : rest) = do
    let shifted =
            if codeLength > previousLength
                then current `shiftL` (codeLength - previousLength)
                else current
        limit = (1 :: Integer) `shiftL` codeLength
    if shifted >= limit
        then Left "oversubscribed canonical Huffman code lengths"
        else do
            let code = leftPad codeLength (toBinary shifted)
            remaining <- assignCanonicalCodes (shifted + 1) codeLength rest
            Right ((byte, code) : remaining)

encodeInput :: Map Int String -> [Word8] -> Either String String
encodeInput table = fmap concat . traverse lookupCode
  where
    lookupCode byte =
        case Map.lookup (fromIntegral byte) table of
            Nothing -> Left ("internal error: missing Huffman code for symbol " ++ show byte)
            Just code -> Right code

decodeSymbols :: Word32 -> Int -> Map String Word8 -> String -> Either String [Word8]
decodeSymbols expected maxLength table = go expected 0 "" []
  where
    go 0 _ _ decoded _ = Right (reverse decoded)
    go remaining decodedCount accumulated decoded bits =
        case bits of
            [] ->
                Left
                    ( "bit stream exhausted after "
                        ++ show decodedCount
                        ++ " symbols; expected "
                        ++ show expected
                    )
            bit : more ->
                let candidate = accumulated ++ [bit]
                 in case Map.lookup candidate table of
                        Just byte -> go (remaining - 1) (decodedCount + 1) "" (byte : decoded) more
                        Nothing
                            | length candidate >= maxLength ->
                                Left "invalid prefix in CMP04 bit stream"
                            | otherwise ->
                                go remaining decodedCount candidate decoded more

packBitsLsbFirst :: String -> BS.ByteString
packBitsLsbFirst bits = BS.pack (reverse (finish (foldl' step ([], 0, 0) bits)))
  where
    step :: ([Word8], Word8, Int) -> Char -> ([Word8], Word8, Int)
    step (output, buffer, bitPosition) bit =
        let nextBuffer = if bit == '1' then setBit buffer bitPosition else buffer
         in if bitPosition == 7
                then (nextBuffer : output, 0, 0)
                else (output, nextBuffer, bitPosition + 1)

    finish :: ([Word8], Word8, Int) -> [Word8]
    finish (output, _, 0) = output
    finish (output, buffer, _) = buffer : output

unpackBitsLsbFirst :: BS.ByteString -> String
unpackBitsLsbFirst = concatMap byteBits . BS.unpack
  where
    byteBits byte =
        [ if testBit byte bitPosition then '1' else '0'
        | bitPosition <- [0 .. 7]
        ]

encodeWord32 :: Word32 -> BS.ByteString
encodeWord32 value =
    BS.pack
        [ fromIntegral (value `shiftR` 24)
        , fromIntegral (value `shiftR` 16)
        , fromIntegral (value `shiftR` 8)
        , fromIntegral value
        ]

decodeWord32 :: BS.ByteString -> Int -> Either String Word32
decodeWord32 input offset
    | BS.length input < offset + 4 = Left "truncated uint32 field"
    | otherwise =
        Right
            ( byteAt 0 `shiftL` 24
                .|. byteAt 1 `shiftL` 16
                .|. byteAt 2 `shiftL` 8
                .|. byteAt 3
            )
  where
    byteAt index = fromIntegral (BS.index input (offset + index))

toBinary :: Integer -> String
toBinary 0 = "0"
toBinary value = showIntAtBase 2 intToDigit value ""

leftPad :: Int -> String -> String
leftPad width value = replicate (max 0 (width - length value)) '0' ++ value
