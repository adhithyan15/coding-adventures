-- | CMP02 LZSS sliding-window compression for strict byte strings.
module LZSS
    ( Token (..)
    , defaultWindowSize
    , defaultMaxMatch
    , defaultMinMatch
    , encode
    , encodeWith
    , decode
    , decodeWithLength
    , serialiseTokens
    , deserialiseTokens
    , compress
    , compressWith
    , decompress
    ) where

import Control.Monad (foldM)
import Data.Bits ((.&.), (.|.), setBit, shiftL, shiftR)
import qualified Data.ByteString as BS
import Data.Foldable (toList)
import Data.Sequence (Seq, (|>))
import qualified Data.Sequence as Seq
import Data.Word (Word8, Word16, Word32)

-- | One LZSS symbol: either a byte or a backreference with no trailing byte.
data Token
    = Literal
        { literalByte :: Word8
        }
    | Match
        { matchOffset :: Word16
        , matchLength :: Word8
        }
    deriving (Eq, Show)

defaultWindowSize :: Int
defaultWindowSize = 4096

defaultMaxMatch :: Int
defaultMaxMatch = 255

defaultMinMatch :: Int
defaultMinMatch = 3

-- | Encode with the CMP02 default window and match thresholds.
encode :: BS.ByteString -> [Token]
encode = encodeUnchecked defaultWindowSize defaultMaxMatch defaultMinMatch

-- | Encode with explicit parameters that must fit the CMP02 wire fields.
encodeWith :: Int -> Int -> Int -> BS.ByteString -> Either String [Token]
encodeWith windowSize maxMatch minMatch input = do
    validateParameters windowSize maxMatch minMatch
    Right (encodeUnchecked windowSize maxMatch minMatch input)

encodeUnchecked :: Int -> Int -> Int -> BS.ByteString -> [Token]
encodeUnchecked windowSize maxMatch minMatch input = go 0
  where
    inputLength = BS.length input

    go cursor
        | cursor >= inputLength = []
        | bestLength >= minMatch =
            Match (fromIntegral bestOffset) (fromIntegral bestLength)
                : go (cursor + bestLength)
        | otherwise =
            Literal (BS.index input cursor) : go (cursor + 1)
      where
        (bestOffset, bestLength) =
            findLongestMatch input cursor windowSize maxMatch

findLongestMatch :: BS.ByteString -> Int -> Int -> Int -> (Int, Int)
findLongestMatch input cursor windowSize maxMatch =
    foldl chooseBetter (0, 0) [searchStart .. cursor - 1]
  where
    searchStart = max 0 (cursor - windowSize)
    lookaheadEnd = min (cursor + maxMatch) (BS.length input)

    chooseBetter best@(_, bestLength) position
        | candidateLength > bestLength = (cursor - position, candidateLength)
        | otherwise = best
      where
        candidateLength = measureMatch position 0

    measureMatch position matched
        | cursor + matched >= lookaheadEnd = matched
        | BS.index input (position + matched) /= BS.index input (cursor + matched) = matched
        | otherwise = measureMatch position (matched + 1)

-- | Decode every token, including self-referential overlapping matches.
decode :: [Token] -> Either String BS.ByteString
decode tokens = do
    output <- foldM decodeToken Seq.empty tokens
    Right (BS.pack (toList output))

-- | Decode and return exactly the length stored in a CMP02 header.
decodeWithLength :: Int -> [Token] -> Either String BS.ByteString
decodeWithLength originalLength tokens
    | originalLength < 0 = Left "original length must not be negative"
    | otherwise = do
        output <- decode tokens
        if BS.length output < originalLength
            then Left "decoded LZSS stream is shorter than the original length"
            else Right (BS.take originalLength output)

decodeToken :: Seq Word8 -> Token -> Either String (Seq Word8)
decodeToken output token = do
    validateToken token
    case token of
        Literal byte -> Right (output |> byte)
        Match offsetWord lengthWord ->
            let offset = fromIntegral offsetWord
                phraseLength = fromIntegral lengthWord
                outputLength = Seq.length output
             in if offset > outputLength
                    then
                        Left
                            ( "LZSS offset "
                                ++ show offset
                                ++ " exceeds decoded prefix length "
                                ++ show outputLength
                            )
                    else copyMatch (outputLength - offset) phraseLength 0 output

copyMatch :: Int -> Int -> Int -> Seq Word8 -> Either String (Seq Word8)
copyMatch _ phraseLength copied output
    | copied >= phraseLength = Right output
copyMatch start phraseLength copied output =
    let sourceIndex = start + copied
     in if sourceIndex < 0 || sourceIndex >= Seq.length output
            then Left "LZSS backreference points outside the decoded prefix"
            else
                copyMatch
                    start
                    phraseLength
                    (copied + 1)
                    (output |> Seq.index output sourceIndex)

-- | Serialise tokens as CMP02 flag blocks after an eight-byte header.
serialiseTokens :: Int -> [Token] -> Either String BS.ByteString
serialiseTokens originalLength tokens = do
    originalLengthWord <- intToWord32 "original length" originalLength
    traverse_ validateToken tokens
    let blocks = chunksOfEight tokens
    blockCountWord <- integerToWord32 "block count" (toInteger (length blocks))
    Right
        ( encodeWord32 originalLengthWord
            <> encodeWord32 blockCountWord
            <> BS.pack (concatMap serialiseBlock blocks)
        )

