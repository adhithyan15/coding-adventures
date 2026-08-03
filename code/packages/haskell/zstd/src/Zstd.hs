-- | CMP07 educational Zstandard frames with raw literals and predefined FSE tables.
module Zstd
    ( magic
    , maxBlockSize
    , maxOutputSize
    , compress
    , decompress
    ) where

import Prelude hiding (foldl')
import Control.Monad (foldM, unless, when)
import Data.Bits ((.&.), (.|.), shiftL, shiftR, testBit)
import qualified Data.ByteString as BS
import Data.Foldable (toList)
import Data.List (foldl')
import Data.Sequence (Seq, (|>))
import qualified Data.Sequence as Seq
import Data.Word (Word8, Word32, Word64)
import qualified LZSS

-- | RFC 8878 frame magic number.
magic :: Word32
magic = 0xFD2FB528

-- | Maximum decompressed payload represented by one block.
maxBlockSize :: Int
maxBlockSize = 128 * 1024

-- | Defensive limit for one decompressed frame.
maxOutputSize :: Int
maxOutputSize = 256 * 1024 * 1024

literalLengthAccuracyLog :: Int
literalLengthAccuracyLog = 6

matchLengthAccuracyLog :: Int
matchLengthAccuracyLog = 6

offsetAccuracyLog :: Int
offsetAccuracyLog = 5

data CodeRange = CodeRange
    { codeBaseline :: Int
    , codeExtraBits :: Int
    }
    deriving (Eq, Show)

data Sequence = Sequence
    { sequenceLiteralLength :: Int
    , sequenceMatchLength :: Int
    , sequenceOffset :: Int
    }
    deriving (Eq, Show)

data DecodeEntry = DecodeEntry
    { decodeEntrySymbol :: Int
    , decodeEntryBits :: Int
    , decodeEntryBaseline :: Int
    }
    deriving (Eq, Show)

data EncodeEntry = EncodeEntry
    { encodeEntryDeltaBits :: Int
    , encodeEntryDeltaFindState :: Int
    }
    deriving (Eq, Show)

literalLengthCodes :: [CodeRange]
literalLengthCodes =
    [ CodeRange 0 0, CodeRange 1 0, CodeRange 2 0, CodeRange 3 0
    , CodeRange 4 0, CodeRange 5 0, CodeRange 6 0, CodeRange 7 0
    , CodeRange 8 0, CodeRange 9 0, CodeRange 10 0, CodeRange 11 0
    , CodeRange 12 0, CodeRange 13 0, CodeRange 14 0, CodeRange 15 0
    , CodeRange 16 1, CodeRange 18 1, CodeRange 20 1, CodeRange 22 1
    , CodeRange 24 2, CodeRange 28 2, CodeRange 32 3, CodeRange 40 3
    , CodeRange 48 4, CodeRange 64 6, CodeRange 128 7, CodeRange 256 8
    , CodeRange 512 9, CodeRange 1024 10, CodeRange 2048 11, CodeRange 4096 12
    , CodeRange 8192 13, CodeRange 16384 14, CodeRange 32768 15, CodeRange 65536 16
    ]

matchLengthCodes :: [CodeRange]
matchLengthCodes =
    [ CodeRange 3 0, CodeRange 4 0, CodeRange 5 0, CodeRange 6 0
    , CodeRange 7 0, CodeRange 8 0, CodeRange 9 0, CodeRange 10 0
    , CodeRange 11 0, CodeRange 12 0, CodeRange 13 0, CodeRange 14 0
    , CodeRange 15 0, CodeRange 16 0, CodeRange 17 0, CodeRange 18 0
    , CodeRange 19 0, CodeRange 20 0, CodeRange 21 0, CodeRange 22 0
    , CodeRange 23 0, CodeRange 24 0, CodeRange 25 0, CodeRange 26 0
    , CodeRange 27 0, CodeRange 28 0, CodeRange 29 0, CodeRange 30 0
    , CodeRange 31 0, CodeRange 32 0, CodeRange 33 0, CodeRange 34 0
    , CodeRange 35 1, CodeRange 37 1, CodeRange 39 1, CodeRange 41 1
    , CodeRange 43 2, CodeRange 47 2, CodeRange 51 3, CodeRange 59 3
    , CodeRange 67 4, CodeRange 83 4, CodeRange 99 5, CodeRange 131 7
    , CodeRange 259 8, CodeRange 515 9, CodeRange 1027 10, CodeRange 2051 11
    , CodeRange 4099 12, CodeRange 8195 13, CodeRange 16387 14
    , CodeRange 32771 15, CodeRange 65539 16
    ]

literalLengthNorm :: [Int]
literalLengthNorm =
    [ 4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1
    , 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1
    , -1, -1, -1, -1
    ]

matchLengthNorm :: [Int]
matchLengthNorm =
    [ 1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1
    , 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1
    , 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1
    , -1, -1, -1, -1, -1
    ]

offsetNorm :: [Int]
offsetNorm =
    [ 1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1
    , 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1
    ]

data BitWriter = BitWriter [Word8] Integer Int

emptyBitWriter :: BitWriter
emptyBitWriter = BitWriter [] 0 0

addBits :: Int -> Int -> BitWriter -> BitWriter
addBits _ 0 writer = writer
addBits value count (BitWriter bytes register bitCount) =
    flushWriter
        ( BitWriter
            bytes
            (register .|. ((toInteger value .&. mask) `shiftL` bitCount))
            (bitCount + count)
        )
  where
    mask = (1 `shiftL` count) - 1

flushWriter :: BitWriter -> BitWriter
flushWriter writer@(BitWriter _ _ bitCount)
    | bitCount < 8 = writer
flushWriter (BitWriter bytes register bitCount) =
    flushWriter
        ( BitWriter
            (fromInteger (register .&. 0xFF) : bytes)
            (register `shiftR` 8)
            (bitCount - 8)
        )

finishWriter :: BitWriter -> BS.ByteString
finishWriter (BitWriter bytes register bitCount) =
    BS.pack (reverse (sentinelByte : bytes))
  where
    sentinelByte = fromInteger ((register .&. 0xFF) .|. (1 `shiftL` bitCount))

newtype BitReader = BitReader [Bool]

makeBitReader :: BS.ByteString -> Either String BitReader
makeBitReader input
    | BS.null input = Left "empty bitstream"
    | lastByte == 0 = Left "bitstream last byte has no sentinel"
    | otherwise =
        Right
            ( BitReader
                ( descendingBits lastByte (sentinelPosition - 1)
                    ++ concatMap (\byte -> descendingBits byte 7) (reverse previousBytes)
                )
            )
  where
    bytes = BS.unpack input
    lastByte = last bytes
    previousBytes = init bytes
    sentinelPosition = floorLog2 (fromIntegral lastByte)

descendingBits :: Word8 -> Int -> [Bool]
descendingBits _ position | position < 0 = []
descendingBits byte position = testBit byte position : descendingBits byte (position - 1)

readBits :: Int -> BitReader -> Either String (Int, BitReader)
readBits count (BitReader bits)
    | count < 0 || count > 32 = Left "invalid bit count"
    | length selected /= count = Left "bitstream is truncated"
    | otherwise = Right (foldl' appendBit 0 selected, BitReader remaining)
  where
    (selected, remaining) = splitAt count bits
    appendBit value bit = value * 2 + if bit then 1 else 0

-- | Compress bytes into a deterministic educational Zstandard frame.
compress :: BS.ByteString -> Either String BS.ByteString
compress input = do
    encodedBlocks <-
        if BS.null input
            then Right [writeBlockHeader 0 0 True]
            else traverse encodeChunk (markLast (chunksOf maxBlockSize input))
    Right
        ( BS.concat
            ( magicBytes
                : BS.singleton 0xE0
                : encodeWord64LittleEndian (fromIntegral (BS.length input))
                : encodedBlocks
            )
        )
  where
    encodeChunk (isLast, block)
        | allEqual block =
            Right
                ( writeBlockHeader (BS.length block) 1 isLast
                    <> BS.singleton (BS.head block)
                )
        | otherwise = do
            compressed <- compressBlock block
            case compressed of
                Just payload
                    | BS.length payload < BS.length block ->
                        Right (writeBlockHeader (BS.length payload) 2 isLast <> payload)
                _ -> Right (writeBlockHeader (BS.length block) 0 isLast <> block)

-- | Decompress one strict educational Zstandard frame.
decompress :: BS.ByteString -> Either String BS.ByteString
decompress input = do
    when (BS.length input < 5) (Left "frame is too short")
    unless (BS.take 4 input == magicBytes) (Left "bad Zstandard magic number")
    let descriptor = BS.index input 4
    -- RFC 8878 SS3.1.1.1 frame header descriptor bit layout:
    --   bit 2 = Content_Checksum_Flag, bit 3 = Reserved_bit (must be 0),
    --   bit 4 = Unused_bit (decoder must ignore it, NOT reserved).
    -- An earlier revision of this decoder read the checksum flag from bit 4
    -- and treated bits 2+3 as jointly reserved -- rejecting every real
    -- checksummed frame as "reserved bits set" while silently never
    -- detecting a checksum trailer on any frame. See lessons.md Lesson 95.
    when (descriptor .&. 0x08 /= 0) (Left "reserved frame-header bits are set")
    let contentSizeFlag = fromIntegral (descriptor `shiftR` 6) :: Int
        singleSegment = descriptor .&. 0x20 /= 0
        checksum = descriptor .&. 0x04 /= 0
        dictionaryFlag = fromIntegral (descriptor .&. 3) :: Int
    afterWindow <-
        if singleSegment
            then Right 5
            else requireAvailable input 5 1 "window descriptor" >> Right 6
    let dictionaryBytes = case dictionaryFlag of 0 -> 0; 1 -> 1; 2 -> 2; _ -> 4
    requireAvailable input afterWindow dictionaryBytes "dictionary id"
    let afterDictionary = afterWindow + dictionaryBytes
        contentSizeBytes =
            case contentSizeFlag of
                0 -> if singleSegment then 1 else 0
                1 -> 2
                2 -> 4
                _ -> 8
    requireAvailable input afterDictionary contentSizeBytes "frame content size"
    (afterBlocks, output) <- decodeBlocks (afterDictionary + contentSizeBytes) Seq.empty
    let afterChecksum = afterBlocks + if checksum then 4 else 0
    when checksum (requireAvailable input afterBlocks 4 "content checksum")
    unless (afterChecksum == BS.length input) (Left "trailing bytes after Zstandard frame")
    Right (BS.pack (toList output))
  where
    decodeBlocks position output = do
        requireAvailable input position 3 "block header"
        let header =
                byteAt input position
                    .|. (byteAt input (position + 1) `shiftL` 8)
                    .|. (byteAt input (position + 2) `shiftL` 16)
            isLast = header .&. 1 /= 0
            blockType = (header `shiftR` 1) .&. 3
            blockSize = header `shiftR` 3
            payloadPosition = position + 3
        when (blockSize > maxBlockSize) (Left "block exceeds the 128 KiB limit")
        (nextPosition, nextOutput) <-
            case blockType of
                0 -> do
                    requireAvailable input payloadPosition blockSize "raw block"
                    ensureOutputLimit (Seq.length output) blockSize
                    let payload = BS.take blockSize (BS.drop payloadPosition input)
                    Right (payloadPosition + blockSize, appendBytes output payload)
                1 -> do
                    requireAvailable input payloadPosition 1 "RLE block"
                    ensureOutputLimit (Seq.length output) blockSize
                    let value = BS.index input payloadPosition
                    Right
                        ( payloadPosition + 1
                        , foldl' (\current _ -> current |> value) output [1 .. blockSize]
                        )
                2 -> do
                    requireAvailable input payloadPosition blockSize "compressed block"
                    let payload = BS.take blockSize (BS.drop payloadPosition input)
                    decoded <- decompressBlock payload output
                    Right (payloadPosition + blockSize, decoded)
                _ -> Left "reserved block type 3"
        if isLast
            then Right (nextPosition, nextOutput)
            else decodeBlocks nextPosition nextOutput

compressBlock :: BS.ByteString -> Either String (Maybe BS.ByteString)
compressBlock block = do
    tokens <-
        LZSS.encodeWith
            LZSS.defaultWindowSize
            LZSS.defaultMaxMatch
            LZSS.defaultMinMatch
            block
    let (literals, sequences) = collectTokens tokens
    if null sequences
        then Right Nothing
        else do
            sequenceBytes <- encodeSequences sequences
            Right
                ( Just
                    ( encodeLiterals literals
                        <> encodeSequenceCount (length sequences)
                        <> BS.singleton 0
                        <> sequenceBytes
                    )
                )

collectTokens :: [LZSS.Token] -> ([Word8], [Sequence])
collectTokens tokens = (reverse literals, reverse sequences)
  where
    (literals, sequences, _) = foldl' step ([], [], 0) tokens
    step (literalBytes, encoded, literalRun) token =
        case token of
            LZSS.Literal byte -> (byte : literalBytes, encoded, literalRun + 1)
            LZSS.Match offset lengthValue ->
                ( literalBytes
                , Sequence literalRun (fromIntegral lengthValue) (fromIntegral offset) : encoded
                , 0
                )

decompressBlock :: BS.ByteString -> Seq Word8 -> Either String (Seq Word8)
decompressBlock input initialOutput = do
    (literals, literalBytes) <- decodeLiterals input
    if literalBytes >= BS.length input
        then appendRemainingLiterals literals 0 initialOutput
        else do
            (sequenceCount, countBytes) <- decodeSequenceCount (BS.drop literalBytes input)
            let modesPosition = literalBytes + countBytes
            if sequenceCount == 0
                then appendRemainingLiterals literals 0 initialOutput
                else do
                    requireAvailable input modesPosition 1 "symbol compression modes"
                    let modes = BS.index input modesPosition
                    when (modes .&. 0xFC /= 0) (Left "only predefined FSE modes are supported")
                    reader0 <- makeBitReader (BS.drop (modesPosition + 1) input)
                    literalTable <- buildDecodeTable literalLengthNorm literalLengthAccuracyLog
                    matchTable <- buildDecodeTable matchLengthNorm matchLengthAccuracyLog
                    offsetTable <- buildDecodeTable offsetNorm offsetAccuracyLog
                    -- RFC 8878 SS3.1.1.3.2.1.2: the initial FSE states are read
                    -- in order LL, OF, ML -- deliberately NOT the same order
                    -- as the per-sequence symbol decode below (LL, ML, OF).
                    -- Verified against the real `zstd` CLI (Lesson 95); an
                    -- earlier revision of this decoder read LL, ML, OF here,
                    -- which is wrong only for this one-time initial read.
                    (literalState, reader1) <- readBits literalLengthAccuracyLog reader0
                    (offsetState, reader2) <- readBits offsetAccuracyLog reader1
                    (matchState, reader3) <- readBits matchLengthAccuracyLog reader2
                    (output, literalPosition, _) <-
                        decodeSequences
                            sequenceCount
                            literals
                            literalTable
                            matchTable
                            offsetTable
                            literalState
                            matchState
                            offsetState
                            initialOutput
                            0
                            reader3
                    appendRemainingLiterals literals literalPosition output

decodeSequences ::
    Int ->
    BS.ByteString ->
    [DecodeEntry] ->
    [DecodeEntry] ->
    [DecodeEntry] ->
    Int ->
    Int ->
    Int ->
    Seq Word8 ->
    Int ->
    BitReader ->
    Either String (Seq Word8, Int, BitReader)
decodeSequences 0 _ _ _ _ _ _ _ output literalPosition reader =
    Right (output, literalPosition, reader)
decodeSequences remaining literals literalTable matchTable offsetTable literalState matchState offsetState output literalPosition reader0 = do
    -- Step 1 -- PEEK all three symbols from the current states. This is a
    -- bare table lookup (table ! state) and consumes NO bits: the FSE
    -- state itself already IS the decode-table index. Only the state
    -- UPDATE (step 3 below) reads bits. An earlier revision of this
    -- decoder fused peek-and-update into one step (via `decodeSymbol`) and
    -- performed it eagerly for LL, OF, ML in that order -- BEFORE reading
    -- any extra bits -- which reads the wrong bits at the wrong bitstream
    -- position. See lessons.md Lesson 95.
    literalEntry <- indexEither "FSE decoder state" literalTable literalState
    matchEntry <- indexEither "FSE decoder state" matchTable matchState
    offsetEntry <- indexEither "FSE decoder state" offsetTable offsetState
    let literalCode = decodeEntrySymbol literalEntry
        matchCode = decodeEntrySymbol matchEntry
        offsetCode = decodeEntrySymbol offsetEntry
    literalRange <- lookupCodeRange "literal length" literalCode literalLengthCodes
    matchRange <- lookupCodeRange "match length" matchCode matchLengthCodes
    -- Step 2 -- read the VALUE extra bits, order OF, ML, LL (RFC 8878
    -- SS3.1.1.3.2.1.2: "Decoding starts by reading the Number_of_Bits
    -- required to decode offset. It does the same for Match_Length and
    -- then for Literals_Length.").
    (offsetExtra, reader1) <- readBits offsetCode reader0
    (matchExtra, reader2) <- readBits (codeExtraBits matchRange) reader1
    (literalExtra, reader3) <- readBits (codeExtraBits literalRange) reader2
    let literalLength = codeBaseline literalRange + literalExtra
        matchLength = codeBaseline matchRange + matchExtra
        rawOffset = (1 `shiftL` offsetCode) .|. offsetExtra
        matchOffset = rawOffset - 3
    when
        (literalLength < 0 || literalPosition > BS.length literals - literalLength)
        (Left "literal run exceeds the literals section")
    ensureOutputLimit (Seq.length output) literalLength
    let literalSlice = BS.take literalLength (BS.drop literalPosition literals)
        afterLiterals = appendBytes output literalSlice
    when
        (matchOffset < 1 || matchOffset > Seq.length afterLiterals)
        (Left "match offset exceeds decoded output")
    ensureOutputLimit (Seq.length afterLiterals) matchLength
    afterMatch <- copyMatch matchOffset matchLength afterLiterals
    -- Step 3 -- update FSE states (consumes bits), order LL, ML, OF,
    -- preparing the states the NEXT sequence's peek (step 1) will use.
    -- Per the reference decoder (ZSTD_decodeSequence), this update is
    -- skipped ENTIRELY for the last sequence in the block: there is no
    -- "next" sequence to prepare a state for, and (symmetrically) the
    -- encoder never flushed any bits for that non-existent transition --
    -- see the `encodeInitState`/`encodeSymbol` split in `encodeOne` below.
    -- Performing this read unconditionally, as an earlier revision of this
    -- decoder did, consumes bits that were never written, corrupting the
    -- position of every read that follows.
    (nextLiteralState, nextMatchState, nextOffsetState, reader6) <-
        if remaining == 1
            then Right (literalState, matchState, offsetState, reader3)
            else do
                (literalValue, reader4) <- readBits (decodeEntryBits literalEntry) reader3
                (matchValue, reader5) <- readBits (decodeEntryBits matchEntry) reader4
                (offsetValue, reader6') <- readBits (decodeEntryBits offsetEntry) reader5
                Right
                    ( decodeEntryBaseline literalEntry + literalValue
                    , decodeEntryBaseline matchEntry + matchValue
                    , decodeEntryBaseline offsetEntry + offsetValue
                    , reader6'
                    )
    decodeSequences
        (remaining - 1)
        literals
        literalTable
        matchTable
        offsetTable
        nextLiteralState
        nextMatchState
        nextOffsetState
        afterMatch
        (literalPosition + literalLength)
        reader6

appendRemainingLiterals :: BS.ByteString -> Int -> Seq Word8 -> Either String (Seq Word8)
appendRemainingLiterals literals literalPosition output = do
    let remaining = BS.length literals - literalPosition
    when (remaining < 0) (Left "literal position exceeds the literals section")
    ensureOutputLimit (Seq.length output) remaining
    Right (appendBytes output (BS.drop literalPosition literals))

copyMatch :: Int -> Int -> Seq Word8 -> Either String (Seq Word8)
copyMatch offset lengthValue output = go 0 output
  where
    start = Seq.length output - offset
    go copied current
        | copied >= lengthValue = Right current
        | start + copied < 0 || start + copied >= Seq.length current =
            Left "match offset exceeds decoded output"
        | otherwise = go (copied + 1) (current |> Seq.index current (start + copied))

encodeLiterals :: [Word8] -> BS.ByteString
encodeLiterals literals = header <> BS.pack literals
  where
    count = length literals
    header
        | count <= 31 = BS.singleton (fromIntegral (count `shiftL` 3))
        | count <= 4095 =
            let value = (count `shiftL` 4) .|. 4
             in BS.pack [fromIntegral value, fromIntegral (value `shiftR` 8)]
        | otherwise =
            let value = (count `shiftL` 4) .|. 12
             in BS.pack
                    [ fromIntegral value
                    , fromIntegral (value `shiftR` 8)
                    , fromIntegral (value `shiftR` 16)
                    ]

decodeLiterals :: BS.ByteString -> Either String (BS.ByteString, Int)
decodeLiterals input = do
    requireAvailable input 0 1 "literals section"
    let first = BS.index input 0
    when (first .&. 3 /= 0) (Left "only raw literals are supported")
    let sizeFormat = fromIntegral ((first `shiftR` 2) .&. 3) :: Int
    (count, headerBytes) <-
        case sizeFormat of
            0 -> Right (fromIntegral (first `shiftR` 3), 1)
            2 -> Right (fromIntegral (first `shiftR` 3), 1)
            1 -> do
                requireAvailable input 0 2 "literals header"
                Right
                    ( fromIntegral (first `shiftR` 4)
                        .|. (byteAt input 1 `shiftL` 4)
                    , 2
                    )
            _ -> do
                requireAvailable input 0 3 "literals header"
                Right
                    ( fromIntegral (first `shiftR` 4)
                        .|. (byteAt input 1 `shiftL` 4)
                        .|. (byteAt input 2 `shiftL` 12)
                    , 3
                    )
    requireAvailable input headerBytes count "literals data"
    Right (BS.take count (BS.drop headerBytes input), headerBytes + count)

encodeSequenceCount :: Int -> BS.ByteString
encodeSequenceCount count
    | count < 128 = BS.singleton (fromIntegral count)
    | count < 0x7F00 =
        BS.pack
            [ fromIntegral (0x80 .|. (count `shiftR` 8))
            , fromIntegral count
            ]
    | otherwise =
        let remainder = count - 0x7F00
         in BS.pack [0xFF, fromIntegral remainder, fromIntegral (remainder `shiftR` 8)]

decodeSequenceCount :: BS.ByteString -> Either String (Int, Int)
decodeSequenceCount input = do
    requireAvailable input 0 1 "sequence count"
    let first = byteAt input 0
    if first < 128
        then Right (first, 1)
        else
            if first < 0xFF
                then do
                    requireAvailable input 0 2 "sequence count"
                    Right (((first .&. 0x7F) `shiftL` 8) .|. byteAt input 1, 2)
                else do
                    requireAvailable input 0 3 "sequence count"
                    Right (0x7F00 + byteAt input 1 + (byteAt input 2 `shiftL` 8), 3)

encodeSequences :: [Sequence] -> Either String BS.ByteString
encodeSequences sequences = do
    (literalEntries, literalStates) <- buildEncodeTables literalLengthNorm literalLengthAccuracyLog
    (matchEntries, matchStates) <- buildEncodeTables matchLengthNorm matchLengthAccuracyLog
    (offsetEntries, offsetStates) <- buildEncodeTables offsetNorm offsetAccuracyLog
    let literalSize = 1 `shiftL` literalLengthAccuracyLog
        matchSize = 1 `shiftL` matchLengthAccuracyLog
        offsetSize = 1 `shiftL` offsetAccuracyLog
        -- Encode sequences in reverse order (last real sequence first); tag
        -- each with its position in that reversed walk so `encodeOne` can
        -- tell whether it is processing index 0 -- the semantically LAST
        -- real sequence, which needs direct state initialisation instead of
        -- a normal transition (see `encodeOne`).
        reversedSequences = zip [0 :: Int ..] (reverse sequences)
    (writer, literalState, matchState, offsetState) <-
        foldM
            (encodeOne literalEntries literalStates matchEntries matchStates offsetEntries offsetStates)
            (emptyBitWriter, literalSize, matchSize, offsetSize)
            reversedSequences
    -- RFC 8878 SS3.1.1.3.2.1.2: a forward decoder reads the initial FSE
    -- states in order LL, OF, ML (see decompressBlock) -- a DIFFERENT order
    -- from the per-sequence update order below. Because this bitstream is
    -- written backward (the LAST bits written here are the FIRST bits a
    -- forward reader consumes), we write them in the reverse of that read
    -- order: ML, then OF, then LL.
    let finished =
            addBits (literalState - literalSize) literalLengthAccuracyLog
                ( addBits (offsetState - offsetSize) offsetAccuracyLog
                    (addBits (matchState - matchSize) matchLengthAccuracyLog writer)
                )
    Right (finishWriter finished)

encodeOne ::
    [EncodeEntry] ->
    [Int] ->
    [EncodeEntry] ->
    [Int] ->
    [EncodeEntry] ->
    [Int] ->
    (BitWriter, Int, Int, Int) ->
    (Int, Sequence) ->
    Either String (BitWriter, Int, Int, Int)
encodeOne literalEntries literalStates matchEntries matchStates offsetEntries offsetStates (writer0, literalState, matchState, offsetState) (index, currentSequence) = do
    let literalCode = valueToCode (sequenceLiteralLength currentSequence) literalLengthCodes
        matchCode = valueToCode (sequenceMatchLength currentSequence) matchLengthCodes
        rawOffset = sequenceOffset currentSequence + 3
        offsetCode = floorLog2 rawOffset
        offsetExtra = rawOffset - (1 `shiftL` offsetCode)
        literalRange = literalLengthCodes !! literalCode
        matchRange = matchLengthCodes !! matchCode
        literalExtraValue = sequenceLiteralLength currentSequence - codeBaseline literalRange
        matchExtraValue = sequenceMatchLength currentSequence - codeBaseline matchRange
    -- The sequence processed FIRST in this reverse loop (index 0) is the
    -- semantically LAST real sequence in the block. A forward decoder never
    -- performs a state-update read after decoding the last sequence (see
    -- decodeSequences' conditional update), so this encoder cannot produce
    -- that sequence's starting state via a normal bit-flushing transition
    -- either -- there is no corresponding decode-side bit read to consume
    -- it. It must be computed directly via `encodeInitState` (mirrors real
    -- zstd's FSE_initCState2), which writes NO bits at all. Every other
    -- sequence gets a normal transition, write order OF, ML, LL. An earlier
    -- revision of this encoder always flushed a transition uniformly,
    -- writing bits a real decoder would never read and shifting the
    -- bit-alignment of everything that followed. See lessons.md Lesson 95.
    (writer1, nextLiteralState, nextMatchState, nextOffsetState) <-
        if index == 0
            then do
                initOffsetState <- encodeInitState offsetCode offsetEntries offsetStates
                initMatchState <- encodeInitState matchCode matchEntries matchStates
                initLiteralState <- encodeInitState literalCode literalEntries literalStates
                Right (writer0, initLiteralState, initMatchState, initOffsetState)
            else do
                (nextOffsetState, offsetBits, offsetValue) <-
                    encodeSymbol offsetState offsetCode offsetEntries offsetStates
                let writerA = addBits offsetValue offsetBits writer0
                (nextMatchState, matchBits, matchValue) <-
                    encodeSymbol matchState matchCode matchEntries matchStates
                let writerB = addBits matchValue matchBits writerA
                (nextLiteralState, literalBits, literalValue) <-
                    encodeSymbol literalState literalCode literalEntries literalStates
                let writerC = addBits literalValue literalBits writerB
                Right (writerC, nextLiteralState, nextMatchState, nextOffsetState)
    -- Extra bits, write order LL, ML, OF (a forward decoder reads these in
    -- order OF, ML, LL immediately after peeking symbols).
    let writer2 = addBits literalExtraValue (codeExtraBits literalRange) writer1
        writer3 = addBits matchExtraValue (codeExtraBits matchRange) writer2
        writer4 = addBits offsetExtra offsetCode writer3
    Right (writer4, nextLiteralState, nextMatchState, nextOffsetState)

-- | Initialise an FSE encoder state directly from a symbol, WITHOUT
-- flushing any bits -- the reverse-encoding-loop analogue of real zstd's
-- @FSE_initCState2@. Used only for the sequence processed first in
-- `encodeSequences`' reverse loop (the semantically last real sequence),
-- whose starting state a forward decoder never derives via a bit-consuming
-- update (see `decodeSequences`).
encodeInitState :: Int -> [EncodeEntry] -> [Int] -> Either String Int
encodeInitState symbol entries states = do
    entry <- indexEither "FSE symbol" entries symbol
    let deltaBits = encodeEntryDeltaBits entry
        nbBitsOut = (deltaBits + (1 `shiftL` 15)) `shiftR` 16
        value = (nbBitsOut `shiftL` 16) - deltaBits
        slot = (value `shiftR` nbBitsOut) + encodeEntryDeltaFindState entry
    indexEither "FSE encoder state" states slot

buildDecodeTable :: [Int] -> Int -> Either String [DecodeEntry]
buildDecodeTable normalized accuracyLog = do
    let size = 1 `shiftL` accuracyLog
    validateNormalizedCounts normalized size
    symbols <- spreadSymbols normalized size
    let initialNext = map effectiveCount normalized
    fst <$> foldM (buildEntry accuracyLog size) ([], initialNext) symbols
  where
    buildEntry accuracyLogValue size (entries, nextValues) symbol = do
        nextState <- indexEither "FSE symbol state" nextValues symbol
        let bits = accuracyLogValue - floorLog2 nextState
            entry = DecodeEntry symbol bits ((nextState `shiftL` bits) - size)
        Right (entries ++ [entry], replaceAt symbol (nextState + 1) nextValues)

buildEncodeTables :: [Int] -> Int -> Either String ([EncodeEntry], [Int])
buildEncodeTables normalized accuracyLog = do
    let size = 1 `shiftL` accuracyLog
        counts = map effectiveCount normalized
        cumulative = init (scanl (+) 0 counts)
    validateNormalizedCounts normalized size
    spread <- spreadSymbols normalized size
    (_, states) <-
        foldM
            (placeState cumulative size)
            (replicate (length normalized) 0, replicate size 0)
            (zip [0 ..] spread)
    let entries = zipWith (makeEncodeEntry accuracyLog) cumulative counts
    Right (entries, states)
  where
    placeState cumulative size (occurrences, states) (index, symbol) = do
        occurrence <- indexEither "FSE symbol occurrence" occurrences symbol
        base <- indexEither "FSE cumulative count" cumulative symbol
        let slot = base + occurrence
        when (slot < 0 || slot >= size) (Left "invalid FSE encoder state slot")
        Right
            ( replaceAt symbol (occurrence + 1) occurrences
            , replaceAt slot (index + size) states
            )

makeEncodeEntry :: Int -> Int -> Int -> EncodeEntry
makeEncodeEntry _ _ 0 = EncodeEntry 0 0
makeEncodeEntry accuracyLog cumulative count =
    EncodeEntry
        ((maxBits `shiftL` 16) - (count `shiftL` maxBits))
        (cumulative - count)
  where
    maxBits
        | count == 1 = accuracyLog
        | otherwise = accuracyLog - floorLog2 count

spreadSymbols :: [Int] -> Int -> Either String [Int]
spreadSymbols normalized size = do
    let lowSymbols = [symbol | (symbol, count) <- zip [0 ..] normalized, count == -1]
        lowPlacements = zip [size - 1, size - 2 ..] lowSymbols
        initial = foldl' (\values (slot, symbol) -> replaceAt slot symbol values) (replicate size 0) lowPlacements
        high = size - 1 - length lowSymbols
        step = (size `shiftR` 1) + (size `shiftR` 3) + 3
        -- A SINGLE pass over symbols 0..maxSymbolValue in ascending order,
        -- placing each symbol's full count immediately when encountered.
        -- This is the real algorithm (FSE_buildDTable_internal's low-
        -- probability branch, verified against the zstd C reference
        -- source). An earlier revision of this codec used a fabricated
        -- two-pass split -- all count>1 symbols first, then all count==1
        -- symbols, both in ascending symbol order -- which produces a
        -- DIFFERENT (but internally self-consistent) table layout: our own
        -- decoder mirrored our own encoder, so every round-trip test
        -- passed, but the real `zstd` CLI rejected our output as corrupt.
        -- See lessons.md Lesson 95.
        commonSymbols = concatMap expandSymbol (zip [0 ..] normalized)
    fst <$> foldM (placeSymbol size high step) (initial, 0) commonSymbols
  where
    expandSymbol (symbol, count)
        | count > 0 = replicate count symbol
        | otherwise = []

placeSymbol :: Int -> Int -> Int -> ([Int], Int) -> Int -> Either String ([Int], Int)
placeSymbol size high step (values, position) symbol = do
    when (position < 0 || position >= size) (Left "invalid FSE spread position")
    let nextValues = replaceAt position symbol values
        nextPosition = advancePosition ((position + step) .&. (size - 1))
    Right (nextValues, nextPosition)
  where
    advancePosition candidate
        | candidate <= high = candidate
        | otherwise = advancePosition ((candidate + step) .&. (size - 1))

encodeSymbol :: Int -> Int -> [EncodeEntry] -> [Int] -> Either String (Int, Int, Int)
encodeSymbol state symbol entries states = do
    entry <- indexEither "FSE symbol" entries symbol
    let bits = (state + encodeEntryDeltaBits entry) `shiftR` 16
        value = state .&. ((1 `shiftL` bits) - 1)
        slot = (state `shiftR` bits) + encodeEntryDeltaFindState entry
    nextState <- indexEither "FSE encoder state" states slot
    Right (nextState, bits, value)

lookupCodeRange :: String -> Int -> [CodeRange] -> Either String CodeRange
lookupCodeRange label code ranges
    | code < 0 || code >= length ranges = Left ("invalid " ++ label ++ " code")
    | otherwise = Right (ranges !! code)

valueToCode :: Int -> [CodeRange] -> Int
valueToCode value ranges =
    foldl'
        (\selected (index, range) -> if codeBaseline range <= value then index else selected)
        0
        (zip [0 ..] ranges)

validateNormalizedCounts :: [Int] -> Int -> Either String ()
validateNormalizedCounts normalized size = do
    when (any (< -1) normalized) (Left "invalid normalized FSE count")
    unless (sum (map effectiveCount normalized) == size) (Left "normalized FSE counts do not fill the table")

effectiveCount :: Int -> Int
effectiveCount (-1) = 1
effectiveCount value = max value 0

allEqual :: BS.ByteString -> Bool
allEqual input
    | BS.null input = True
    | otherwise = BS.all (== BS.head input) input

chunksOf :: Int -> BS.ByteString -> [BS.ByteString]
chunksOf _ input | BS.null input = []
chunksOf size input =
    let (chunk, rest) = BS.splitAt size input
     in chunk : chunksOf size rest

markLast :: [value] -> [(Bool, value)]
markLast [] = []
markLast [value] = [(True, value)]
markLast (value : rest) = (False, value) : markLast rest

writeBlockHeader :: Int -> Int -> Bool -> BS.ByteString
writeBlockHeader size blockType isLast =
    BS.pack
        [ fromIntegral value
        , fromIntegral (value `shiftR` 8)
        , fromIntegral (value `shiftR` 16)
        ]
  where
    value = (size `shiftL` 3) .|. (blockType `shiftL` 1) .|. if isLast then 1 else 0

magicBytes :: BS.ByteString
magicBytes = BS.pack [0x28, 0xB5, 0x2F, 0xFD]

encodeWord64LittleEndian :: Word64 -> BS.ByteString
encodeWord64LittleEndian value =
    BS.pack [fromIntegral (value `shiftR` (8 * index)) | index <- [0 .. 7]]

appendBytes :: Seq Word8 -> BS.ByteString -> Seq Word8
appendBytes = BS.foldl' (|>)

requireAvailable :: BS.ByteString -> Int -> Int -> String -> Either String ()
requireAvailable input position count field
    | position < 0 || count < 0 || position > BS.length input - count =
        Left ("truncated " ++ field)
    | otherwise = Right ()

ensureOutputLimit :: Int -> Int -> Either String ()
ensureOutputLimit current additional
    | additional < 0 || current > maxOutputSize - additional =
        Left ("decompressed size exceeds " ++ show maxOutputSize ++ " bytes")
    | otherwise = Right ()

byteAt :: BS.ByteString -> Int -> Int
byteAt input index = fromIntegral (BS.index input index)

floorLog2 :: Int -> Int
floorLog2 value
    | value <= 0 = error "floorLog2 requires a positive value"
    | otherwise = go 0 value
  where
    go result current
        | current < 2 = result
        | otherwise = go (result + 1) (current `shiftR` 1)

replaceAt :: Int -> value -> [value] -> [value]
replaceAt index value values =
    case splitAt index values of
        (before, _ : after) -> before ++ value : after
        _ -> values

indexEither :: String -> [value] -> Int -> Either String value
indexEither label values index
    | index < 0 || index >= length values = Left ("invalid " ++ label)
    | otherwise = Right (values !! index)
