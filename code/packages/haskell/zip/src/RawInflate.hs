module RawInflate
    ( RawInflateError(..)
    , rawInflateErrorCode
    , rawInflateErrorCodes
    , RawInflateResult(..)
    , rawInflateMaxOutput
    , rawInflate
    , rawInflateCounted
    ) where

import Data.Bits ((.&.), (.|.), shiftL, shiftR, xor)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Foldable (toList)
import qualified Data.Map.Strict as Map
import Data.Sequence (Seq, (|>))
import qualified Data.Sequence as Seq
import Data.Word (Word8, Word32)

-- | Closed stable error taxonomy for portable raw-inflate callers.
data RawInflateError
    = InvalidOutputLimit
    | UnexpectedEof
    | ReservedBlockType
    | StoredLengthMismatch
    | HuffmanOversubscribed
    | IncompleteCodeLengthTree
    | IncompleteLiteralLengthTree
    | IncompleteDistanceTree
    | RepeatWithoutPrevious
    | RepeatOverrun
    | InvalidLiteralLengthSymbol
    | ReservedDistanceSymbol
    | InvalidBackReference
    | OutputLimitExceeded
    deriving (Eq)

instance Show RawInflateError where
    show = rawInflateErrorCode

-- | Stable payload-blind identifier for a raw-inflate failure.
rawInflateErrorCode :: RawInflateError -> String
rawInflateErrorCode err = case err of
    InvalidOutputLimit -> "invalid-output-limit"
    UnexpectedEof -> "unexpected-eof"
    ReservedBlockType -> "reserved-block-type"
    StoredLengthMismatch -> "stored-length-mismatch"
    HuffmanOversubscribed -> "huffman-oversubscribed"
    IncompleteCodeLengthTree -> "incomplete-code-length-tree"
    IncompleteLiteralLengthTree -> "incomplete-literal-length-tree"
    IncompleteDistanceTree -> "incomplete-distance-tree"
    RepeatWithoutPrevious -> "repeat-without-previous"
    RepeatOverrun -> "repeat-overrun"
    InvalidLiteralLengthSymbol -> "invalid-literal-length-symbol"
    ReservedDistanceSymbol -> "reserved-distance-symbol"
    InvalidBackReference -> "invalid-back-reference"
    OutputLimitExceeded -> "output-limit-exceeded"

-- | Stable identifiers in language-neutral fixture order.
rawInflateErrorCodes :: [String]
rawInflateErrorCodes = map rawInflateErrorCode
    [ InvalidOutputLimit, UnexpectedEof, ReservedBlockType
    , StoredLengthMismatch, HuffmanOversubscribed
    , IncompleteCodeLengthTree, IncompleteLiteralLengthTree
    , IncompleteDistanceTree, RepeatWithoutPrevious, RepeatOverrun
    , InvalidLiteralLengthSymbol, ReservedDistanceSymbol
    , InvalidBackReference, OutputLimitExceeded
    ]

-- | Successful raw inflation plus the exact final input byte reached.
data RawInflateResult = RawInflateResult
    { rawInflateOutput :: !ByteString
    , rawInflateBytesConsumed :: !Int
    } deriving (Show, Eq)

-- | Hard output ceiling shared by every portable ZIP implementation.
rawInflateMaxOutput :: Int
rawInflateMaxOutput = 256 * 1024 * 1024

data BitReader = BitReader
    { readerData :: !ByteString
    , readerBitPosition :: !Int
    }

newBitReader :: ByteString -> BitReader
newBitReader input = BitReader input 0

readLsb :: Int -> BitReader -> Either RawInflateError (Word32, BitReader)
readLsb count reader
    | count < 0 = Left UnexpectedEof
    | readerBitPosition reader + count > BS.length (readerData reader) * 8 =
        Left UnexpectedEof
    | otherwise = Right (value, reader { readerBitPosition = start + count })
  where
    start = readerBitPosition reader
    value = foldl appendBit 0 [0 .. count - 1]
    appendBit acc offset =
        let absolute = start + offset
            byte = BS.index (readerData reader) (absolute `div` 8)
            bit = fromIntegral ((byte `shiftR` (absolute `mod` 8)) .&. 1)
        in acc .|. (bit `shiftL` offset)

