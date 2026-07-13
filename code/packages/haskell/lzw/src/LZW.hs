-- | CMP03 LZW compression for strict byte strings.
module LZW
    ( clearCode
    , stopCode
    , initialNextCode
    , initialCodeSize
    , maxCodeSize
    , encode
    , decode
    , packCodes
    , unpackCodes
    , compress
    , decompress
    ) where

import Data.Bits ((.&.), (.|.), shiftL, shiftR)
import qualified Data.ByteString as BS
import qualified Data.IntMap.Strict as IntMap
import qualified Data.Map.Strict as Map
import Data.Word (Word8, Word16, Word32, Word64)

clearCode :: Word16
clearCode = 256

stopCode :: Word16
stopCode = 257

initialNextCode :: Int
initialNextCode = 258

initialCodeSize :: Int
initialCodeSize = 9

maxCodeSize :: Int
maxCodeSize = 16

maximumEntries :: Int
maximumEntries = 1 `shiftL` maxCodeSize

initialEncodeDictionary :: Map.Map BS.ByteString Int
initialEncodeDictionary =
    Map.fromList [(BS.singleton byte, fromIntegral byte) | byte <- [0 .. 255]]

initialDecodeDictionary :: IntMap.IntMap BS.ByteString
initialDecodeDictionary =
    IntMap.fromList
        ( [(fromIntegral byte, BS.singleton byte) | byte <- [0 .. 255]]
            ++ [(fromIntegral clearCode, BS.empty), (fromIntegral stopCode, BS.empty)]
        )

-- | Encode bytes as logical LZW codes, including CLEAR and STOP.
encode :: BS.ByteString -> [Word16]
encode input = clearCode : encodeBytes initialEncodeDictionary initialNextCode BS.empty (BS.unpack input)

encodeBytes :: Map.Map BS.ByteString Int -> Int -> BS.ByteString -> [Word8] -> [Word16]
encodeBytes dictionary _ prefix []
    | BS.null prefix = [stopCode]
    | otherwise = [lookupEncodeCode dictionary prefix, stopCode]
encodeBytes dictionary nextCode prefix (byte : rest)
    | Map.member extended dictionary = encodeBytes dictionary nextCode extended rest
    | otherwise =
        let emitted = lookupEncodeCode dictionary prefix
            restarted = BS.singleton byte
         in if nextCode < maximumEntries
                then
                    emitted
                        : encodeBytes
                            (Map.insert extended nextCode dictionary)
                            (nextCode + 1)
                            restarted
                            rest
                else
                    emitted
                        : clearCode
                        : encodeBytes initialEncodeDictionary initialNextCode restarted rest
  where
    extended = BS.snoc prefix byte

lookupEncodeCode :: Map.Map BS.ByteString Int -> BS.ByteString -> Word16
lookupEncodeCode dictionary prefix =
    case Map.lookup prefix dictionary of
        Just code -> fromIntegral code
        Nothing -> error "internal LZW encoder dictionary mismatch"

-- | Decode a logical code stream. The first code must be CLEAR and the stream
-- must contain STOP.
decode :: [Word16] -> Either String BS.ByteString
decode [] = Left "empty LZW code stream"
decode (first : rest)
    | first /= clearCode =
        Left
            ( "expected CLEAR code 256 at start, got "
                ++ show first
            )
    | otherwise = decodeCodes initialDecodeDictionary initialNextCode Nothing [] rest

decodeCodes :: IntMap.IntMap BS.ByteString -> Int -> Maybe Int -> [BS.ByteString] -> [Word16] -> Either String BS.ByteString
decodeCodes _ _ _ _ [] = Left "LZW code stream is missing STOP"
decodeCodes dictionary nextCode previous chunks (codeWord : rest)
    | codeWord == clearCode =
        decodeCodes initialDecodeDictionary initialNextCode Nothing chunks rest
    | codeWord == stopCode = Right (BS.concat (reverse chunks))
    | otherwise = do
        entry <- resolveEntry dictionary nextCode previous code
        (nextDictionary, nextDynamicCode) <- addDecodeEntry dictionary nextCode previous entry
        decodeCodes nextDictionary nextDynamicCode (Just code) (entry : chunks) rest
  where
    code = fromIntegral codeWord

resolveEntry :: IntMap.IntMap BS.ByteString -> Int -> Maybe Int -> Int -> Either String BS.ByteString
resolveEntry dictionary nextCode previous code =
    case IntMap.lookup code dictionary of
        Just entry
            | not (BS.null entry) -> Right entry
        _
            | code == nextCode -> do
                previousCode <- maybe (Left "tricky LZW token has no previous code") Right previous
                previousEntry <- lookupDecodeEntry dictionary previousCode
                firstByte <- maybe (Left "tricky LZW token has an empty previous entry") Right (fst <$> BS.uncons previousEntry)
                Right (BS.snoc previousEntry firstByte)
            | otherwise ->
                Left
                    ( "invalid LZW code "
                        ++ show code
                        ++ ": next dynamic code is "
                        ++ show nextCode
                    )