serialiseBlock :: [Token] -> [Word8]
serialiseBlock tokens = flag : concatMap serialiseToken tokens
  where
    flag = foldl markMatch 0 (zip [0 ..] tokens)

    markMatch value (bit, Match {}) = setBit value bit
    markMatch value _ = value

    serialiseToken (Literal byte) = [byte]
    serialiseToken (Match offset lengthValue) =
        [ fromIntegral (offset `shiftR` 8)
        , fromIntegral offset
        , lengthValue
        ]

chunksOfEight :: [value] -> [[value]]
chunksOfEight [] = []
chunksOfEight values =
    let (chunk, rest) = splitAt 8 values
     in chunk : chunksOfEight rest

-- | Parse one strict CMP02 payload and reject malformed or trailing data.
deserialiseTokens :: BS.ByteString -> Either String ([Token], Int)
deserialiseTokens input
    | BS.length input < 8 = Left "truncated LZSS header"
    | otherwise = do
        originalLengthWord <- decodeWord32 input 0
        blockCountWord <- decodeWord32 input 4
        originalLength <- word32ToInt "original length" originalLengthWord
        blockCount <- word32ToInt "block count" blockCountWord
        let payloadLength = BS.length input - 8
        if blockCount > payloadLength
            then Left "LZSS block count exceeds the available payload"
            else do
                (tokens, finalPosition) <- parseBlocks blockCount 8 []
                if finalPosition /= BS.length input
                    then Left "trailing bytes after declared LZSS blocks"
                    else Right (reverse tokens, originalLength)
  where
    inputLength = BS.length input

    parseBlocks :: Int -> Int -> [Token] -> Either String ([Token], Int)
    parseBlocks 0 position tokens = Right (tokens, position)
    parseBlocks remaining position tokens
        | position >= inputLength = Left "truncated LZSS block flag"
        | otherwise =
            parseSymbols
                (remaining == 1)
                (BS.index input position)
                0
                (position + 1)
                0
                tokens
                >>= \(nextPosition, nextTokens) ->
                    parseBlocks (remaining - 1) nextPosition nextTokens

    parseSymbols :: Bool -> Word8 -> Int -> Int -> Int -> [Token] -> Either String (Int, [Token])
    parseSymbols isLast flag bit position symbolsInBlock tokens
        | bit >= 8 = Right (position, tokens)
        | position >= inputLength =
            if isLast && symbolsInBlock > 0 && flag `shiftR` bit == 0
                then Right (position, tokens)
                else
                    if isLast && flag `shiftR` bit /= 0
                        then Left "truncated LZSS match record"
                        else Left "truncated LZSS symbol block"
        | flag `hasBit` bit =
            if position + 3 > inputLength
                then Left "truncated LZSS match record"
                else do
                    let offset =
                            (fromIntegral (BS.index input position) `shiftL` 8)
                                .|. fromIntegral (BS.index input (position + 1))
                        token = Match offset (BS.index input (position + 2))
                    validateToken token
                    parseSymbols
                        isLast
                        flag
                        (bit + 1)
                        (position + 3)
                        (symbolsInBlock + 1)
                        (token : tokens)
        | otherwise =
            parseSymbols
                isLast
                flag
                (bit + 1)
                (position + 1)
                (symbolsInBlock + 1)
                (Literal (BS.index input position) : tokens)

    hasBit value bit = (value `shiftR` bit) .&. 1 == 1

-- | Compress with CMP02 defaults.
compress :: BS.ByteString -> Either String BS.ByteString
compress input = serialiseTokens (BS.length input) (encode input)

-- | Compress with explicit window size, maximum match, and minimum match.
compressWith :: Int -> Int -> Int -> BS.ByteString -> Either String BS.ByteString
compressWith windowSize maxMatch minMatch input = do
    tokens <- encodeWith windowSize maxMatch minMatch input
    serialiseTokens (BS.length input) tokens

-- | Decompress one strict CMP02 payload.
decompress :: BS.ByteString -> Either String BS.ByteString
decompress input = do
    (tokens, originalLength) <- deserialiseTokens input
    decodeWithLength originalLength tokens

validateParameters :: Int -> Int -> Int -> Either String ()
validateParameters windowSize maxMatch minMatch
    | windowSize <= 0 = Left "window size must be positive"
    | windowSize > fromIntegral (maxBound :: Word16) =
        Left "window size exceeds the uint16 offset field"
    | maxMatch <= 0 = Left "maximum match length must be positive"
    | maxMatch > fromIntegral (maxBound :: Word8) =
        Left "maximum match length exceeds the uint8 length field"
    | minMatch <= 0 = Left "minimum match length must be positive"
    | minMatch > maxMatch = Left "minimum match length exceeds maximum match length"
    | otherwise = Right ()

validateToken :: Token -> Either String ()
validateToken (Literal _) = Right ()
validateToken (Match offset lengthValue)
    | offset == 0 = Left "LZSS backreferences must use a positive offset"
    | lengthValue == 0 = Left "LZSS backreferences must use a positive length"
    | otherwise = Right ()

intToWord32 :: String -> Int -> Either String Word32
intToWord32 label value
    | value < 0 = Left (label ++ " must not be negative")
    | otherwise = integerToWord32 label (toInteger value)

integerToWord32 :: String -> Integer -> Either String Word32
integerToWord32 label value
    | value > toInteger (maxBound :: Word32) =
        Left (label ++ " exceeds the uint32 field")
    | otherwise = Right (fromInteger value)

word32ToInt :: String -> Word32 -> Either String Int
word32ToInt label value
    | toInteger value > toInteger (maxBound :: Int) =
        Left (label ++ " exceeds this platform's Int range")
    | otherwise = Right (fromIntegral value)

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

traverse_ :: (value -> Either String ()) -> [value] -> Either String ()
traverse_ validator = foldM (\() value -> validator value) ()