alignReader :: BitReader -> BitReader
alignReader reader = reader
    { readerBitPosition = ((readerBitPosition reader + 7) `div` 8) * 8 }

bytesConsumed :: BitReader -> Int
bytesConsumed reader = (readerBitPosition reader + 7) `div` 8

data HuffmanTable = HuffmanTable
    { huffmanCodes :: !(Map.Map (Int, Word32) Int)
    , huffmanMaximumLength :: !Int
    }

data HuffmanCompleteness
    = CodeLengthTree
    | LiteralLengthTree
    | DistanceTree

decodeHuffman :: HuffmanTable -> BitReader -> Either RawInflateError (Int, BitReader)
decodeHuffman table reader
    | huffmanMaximumLength table <= 0 = Left UnexpectedEof
    | otherwise = go 1 0 reader
  where
    go width code current
        | width > huffmanMaximumLength table = Left UnexpectedEof
        | otherwise = do
            (bit, next) <- readLsb 1 current
            let code' = (code `shiftL` 1) .|. bit
            case Map.lookup (width, code') (huffmanCodes table) of
                Just symbol -> Right (symbol, next)
                Nothing -> go (width + 1) code' next

buildHuffman :: [Int] -> HuffmanCompleteness -> Either RawInflateError HuffmanTable
buildHuffman lengths completeness = do
    if any (\width -> width < 0 || width > 15) lengths
        then Left HuffmanOversubscribed
        else Right ()
    let count 0 = 0
        count width = length (filter (== width) lengths)
        remainingSlots = tail (scanl (\left width -> left * 2 - count width) 1 [1 .. 15])
    if any (< 0) remainingSlots
        then Left HuffmanOversubscribed
        else Right ()
    let left = last remainingSlots
        symbolCount = length (filter (> 0) lengths)
        permittedDistance = symbolCount == 0 || (symbolCount == 1 && count 1 == 1)
    if left /= 0
        then case completeness of
            CodeLengthTree -> Left IncompleteCodeLengthTree
            LiteralLengthTree -> Left IncompleteLiteralLengthTree
            DistanceTree | not permittedDistance -> Left IncompleteDistanceTree
            DistanceTree -> Right ()
        else Right ()
    let canonicalStarts = tail (scanl
            (\code width -> (code + count (width - 1)) `shiftL` 1)
            0 [1 .. 15])
        starts = zip [1 .. 15] canonicalStarts
        step (codes, nextCodes, maxLength) (symbol, width)
            | width == 0 = (codes, nextCodes, maxLength)
            | otherwise =
                let code = requiredLookup width nextCodes
                    nextCodes' = replaceAssoc width (code + 1) nextCodes
                    codes' = Map.insert (width, fromIntegral code) symbol codes
                in (codes', nextCodes', max maxLength width)
        (finalCodes, _, maximumLength) = foldl step
            (Map.empty, starts, 0) (zip [0 ..] lengths)
    Right (HuffmanTable finalCodes maximumLength)

requiredLookup :: Eq a => a -> [(a, b)] -> b
requiredLookup key pairs =
    case lookup key pairs of
        Just value -> value
        Nothing -> error "internal Huffman table key missing"

replaceAssoc :: Eq a => a -> b -> [(a, b)] -> [(a, b)]
replaceAssoc key value = map replace
  where
    replace pair@(existing, _)
        | existing == key = (key, value)
        | otherwise = pair

fixedTables :: Either RawInflateError (HuffmanTable, HuffmanTable)
fixedTables = do
    literalLength <- buildHuffman
        (replicate 144 8 ++ replicate 112 9 ++ replicate 24 7 ++ replicate 8 8)
        LiteralLengthTree
    distance <- buildHuffman (replicate 32 5) DistanceTree
    Right (literalLength, distance)

readDynamicTables :: BitReader -> Either RawInflateError ((HuffmanTable, HuffmanTable), BitReader)
readDynamicTables reader0 = do
    (rawLiteralCount, reader1) <- readLsb 5 reader0
    (rawDistanceCount, reader2) <- readLsb 5 reader1
    (rawCodeLengthCount, reader3) <- readLsb 4 reader2
    let literalCount = fromIntegral rawLiteralCount + 257
        distanceCount = fromIntegral rawDistanceCount + 1
        codeLengthCount = fromIntegral rawCodeLengthCount + 4
    if literalCount > 286
        then Left InvalidLiteralLengthSymbol
        else Right ()
    (codeLengths, reader4) <- readCodeLengthAlphabet codeLengthCount reader3
    codeLengthTable <- buildHuffman codeLengths CodeLengthTree
    (lengths, reader5) <- readEncodedLengths
        (literalCount + distanceCount) codeLengthTable reader4 []
    let literalLengths = take literalCount lengths
        distanceLengths = drop literalCount lengths
    if literalLengths !! 256 == 0
        then Left IncompleteLiteralLengthTree
        else Right ()
    literalLengthTable <- buildHuffman literalLengths LiteralLengthTree
    distanceHuffman <- buildHuffman distanceLengths DistanceTree
    Right ((literalLengthTable, distanceHuffman), reader5)

readCodeLengthAlphabet :: Int -> BitReader -> Either RawInflateError ([Int], BitReader)
readCodeLengthAlphabet count reader = go 0 (replicate 19 0) reader
  where
    order = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]
    go index lengths current
        | index >= count = Right (lengths, current)
        | otherwise = do
            (value, next) <- readLsb 3 current
            let target = order !! index
                lengths' = take target lengths ++ [fromIntegral value] ++ drop (target + 1) lengths
            go (index + 1) lengths' next