addDecodeEntry :: IntMap.IntMap BS.ByteString -> Int -> Maybe Int -> BS.ByteString -> Either String (IntMap.IntMap BS.ByteString, Int)
addDecodeEntry dictionary nextCode Nothing _ = Right (dictionary, nextCode)
addDecodeEntry dictionary nextCode (Just previousCode) entry
    | nextCode >= maximumEntries = Right (dictionary, nextCode)
    | otherwise = do
        previousEntry <- lookupDecodeEntry dictionary previousCode
        firstByte <- maybe (Left "decoded LZW entry is empty") Right (fst <$> BS.uncons entry)
        Right (IntMap.insert nextCode (BS.snoc previousEntry firstByte) dictionary, nextCode + 1)

lookupDecodeEntry :: IntMap.IntMap BS.ByteString -> Int -> Either String BS.ByteString
lookupDecodeEntry dictionary code =
    case IntMap.lookup code dictionary of
        Just entry
            | not (BS.null entry) -> Right entry
        _ -> Left ("invalid previous LZW code " ++ show code)

data BitWriter = BitWriter
    { writerBuffer :: Word64
    , writerBitCount :: Int
    , writerBytesReversed :: [Word8]
    }

emptyWriter :: BitWriter
emptyWriter = BitWriter 0 0 []

writeCode :: Word16 -> Int -> BitWriter -> BitWriter
writeCode code width writer = drainWriter filled
  where
    filled =
        writer
            { writerBuffer = writerBuffer writer .|. (fromIntegral code `shiftL` writerBitCount writer)
            , writerBitCount = writerBitCount writer + width
            }

drainWriter :: BitWriter -> BitWriter
drainWriter writer
    | writerBitCount writer < 8 = writer
    | otherwise =
        drainWriter
            writer
                { writerBuffer = writerBuffer writer `shiftR` 8
                , writerBitCount = writerBitCount writer - 8
                , writerBytesReversed = fromIntegral (writerBuffer writer .&. 0xff) : writerBytesReversed writer
                }

finishWriter :: BitWriter -> BS.ByteString
finishWriter writer =
    BS.pack
        ( reverse
            ( if writerBitCount writer == 0
                then writerBytesReversed writer
                else fromIntegral (writerBuffer writer .&. 0xff) : writerBytesReversed writer
            )
        )

data BitReader = BitReader
    { readerData :: BS.ByteString
    , readerPosition :: Int
    , readerBuffer :: Word64
    , readerBitCount :: Int
    }

newReader :: BS.ByteString -> BitReader
newReader bytes = BitReader bytes 0 0 0

readCode :: Int -> BitReader -> Either String (Word16, BitReader)
readCode width reader = do
    filled <- fillReader width reader
    if readerBitCount filled < width
        then Left "truncated LZW bit stream"
        else
            let mask = (1 `shiftL` width) - 1 :: Word64
                code = fromIntegral (readerBuffer filled .&. mask)
             in Right
                    ( code
                    , filled
                        { readerBuffer = readerBuffer filled `shiftR` width
                        , readerBitCount = readerBitCount filled - width
                        }
                    )

fillReader :: Int -> BitReader -> Either String BitReader
fillReader width reader
    | readerBitCount reader >= width = Right reader
    | readerPosition reader >= BS.length (readerData reader) = Right reader
    | otherwise =
        let byte = BS.index (readerData reader) (readerPosition reader)
         in fillReader
                width
                reader
                    { readerPosition = readerPosition reader + 1
                    , readerBuffer = readerBuffer reader .|. (fromIntegral byte `shiftL` readerBitCount reader)
                    , readerBitCount = readerBitCount reader + 8
                    }

readerHasValidPadding :: BitReader -> Bool
readerHasValidPadding reader = bufferedBitsAreZero && noRemainingBytes
  where
    bufferedMask
        | readerBitCount reader == 0 = 0
        | otherwise = (1 `shiftL` readerBitCount reader) - 1
    bufferedBitsAreZero = readerBuffer reader .&. bufferedMask == 0
    noRemainingBytes = readerPosition reader == BS.length (readerData reader)

