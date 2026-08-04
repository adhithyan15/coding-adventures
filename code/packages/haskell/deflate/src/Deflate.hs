-- | CMP05 educational DEFLATE composition for strict byte strings.
--
-- This is the repository's self-describing teaching format, not an RFC 1951
-- bit stream. It combines CMP02 LZSS tokens with canonical Huffman codes.
module Deflate
    ( compress
    , compressWith
    , decompress
    ) where

import Control.Monad (unless, when)
import Data.Bits (setBit, shiftL, shiftR, testBit)
import qualified Data.ByteString as BS
import Data.Char (intToDigit)
import Data.Foldable (toList)
import Data.List (find, foldl', sortOn)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Sequence (Seq, (|>))
import qualified Data.Sequence as Seq
import Data.Word (Word8, Word16, Word32)
import qualified HuffmanTree
import qualified LZSS
import Numeric (showIntAtBase)

type CodeEntry = (Int, Int)

lengthTable :: [(Int, Int, Int)]
lengthTable =
    [ (257, 3, 0), (258, 4, 0), (259, 5, 0), (260, 6, 0)
    , (261, 7, 0), (262, 8, 0), (263, 9, 0), (264, 10, 0)
    , (265, 11, 1), (266, 13, 1), (267, 15, 1), (268, 17, 1)
    , (269, 19, 2), (270, 23, 2), (271, 27, 2), (272, 31, 2)
    , (273, 35, 3), (274, 43, 3), (275, 51, 3), (276, 59, 3)
    , (277, 67, 4), (278, 83, 4), (279, 99, 4), (280, 115, 4)
    , (281, 131, 5), (282, 163, 5), (283, 195, 5), (284, 227, 5)
    ]

distanceTable :: [(Int, Int, Int)]
distanceTable =
    [ (0, 1, 0), (1, 2, 0), (2, 3, 0), (3, 4, 0)
    , (4, 5, 1), (5, 7, 1), (6, 9, 2), (7, 13, 2)
    , (8, 17, 3), (9, 25, 3), (10, 33, 4), (11, 49, 4)
    , (12, 65, 5), (13, 97, 5), (14, 129, 6), (15, 193, 6)
    , (16, 257, 7), (17, 385, 7), (18, 513, 8), (19, 769, 8)
    , (20, 1025, 9), (21, 1537, 9), (22, 2049, 10), (23, 3073, 10)
    ]

-- | Compress with the CMP05 default LZSS parameters.
compress :: BS.ByteString -> Either String BS.ByteString
compress =
    compressWith
        LZSS.defaultWindowSize
        LZSS.defaultMaxMatch
        LZSS.defaultMinMatch

-- | Compress with explicit LZSS parameters supported by the CMP05 tables.
compressWith :: Int -> Int -> Int -> BS.ByteString -> Either String BS.ByteString
compressWith windowSize maxMatch minMatch input = do
    when (windowSize > 4096) (Left "CMP05 window size must not exceed 4096")
    when (minMatch < 3) (Left "CMP05 minimum match length must be at least 3")
    originalLength <- intToWord32 "original length" (BS.length input)
    tokens <- LZSS.encodeWith windowSize maxMatch minMatch input
    let (literalLengthFrequencies, distanceFrequencies) =
            foldl' addTokenFrequency (Map.singleton 256 1, Map.empty) tokens
    literalLengthCodes <- buildCodes literalLengthFrequencies
    distanceCodes <-
        if Map.null distanceFrequencies
            then Right Map.empty
            else buildCodes distanceFrequencies
    literalLengthEntries <- codeEntries literalLengthCodes
    distanceEntries <- codeEntries distanceCodes
    payloadBits <- encodeTokens literalLengthCodes distanceCodes tokens
    let header =
            encodeWord32 originalLength
                <> encodeWord16 (fromIntegral (length literalLengthEntries))
                <> encodeWord16 (fromIntegral (length distanceEntries))
        tables =
            BS.concat
                (map encodeCodeEntry literalLengthEntries ++ map encodeCodeEntry distanceEntries)
    Right (header <> tables <> packBitsLsbFirst payloadBits)

addTokenFrequency :: (Map Int Int, Map Int Int) -> LZSS.Token -> (Map Int Int, Map Int Int)
addTokenFrequency (literalLengths, distances) token =
    case token of
        LZSS.Literal byte ->
            (Map.insertWith (+) (fromIntegral byte) 1 literalLengths, distances)
        LZSS.Match offset lengthValue ->
            let (lengthSymbol, _, _) = findEncodingEntry lengthTable (fromIntegral lengthValue)
                (distanceCode, _, _) = findEncodingEntry distanceTable (fromIntegral offset)
             in ( Map.insertWith (+) lengthSymbol 1 literalLengths
                , Map.insertWith (+) distanceCode 1 distances
                )

buildCodes :: Map Int Int -> Either String (Map Int String)
buildCodes frequencies = do
    tree <-
        HuffmanTree.build
            [ HuffmanTree.WeightPair symbol frequency
            | (symbol, frequency) <- Map.toList frequencies
            ]
    Right (HuffmanTree.canonicalCodeTable tree)

codeEntries :: Map Int String -> Either String [CodeEntry]
codeEntries codes = do
    let entries = sortOn (\(symbol, codeLength) -> (codeLength, symbol))
            [(symbol, length code) | (symbol, code) <- Map.toList codes]
    case find ((> 255) . snd) entries of
        Just (symbol, codeLength) ->
            Left
                ( "Huffman code for symbol "
                    ++ show symbol
                    ++ " exceeds the uint8 length field: "
                    ++ show codeLength
                )
        Nothing -> Right entries

encodeTokens :: Map Int String -> Map Int String -> [LZSS.Token] -> Either String [Bool]
encodeTokens literalLengthCodes distanceCodes tokens = do
    tokenBits <- fmap concat (traverse encodeToken tokens)
    endCode <- lookupCode "literal/length" 256 literalLengthCodes
    Right (tokenBits ++ codeBits endCode)
  where
    encodeToken (LZSS.Literal byte) =
        codeBits <$> lookupCode "literal/length" (fromIntegral byte) literalLengthCodes
    encodeToken (LZSS.Match offsetWord lengthWord) = do
        let offset = fromIntegral offsetWord
            lengthValue = fromIntegral lengthWord
            (lengthSymbol, lengthBase, lengthExtra) = findEncodingEntry lengthTable lengthValue
            (distanceCode, distanceBase, distanceExtra) = findEncodingEntry distanceTable offset
        lengthCode <- lookupCode "literal/length" lengthSymbol literalLengthCodes
        distanceCodeBits <- lookupCode "distance" distanceCode distanceCodes
        Right
            ( codeBits lengthCode
                ++ rawBits (lengthValue - lengthBase) lengthExtra
                ++ codeBits distanceCodeBits
                ++ rawBits (offset - distanceBase) distanceExtra
            )

lookupCode :: String -> Int -> Map Int String -> Either String String
lookupCode label symbol codes =
    maybe
        (Left ("internal error: missing " ++ label ++ " Huffman code for symbol " ++ show symbol))
        Right
        (Map.lookup symbol codes)

findEncodingEntry :: [(Int, Int, Int)] -> Int -> (Int, Int, Int)
findEncodingEntry table value =
    case find covers table of
        Just entry -> entry
        Nothing -> last table
  where
    covers (_, baseValue, extraBits) =
        value <= baseValue + ((1 `shiftL` extraBits) - 1)

-- | Decompress one strict CMP05 payload.
decompress :: BS.ByteString -> Either String BS.ByteString
decompress input = do
    when (BS.length input < 8) (Left "truncated CMP05 header")
    originalLengthWord <- decodeWord32 input 0
    literalLengthCountWord <- decodeWord16 input 4
    distanceCountWord <- decodeWord16 input 6
    originalLength <- word32ToInt "original length" originalLengthWord
    let literalLengthCount = fromIntegral literalLengthCountWord
        distanceCount = fromIntegral distanceCountWord
    when (literalLengthCount > 285) (Left "literal/length table exceeds the CMP05 alphabet")
    when (distanceCount > 24) (Left "distance table exceeds the CMP05 alphabet")
    (literalLengthEntries, afterLiteralLengths) <-
        parseCodeEntries "literal/length" literalLengthCount 8 input
    (distanceEntries, payloadOffset) <-
        parseCodeEntries "distance" distanceCount afterLiteralLengths input
    validateLiteralLengthEntries literalLengthEntries
    validateDistanceEntries distanceEntries
    literalLengthCodes <- reconstructCanonicalCodes "literal/length" literalLengthEntries
    distanceCodes <- reconstructCanonicalCodes "distance" distanceEntries
    decodePayload
        originalLength
        literalLengthCodes
        distanceCodes
        (unpackBitsLsbFirst (BS.drop payloadOffset input))

parseCodeEntries :: String -> Int -> Int -> BS.ByteString -> Either String ([CodeEntry], Int)
parseCodeEntries label count start input = go count start []
  where
    go 0 position entries = Right (reverse entries, position)
    go remaining position entries
        | BS.length input < position + 3 = Left ("truncated CMP05 " ++ label ++ " code-length table")
        | otherwise = do
            symbol <- decodeWord16 input position
            let codeLength = fromIntegral (BS.index input (position + 2))
            go (remaining - 1) (position + 3) ((fromIntegral symbol, codeLength) : entries)

validateLiteralLengthEntries :: [CodeEntry] -> Either String ()
validateLiteralLengthEntries entries = do
    unless (any ((== 256) . fst) entries) (Left "literal/length table is missing the end-of-block symbol")
    case find (\(symbol, _) -> symbol < 0 || symbol > 284) entries of
        Just (symbol, _) -> Left ("unknown literal/length symbol " ++ show symbol)
        Nothing -> Right ()

validateDistanceEntries :: [CodeEntry] -> Either String ()
validateDistanceEntries entries =
    case find (\(symbol, _) -> symbol < 0 || symbol > 23) entries of
        Just (symbol, _) -> Left ("unknown distance symbol " ++ show symbol)
        Nothing -> Right ()

reconstructCanonicalCodes :: String -> [CodeEntry] -> Either String (Map String Int)
reconstructCanonicalCodes label entries = do
    case find ((== 0) . snd) entries of
        Just (symbol, _) -> Left (label ++ " code length is zero for symbol " ++ show symbol)
        Nothing -> Right ()
    let symbols = map fst entries
    unless
        (Map.size (Map.fromList [(symbol, ()) | symbol <- symbols]) == length symbols)
        (Left ("duplicate symbol in CMP05 " ++ label ++ " table"))
    unless
        (entries == sortOn (\(symbol, codeLength) -> (codeLength, symbol)) entries)
        (Left ("CMP05 " ++ label ++ " table is not sorted by length and symbol"))
    case entries of
        [] -> Right Map.empty
        [(symbol, codeLength)] ->
            if codeLength == 1
                then Right (Map.singleton "0" symbol)
                else Left ("single-symbol " ++ label ++ " table must use code length 1")
        (_, firstLength) : _ -> do
            assigned <- assignCanonicalCodes label 0 firstLength entries
            Right (Map.fromList [(bits, symbol) | (symbol, bits) <- assigned])

assignCanonicalCodes :: String -> Integer -> Int -> [CodeEntry] -> Either String [(Int, String)]
assignCanonicalCodes _ _ _ [] = Right []
assignCanonicalCodes label current previousLength ((symbol, codeLength) : rest) = do
    let shifted =
            if codeLength > previousLength
                then current `shiftL` (codeLength - previousLength)
                else current
        limit = (1 :: Integer) `shiftL` codeLength
    when (shifted >= limit) (Left ("oversubscribed CMP05 " ++ label ++ " code lengths"))
    remaining <- assignCanonicalCodes label (shifted + 1) codeLength rest
    Right ((symbol, leftPad codeLength (toBinary shifted)) : remaining)

decodePayload :: Int -> Map String Int -> Map String Int -> [Bool] -> Either String BS.ByteString
decodePayload originalLength literalLengthCodes distanceCodes = go Seq.empty
  where
    go output bits = do
        (symbol, afterSymbol) <- nextHuffmanSymbol "literal/length" literalLengthCodes bits
        if symbol == 256
            then
                if Seq.length output == originalLength
                    then Right (BS.pack (toList output))
                    else
                        Left
                            ( "decoded length "
                                ++ show (Seq.length output)
                                ++ " does not match CMP05 header length "
                                ++ show originalLength
                            )
            else
                if symbol < 256
                    then do
                        when (Seq.length output >= originalLength) (Left "decoded data exceeds the CMP05 header length")
                        go (output |> fromIntegral symbol) afterSymbol
                    else do
                        (_, lengthBase, lengthExtra) <- lookupDecodingEntry "length" lengthTable symbol
                        (lengthDelta, afterLength) <- readRawBits lengthExtra afterSymbol
                        when (Map.null distanceCodes) (Left "compressed stream references a missing distance tree")
                        (distanceSymbol, afterDistanceSymbol) <-
                            nextHuffmanSymbol "distance" distanceCodes afterLength
                        (_, distanceBase, distanceExtra) <-
                            lookupDecodingEntry "distance" distanceTable distanceSymbol
                        (distanceDelta, afterDistance) <- readRawBits distanceExtra afterDistanceSymbol
                        copied <- copyMatch (distanceBase + distanceDelta) (lengthBase + lengthDelta) output
                        when (Seq.length copied > originalLength) (Left "decoded data exceeds the CMP05 header length")
                        go copied afterDistance

lookupDecodingEntry :: String -> [(Int, Int, Int)] -> Int -> Either String (Int, Int, Int)
lookupDecodingEntry label table symbol =
    maybe
        (Left ("unknown " ++ label ++ " symbol " ++ show symbol))
        Right
        (find (\(entrySymbol, _, _) -> entrySymbol == symbol) table)

copyMatch :: Int -> Int -> Seq Word8 -> Either String (Seq Word8)
copyMatch distance lengthValue output
    | distance <= 0 = Left "CMP05 distance must be positive"
    | distance > Seq.length output = Left "CMP05 distance extends before the output buffer"
    | otherwise = copyFrom (Seq.length output - distance) 0 output
  where
    copyFrom _ copied current | copied >= lengthValue = Right current
    copyFrom start copied current =
        let sourceIndex = start + copied
         in if sourceIndex < 0 || sourceIndex >= Seq.length current
                then Left "CMP05 backreference points outside the output buffer"
                else copyFrom start (copied + 1) (current |> Seq.index current sourceIndex)

nextHuffmanSymbol :: String -> Map String Int -> [Bool] -> Either String (Int, [Bool])
nextHuffmanSymbol label codes bits
    | Map.null codes = Left ("missing CMP05 " ++ label ++ " Huffman codes")
    | otherwise = go "" bits
  where
    maximumLength = maximum (map length (Map.keys codes))

    go _ [] = Left "unexpected end of CMP05 compressed bit stream"
    go prefix (bit : remaining) =
        let candidate = prefix ++ [if bit then '1' else '0']
         in case Map.lookup candidate codes of
                Just symbol -> Right (symbol, remaining)
                Nothing
                    | length candidate >= maximumLength ->
                        Left ("invalid prefix in CMP05 " ++ label ++ " bit stream")
                    | otherwise -> go candidate remaining

readRawBits :: Int -> [Bool] -> Either String (Int, [Bool])
readRawBits count bits =
    let (valueBits, remaining) = splitAt count bits
     in if length valueBits /= count
            then Left "unexpected end of CMP05 compressed bit stream"
            else
                Right
                    ( foldl'
                        (\value (bitPosition, bit) -> if bit then setBit value bitPosition else value)
                        0
                        (zip [0 ..] valueBits)
                    , remaining
                    )

codeBits :: String -> [Bool]
codeBits = map (== '1')

rawBits :: Int -> Int -> [Bool]
rawBits value count = [testBit value bitPosition | bitPosition <- [0 .. count - 1]]

packBitsLsbFirst :: [Bool] -> BS.ByteString
packBitsLsbFirst bits = BS.pack (reverse (finish (foldl' step ([], 0, 0) bits)))
  where
    step :: ([Word8], Word8, Int) -> Bool -> ([Word8], Word8, Int)
    step (output, buffer, bitPosition) bit =
        let nextBuffer = if bit then setBit buffer bitPosition else buffer
         in if bitPosition == 7
                then (nextBuffer : output, 0, 0)
                else (output, nextBuffer, bitPosition + 1)

    finish :: ([Word8], Word8, Int) -> [Word8]
    finish (output, _, 0) = output
    finish (output, buffer, _) = buffer : output

unpackBitsLsbFirst :: BS.ByteString -> [Bool]
unpackBitsLsbFirst = concatMap byteBits . BS.unpack
  where
    byteBits byte = [testBit byte bitPosition | bitPosition <- [0 .. 7]]

encodeCodeEntry :: CodeEntry -> BS.ByteString
encodeCodeEntry (symbol, codeLength) =
    encodeWord16 (fromIntegral symbol) <> BS.singleton (fromIntegral codeLength)

encodeWord16 :: Word16 -> BS.ByteString
encodeWord16 value =
    BS.pack
        [ fromIntegral (value `shiftR` 8)
        , fromIntegral value
        ]

decodeWord16 :: BS.ByteString -> Int -> Either String Word16
decodeWord16 input offset
    | BS.length input < offset + 2 = Left "truncated uint16 field"
    | otherwise =
        Right
            ( (fromIntegral (BS.index input offset) `shiftL` 8)
                + fromIntegral (BS.index input (offset + 1))
            )

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
            ( (fromIntegral (BS.index input offset) `shiftL` 24)
                + (fromIntegral (BS.index input (offset + 1)) `shiftL` 16)
                + (fromIntegral (BS.index input (offset + 2)) `shiftL` 8)
                + fromIntegral (BS.index input (offset + 3))
            )

intToWord32 :: String -> Int -> Either String Word32
intToWord32 label value
    | value < 0 = Left (label ++ " must not be negative")
    | toInteger value > toInteger (maxBound :: Word32) = Left (label ++ " exceeds the uint32 field")
    | otherwise = Right (fromIntegral value)

word32ToInt :: String -> Word32 -> Either String Int
word32ToInt label value
    | toInteger value > toInteger (maxBound :: Int) = Left (label ++ " exceeds this platform's Int range")
    | otherwise = Right (fromIntegral value)

toBinary :: Integer -> String
toBinary 0 = "0"
toBinary value = showIntAtBase 2 intToDigit value ""

leftPad :: Int -> String -> String
leftPad width value = replicate (max 0 (width - length value)) '0' ++ value