readEncodedLengths
    :: Int
    -> HuffmanTable
    -> BitReader
    -> [Int]
    -> Either RawInflateError ([Int], BitReader)
readEncodedLengths total table reader lengths
    | length lengths >= total = Right (lengths, reader)
    | otherwise = do
        (symbol, reader1) <- decodeHuffman table reader
        case symbol of
            value | value >= 0 && value <= 15 ->
                readEncodedLengths total table reader1 (lengths ++ [value])
            16 -> case reverse lengths of
                [] -> Left RepeatWithoutPrevious
                previous : _ -> do
                    (extra, reader2) <- readLsb 2 reader1
                    appendRepeat previous (fromIntegral extra + 3) reader2
            17 -> do
                (extra, reader2) <- readLsb 3 reader1
                appendRepeat 0 (fromIntegral extra + 3) reader2
            18 -> do
                (extra, reader2) <- readLsb 7 reader1
                appendRepeat 0 (fromIntegral extra + 11) reader2
            _ -> Left UnexpectedEof
  where
    appendRepeat value count reader'
        | length lengths + count > total = Left RepeatOverrun
        | otherwise = readEncodedLengths total table reader'
            (lengths ++ replicate count value)

lengthTable :: [(Int, Int)]
lengthTable =
    [ (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0)
    , (11, 1), (13, 1), (15, 1), (17, 1)
    , (19, 2), (23, 2), (27, 2), (31, 2)
    , (35, 3), (43, 3), (51, 3), (59, 3)
    , (67, 4), (83, 4), (99, 4), (115, 4)
    , (131, 5), (163, 5), (195, 5), (227, 5), (258, 0)
    ]

distanceTable :: [(Int, Int)]
distanceTable =
    [ (1, 0), (2, 0), (3, 0), (4, 0)
    , (5, 1), (7, 1), (9, 2), (13, 2)
    , (17, 3), (25, 3), (33, 4), (49, 4)
    , (65, 5), (97, 5), (129, 6), (193, 6)
    , (257, 7), (385, 7), (513, 8), (769, 8)
    , (1025, 9), (1537, 9), (2049, 10), (3073, 10)
    , (4097, 11), (6145, 11), (8193, 12), (12289, 12)
    , (16385, 13), (24577, 13)
    ]

ensureOutputCapacity :: Int -> Seq Word8 -> Int -> Either RawInflateError ()
ensureOutputCapacity additional output outputLimit
    | additional <= outputLimit - Seq.length output = Right ()
    | otherwise = Left OutputLimitExceeded