-- | Pack logical codes as a four-byte big-endian length followed by
-- variable-width LSB-first codes.
packCodes :: Int -> [Word16] -> Either String BS.ByteString
packCodes originalLength codes = do
    lengthWord <- intToWord32 originalLength
    validateOpeningCode codes
    payload <- packCodeStream initialCodeSize initialNextCode emptyWriter codes
    Right (encodeWord32 lengthWord <> payload)

validateOpeningCode :: [Word16] -> Either String ()
validateOpeningCode [] = Left "empty LZW code stream"
validateOpeningCode (first : _)
    | first == clearCode = Right ()
    | otherwise = Left ("expected CLEAR code 256 at start, got " ++ show first)

packCodeStream :: Int -> Int -> BitWriter -> [Word16] -> Either String BS.ByteString
packCodeStream _ _ _ [] = Left "LZW code stream is missing STOP"
packCodeStream width nextCode writer (code : rest)
    | fromIntegral code >= (1 `shiftL` width :: Int) =
        Left ("LZW code " ++ show code ++ " does not fit in " ++ show width ++ " bits")
    | code == stopCode =
        if null rest
            then Right (finishWriter written)
            else Left "LZW code stream contains data after STOP"
    | code == clearCode =
        packCodeStream initialCodeSize initialNextCode written rest
    | otherwise =
        let (nextWidth, followingCode) = advanceWireWidth width nextCode
         in packCodeStream nextWidth followingCode written rest
  where
    written = writeCode code width writer

advanceWireWidth :: Int -> Int -> (Int, Int)
advanceWireWidth width nextCode
    | nextCode >= maximumEntries = (width, nextCode)
    | followingCode > (1 `shiftL` width) && width < maxCodeSize = (width + 1, followingCode)
    | otherwise = (width, followingCode)
  where
    followingCode = nextCode + 1

-- | Parse one strict CMP03 payload and return its logical codes and stored
-- original length.
unpackCodes :: BS.ByteString -> Either String ([Word16], Int)
unpackCodes input
    | BS.length input < 4 = Left "input too short: missing four-byte LZW header"
    | otherwise = do
        originalLengthWord <- decodeWord32 input
        originalLength <- word32ToInt originalLengthWord
        let reader = newReader (BS.drop 4 input)
        (first, afterFirst) <- readCode initialCodeSize reader
        if first /= clearCode
            then Left ("expected CLEAR code 256 at start, got " ++ show first)
            else do
                codes <- unpackCodeStream initialCodeSize initialNextCode afterFirst [clearCode]
                Right (codes, originalLength)

unpackCodeStream :: Int -> Int -> BitReader -> [Word16] -> Either String [Word16]
unpackCodeStream width nextCode reader codesReversed = do
    (code, afterCode) <- readCode width reader
    if code == stopCode
        then
            if readerHasValidPadding afterCode
                then Right (reverse (stopCode : codesReversed))
                else Left "non-zero padding or trailing bytes follow the LZW STOP code"
        else
            if code == clearCode
                then unpackCodeStream initialCodeSize initialNextCode afterCode (code : codesReversed)
                else
                    let (nextWidth, followingCode) = advanceWireWidth width nextCode
                     in unpackCodeStream nextWidth followingCode afterCode (code : codesReversed)

-- | Compress bytes to the CMP03 wire format.
compress :: BS.ByteString -> Either String BS.ByteString
compress input = packCodes (BS.length input) (encode input)

-- | Decompress one strict CMP03 payload.
decompress :: BS.ByteString -> Either String BS.ByteString
decompress input = do
    (codes, originalLength) <- unpackCodes input
    output <- decode codes
    if BS.length output < originalLength
        then Left "decoded LZW stream is shorter than the stored original length"
        else Right (BS.take originalLength output)

intToWord32 :: Int -> Either String Word32
intToWord32 value
    | value < 0 = Left "original length must not be negative"
    | toInteger value > toInteger (maxBound :: Word32) = Left "original length exceeds the uint32 field"
    | otherwise = Right (fromIntegral value)

word32ToInt :: Word32 -> Either String Int
word32ToInt value
    | toInteger value > toInteger (maxBound :: Int) = Left "original length exceeds this platform's Int range"
    | otherwise = Right (fromIntegral value)

encodeWord32 :: Word32 -> BS.ByteString
encodeWord32 value =
    BS.pack
        [ fromIntegral (value `shiftR` 24)
        , fromIntegral (value `shiftR` 16)
        , fromIntegral (value `shiftR` 8)
        , fromIntegral value
        ]

decodeWord32 :: BS.ByteString -> Either String Word32
decodeWord32 input =
    Right
        ( byteAt 0 `shiftL` 24
            .|. byteAt 1 `shiftL` 16
            .|. byteAt 2 `shiftL` 8
            .|. byteAt 3
        )
  where
    byteAt index = fromIntegral (BS.index input index)
