-- | CMP00 LZ77 sliding-window compression for strict byte strings.
module LZ77
    ( Token (..)
    , defaultWindowSize
    , defaultMaxMatch
    , defaultMinMatch
    , encode
    , encodeWith
    , decode
    , decodeWithInitialBuffer
    , serialiseTokens
    , deserialiseTokens
    , compress
    , compressWith
    , decompress
    ) where

import Control.Monad (foldM)
import Data.Bits ((.|.), shiftL, shiftR)
import qualified Data.ByteString as BS
import Data.Foldable (toList)
import Data.Sequence (Seq, (|>))
import qualified Data.Sequence as Seq
import Data.Word (Word8, Word16, Word32)

-- | One LZ77 phrase. A zero length denotes a literal and requires offset 0.
data Token = Token
    { tokenOffset :: Word16
    , tokenLength :: Word8
    , tokenNextChar :: Word8
    }
    deriving (Eq, Show)

defaultWindowSize :: Int
defaultWindowSize = 4096

defaultMaxMatch :: Int
defaultMaxMatch = 255

defaultMinMatch :: Int
defaultMinMatch = 3

-- | Encode with the CMP00 default window and match thresholds.
encode :: BS.ByteString -> [Token]
encode input = encodeUnchecked defaultWindowSize defaultMaxMatch defaultMinMatch input

-- | Encode with explicit parameters. Values must fit the fixed-width wire format.
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
            Token
                (fromIntegral bestOffset)
                (fromIntegral bestLength)
                (BS.index input (cursor + bestLength))
                : go (cursor + bestLength + 1)
        | otherwise =
            Token 0 0 (BS.index input cursor) : go (cursor + 1)
      where
        (bestOffset, bestLength) =
            findLongestMatch input cursor windowSize maxMatch

findLongestMatch :: BS.ByteString -> Int -> Int -> Int -> (Int, Int)
findLongestMatch input cursor windowSize maxMatch =
    foldl chooseBetter (0, 0) [searchStart .. cursor - 1]
  where
    searchStart = max 0 (cursor - windowSize)
    lookaheadEnd = min (cursor + maxMatch) (BS.length input - 1)

    chooseBetter best@(_, bestLength) position
        | candidateLength > bestLength = (cursor - position, candidateLength)
        | otherwise = best
      where
        candidateLength = matchLength position 0

    matchLength position matched
        | cursor + matched >= lookaheadEnd = matched
        | BS.index input (position + matched) /= BS.index input (cursor + matched) = matched
        | otherwise = matchLength position (matched + 1)

-- | Decode tokens with an empty initial search buffer.
decode :: [Token] -> Either String BS.ByteString
decode = decodeWithInitialBuffer BS.empty

-- | Decode tokens after seeding the search buffer with existing bytes.
-- The returned bytes include the initial buffer, matching CMP00 streaming semantics.
decodeWithInitialBuffer :: BS.ByteString -> [Token] -> Either String BS.ByteString
decodeWithInitialBuffer initialBuffer tokens = do
    output <- foldM decodeToken (Seq.fromList (BS.unpack initialBuffer)) tokens
    Right (BS.pack (toList output))

decodeToken :: Seq Word8 -> Token -> Either String (Seq Word8)
decodeToken output token = do
    validateToken token
    let offset = fromIntegral (tokenOffset token)
        phraseLength = fromIntegral (tokenLength token)
        outputLength = Seq.length output
    if phraseLength > 0 && offset > outputLength
        then
            Left
                ( "LZ77 offset "
                    ++ show offset
                    ++ " exceeds decoded prefix length "
                    ++ show outputLength
                )
        else do
            copied <- copyMatch (outputLength - offset) phraseLength 0 output
            Right (copied |> tokenNextChar token)

copyMatch :: Int -> Int -> Int -> Seq Word8 -> Either String (Seq Word8)
copyMatch _ phraseLength copied output
    | copied >= phraseLength = Right output
copyMatch start phraseLength copied output =
    let sourceIndex = start + copied
     in if sourceIndex < 0 || sourceIndex >= Seq.length output
            then Left "LZ77 backreference points outside the decoded prefix"
            else
                copyMatch
                    start
                    phraseLength
                    (copied + 1)
                    (output |> Seq.index output sourceIndex)

-- | Serialise tokens as a big-endian uint32 count followed by four bytes per token.
serialiseTokens :: [Token] -> Either String BS.ByteString
serialiseTokens tokens
    | toInteger (length tokens) > toInteger (maxBound :: Word32) =
        Left "too many LZ77 tokens for the uint32 count field"
    | otherwise = do
        traverse_ validateToken tokens
        Right
            ( encodeWord32 (fromIntegral (length tokens))
                <> BS.pack (concatMap encodeToken tokens)
            )
  where
    encodeToken token =
        [ fromIntegral (tokenOffset token `shiftR` 8)
        , fromIntegral (tokenOffset token)
        , tokenLength token
        , tokenNextChar token
        ]

-- | Parse the fixed-width CMP00 teaching format.
deserialiseTokens :: BS.ByteString -> Either String [Token]
deserialiseTokens input
    | BS.null input = Right []
    | BS.length input < 4 = Left "truncated LZ77 token-count header"
    | otherwise = do
        tokenCount <- decodeWord32 input 0
        let requiredLength = 4 + 4 * toInteger tokenCount
        if toInteger (BS.length input) < requiredLength
            then Left "truncated LZ77 token stream"
            else traverse parseToken [0 .. fromIntegral tokenCount - 1]
  where
    parseToken index = do
        let base = 4 + 4 * index
            offset =
                (fromIntegral (BS.index input base) `shiftL` 8)
                    .|. fromIntegral (BS.index input (base + 1))
            token =
                Token
                    offset
                    (BS.index input (base + 2))
                    (BS.index input (base + 3))
        validateToken token
        Right token

-- | Compress with the CMP00 defaults.
compress :: BS.ByteString -> Either String BS.ByteString
compress = serialiseTokens . encode

-- | Compress with an explicit window size, maximum match, and minimum match.
compressWith :: Int -> Int -> Int -> BS.ByteString -> Either String BS.ByteString
compressWith windowSize maxMatch minMatch input =
    encodeWith windowSize maxMatch minMatch input >>= serialiseTokens

-- | Decompress one fixed-width CMP00 payload.
decompress :: BS.ByteString -> Either String BS.ByteString
decompress input = deserialiseTokens input >>= decode

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
validateToken token
    | tokenLength token == 0 && tokenOffset token /= 0 =
        Left "literal LZ77 tokens must use offset 0"
    | tokenLength token > 0 && tokenOffset token == 0 =
        Left "LZ77 backreferences must use a positive offset"
    | otherwise = Right ()

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