decodeCompressedBlock
    :: BitReader
    -> Seq Word8
    -> HuffmanTable
    -> HuffmanTable
    -> Int
    -> Either RawInflateError (BitReader, Seq Word8)
decodeCompressedBlock reader output literalLength distance outputLimit = do
    (symbol, reader1) <- decodeHuffman literalLength reader
    case symbol of
        value | value >= 0 && value <= 255 -> do
            ensureOutputCapacity 1 output outputLimit
            decodeCompressedBlock reader1 (output |> fromIntegral value)
                literalLength distance outputLimit
        256 -> Right (reader1, output)
        value | value >= 257 && value <= 285 -> do
            let (baseLength, extraLengthBits) = lengthTable !! (value - 257)
            (extraLength, reader2) <- readLsb extraLengthBits reader1
            let matchLength = baseLength + fromIntegral extraLength
            (distanceSymbol, reader3) <- decodeHuffman distance reader2
            if distanceSymbol >= 30
                then Left ReservedDistanceSymbol
                else Right ()
            let (baseDistance, extraDistanceBits) = distanceTable !! distanceSymbol
            (extraDistance, reader4) <- readLsb extraDistanceBits reader3
            let backwardDistance = baseDistance + fromIntegral extraDistance
            if backwardDistance <= 0 || backwardDistance > Seq.length output
                then Left InvalidBackReference
                else Right ()
            ensureOutputCapacity matchLength output outputLimit
            decodeCompressedBlock reader4
                (copyBackReference output backwardDistance matchLength)
                literalLength distance outputLimit
        _ -> Left InvalidLiteralLengthSymbol

copyBackReference :: Seq Word8 -> Int -> Int -> Seq Word8
copyBackReference initial distance count = go initial count
  where
    go output remaining
        | remaining <= 0 = output
        | otherwise =
            let byte = Seq.index output (Seq.length output - distance)
            in go (output |> byte) (remaining - 1)

copyStored
    :: Int
    -> BitReader
    -> Seq Word8
    -> Int
    -> Either RawInflateError (BitReader, Seq Word8)
copyStored remaining reader output outputLimit
    | remaining <= 0 = Right (reader, output)
    | otherwise = do
        ensureOutputCapacity 1 output outputLimit
        (byte, reader') <- readLsb 8 reader
        copyStored (remaining - 1) reader' (output |> fromIntegral byte) outputLimit

-- | Inflate a raw RFC 1951 stream and report exact final-byte consumption.
rawInflateCounted :: ByteString -> Int -> Either RawInflateError RawInflateResult
rawInflateCounted input outputLimit
    | outputLimit < 0 || outputLimit > rawInflateMaxOutput = Left InvalidOutputLimit
    | otherwise = go (newBitReader input) Seq.empty
  where
    go reader output = do
        (finalBlock, reader1) <- readLsb 1 reader
        (blockType, reader2) <- readLsb 2 reader1
        (reader3, output') <- case blockType of
            0 -> do
                let aligned = alignReader reader2
                (storedLength, reader4) <- readLsb 16 aligned
                (storedComplement, reader5) <- readLsb 16 reader4
                if storedLength /= (storedComplement `xor` 0xffff)
                    then Left StoredLengthMismatch
                    else Right ()
                ensureOutputCapacity (fromIntegral storedLength) output outputLimit
                copyStored (fromIntegral storedLength) reader5 output outputLimit
            1 -> do
                (literalLength, distance) <- fixedTables
                decodeCompressedBlock reader2 output literalLength distance outputLimit
            2 -> do
                ((literalLength, distance), reader4) <- readDynamicTables reader2
                decodeCompressedBlock reader4 output literalLength distance outputLimit
            _ -> Left ReservedBlockType
        if finalBlock == 1
            then Right (RawInflateResult (BS.pack (toList output')) (bytesConsumed reader3))
            else go reader3 output'

-- | Inflate a raw RFC 1951 stream with a validated caller-lowerable ceiling.
rawInflate :: ByteString -> Int -> Either RawInflateError ByteString
rawInflate input outputLimit = rawInflateOutput <$> rawInflateCounted input outputLimit
