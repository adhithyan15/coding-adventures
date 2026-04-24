-- | ZIP archive format — CMP09 (PKZIP, 1989).
--
-- ZIP bundles one or more files into a single @.zip@ archive, compressing each
-- entry independently with DEFLATE (method 8) or storing it verbatim (method
-- 0).  The same format underlies Java JARs, Office Open XML (@.docx@), Android
-- APKs, Python wheels, and many more.
--
-- == Architecture
--
-- @
-- ┌──────────────────────────────────────────────────────┐
-- │  [Local File Header + File Data]  ← entry 1          │
-- │  [Local File Header + File Data]  ← entry 2          │
-- │  ...                                                  │
-- │  ══════════ Central Directory ══════════              │
-- │  [Central Dir Header]  ← entry 1 (has local offset)  │
-- │  [Central Dir Header]  ← entry 2                     │
-- │  [End of Central Directory Record]                    │
-- └──────────────────────────────────────────────────────┘
-- @
--
-- The dual-header design enables two workflows:
--
-- * __Sequential write__: append Local Headers one-by-one, write CD at the end.
-- * __Random-access read__: seek to EOCD at the end, read CD, jump to any entry.
--
-- == Wire Format (all integers little-endian)
--
-- @
-- Local File Header (30 + n + e bytes):
-- [0x04034B50]  signature
-- [version_needed u16]  20=DEFLATE, 10=Stored
-- [flags u16]           bit 11 = UTF-8 filename
-- [method u16]          0=Stored, 8=DEFLATE
-- [mod_time u16]        MS-DOS packed time
-- [mod_date u16]        MS-DOS packed date
-- [crc32 u32]
-- [compressed_size u32]
-- [uncompressed_size u32]
-- [name_len u16]
-- [extra_len u16]
-- [name bytes...]
-- [extra bytes...]
-- [file data...]
-- @
--
-- == DEFLATE Inside ZIP
--
-- ZIP method 8 stores __raw RFC 1951 DEFLATE__ — no zlib wrapper (no CMF/FLG
-- header, no Adler-32 checksum). This implementation produces RFC 1951 fixed-
-- Huffman compressed blocks (BTYPE=01) using the 'Lzss' package for LZ77
-- match-finding.
--
-- == Series
--
-- @
-- CMP00 (LZ77,    1977) — Sliding-window back-references.
-- CMP01 (LZ78,    1978) — Explicit dictionary (trie).
-- CMP02 (LZSS,    1982) — LZ77 + flag bits.
-- CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
-- CMP04 (Huffman, 1952) — Entropy coding.
-- CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
-- CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← this package
-- @

module Zip
    ( -- * Data types
      ZipEntry(..)
      -- * Write
    , writeZip
      -- * Read
    , readZip
    , readEntry
      -- * Convenience
    , zip'
    , unzip'
      -- * Low-level (exported for tests)
    , crc32
    , deflateCompress
    , deflateDecompress
    ) where

import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8, Word16, Word32, Word64)
import Data.Bits ((.&.), (.|.), shiftL, shiftR, xor, testBit, complement, popCount)
import Data.List (foldl')
import qualified Data.Array as Array
import Lzss (Token(..), encode)

-- =============================================================================
-- Wire constants
-- =============================================================================
--
-- All ZIP integers are little-endian. The four magic signatures are written
-- LSB-first, so 0x04034B50 on disk is the bytes [0x50, 0x4B, 0x03, 0x04].

-- | Signature for Local File Header.
localSig :: Word32
localSig = 0x04034B50

-- | Signature for Central Directory File Header.
cdSig :: Word32
cdSig = 0x02014B50

-- | Signature for End of Central Directory record.
eocdSig :: Word32
eocdSig = 0x06054B50

-- | MS-DOS epoch: 1980-01-01 00:00:00.
--
-- ZIP timestamps are in the 16-bit MS-DOS packed format inherited from FAT:
--   Date: bits 15-9 = year-1980, bits 8-5 = month, bits 4-0 = day.
--   Time: bits 15-11 = hours, bits 10-5 = minutes, bits 4-0 = seconds\/2.
-- date=(0<<9)|(1<<5)|1=33=0x0021; time=0 → combined = 0x00210000.
dosEpoch :: Word32
dosEpoch = 0x00210000

-- | General-purpose bit flag: bit 11 = filename uses UTF-8 encoding.
gpFlags :: Word16
gpFlags = 0x0800

-- | Version needed for DEFLATE (2.0).
versionDeflate :: Word16
versionDeflate = 20

-- | Version needed for Stored (1.0).
versionStored :: Word16
versionStored = 10

-- | Version made by: Unix, specification version 3.0.
--   High byte = 3 (Unix), low byte = 30 (version 3.0) → 0x031E.
versionMadeBy :: Word16
versionMadeBy = 0x031E

-- =============================================================================
-- CRC-32
-- =============================================================================
--
-- CRC-32 uses polynomial 0xEDB88320 (the reflected form of 0x04C11DB7).
-- It detects accidental corruption of file data. It is NOT cryptographic —
-- for tamper detection, use a signed manifest or HMAC.
--
-- Algorithm:
--   1. XOR the running CRC with 0xFFFFFFFF.
--   2. For each byte b: crc = table[(crc XOR b) AND 0xFF] XOR (crc >> 8).
--   3. XOR again with 0xFFFFFFFF for the final value.
--
-- The table-driven approach pre-computes all 256 one-byte contributions,
-- making the per-byte cost a single XOR + table lookup + right shift.

-- | Build the 256-entry CRC-32 lookup table for polynomial 0xEDB88320.
buildCrcTable :: Array.Array Int Word32
buildCrcTable = Array.listArray (0, 255) (map entry [0..255])
  where
    entry i = foldl' step (fromIntegral i) ([0..7] :: [Int])
    step c _ =
        if c .&. 1 /= 0
            then 0xEDB88320 `xor` (c `shiftR` 1)
            else c `shiftR` 1

-- | The pre-built CRC-32 table.
crcTable :: Array.Array Int Word32
crcTable = buildCrcTable

-- | Compute CRC-32 over @data@, starting from @initial@ (use 0 for a fresh hash,
-- or the previous result for incremental updates).
--
-- Example: CRC-32("hello world") = 0x0D4A1185
-- Standard test vector: CRC-32("123456789") = 0xCBF43926
--
-- @
-- crc32 "hello world" 0 == 0x0D4A1185
-- -- incremental: same result
-- crc32 "world" (crc32 "hello " 0) == 0x0D4A1185
-- @
crc32 :: ByteString -> Word32 -> Word32
crc32 bs initial =
    let seed = initial `xor` 0xFFFFFFFF
        loop acc byte =
            let idx = fromIntegral ((acc `xor` fromIntegral byte) .&. 0xFF)
            in (crcTable Array.! idx) `xor` (acc `shiftR` 8)
        result = BS.foldl' loop seed bs
    in result `xor` 0xFFFFFFFF

-- =============================================================================
-- Little-endian I/O helpers
-- =============================================================================

-- | Serialise a little-endian 16-bit word into 2 bytes.
leWord16 :: Word16 -> [Word8]
leWord16 w = [fromIntegral w, fromIntegral (w `shiftR` 8)]

-- | Serialise a little-endian 32-bit word into 4 bytes.
leWord32 :: Word32 -> [Word8]
leWord32 w =
    [ fromIntegral  w
    , fromIntegral (w `shiftR`  8)
    , fromIntegral (w `shiftR` 16)
    , fromIntegral (w `shiftR` 24)
    ]

-- | Read a little-endian 16-bit word from a 'ByteString' at the given offset.
-- Returns 0 if out of bounds.
readLE16 :: ByteString -> Int -> Word16
readLE16 bs i
    | i + 1 >= BS.length bs = 0
    | otherwise =
        fromIntegral (BS.index bs  i      )
        .|. (fromIntegral (BS.index bs (i+1)) `shiftL` 8)

-- | Read a little-endian 32-bit word from a 'ByteString' at the given offset.
-- Returns 0 if out of bounds.
readLE32 :: ByteString -> Int -> Word32
readLE32 bs i
    | i + 3 >= BS.length bs = 0
    | otherwise =
         fromIntegral  (BS.index bs  i      )
        .|. (fromIntegral (BS.index bs (i+1)) `shiftL`  8)
        .|. (fromIntegral (BS.index bs (i+2)) `shiftL` 16)
        .|. (fromIntegral (BS.index bs (i+3)) `shiftL` 24)

-- | Safe version: returns Nothing if out of bounds.
readLE16M :: ByteString -> Int -> Maybe Word16
readLE16M bs i
    | i + 1 < BS.length bs = Just (readLE16 bs i)
    | otherwise             = Nothing

-- | Safe version: returns Nothing if out of bounds.
readLE32M :: ByteString -> Int -> Maybe Word32
readLE32M bs i
    | i + 3 < BS.length bs = Just (readLE32 bs i)
    | otherwise             = Nothing

-- =============================================================================
-- RFC 1951 DEFLATE — BitWriter
-- =============================================================================
--
-- RFC 1951 packs bits LSB-first within bytes. Huffman codes are sent with the
-- most-significant bit first — so before writing a Huffman code we reverse its
-- bits, then write the reversed value LSB-first. Extra bits (length/distance
-- extras, block headers) are written directly LSB-first without reversal.
--
-- Implementation: accumulate bits into a Word64 register. When 8 or more bits
-- are present, flush the low byte to the output list.

-- | State for the bit-packing writer.
data BitWriter = BitWriter
    { bwBuf  :: !Word64  -- ^ Bit register (LSB-first accumulator)
    , bwBits :: !Int     -- ^ Number of valid bits in bwBuf
    , bwOut  :: [Word8]  -- ^ Output bytes (accumulated in reverse for O(1) append)
    }

-- | Create an empty 'BitWriter'.
newBitWriter :: BitWriter
newBitWriter = BitWriter 0 0 []

-- | Write @nbits@ low bits of @value@, LSB-first (for extra bits and block headers).
writeLsb :: BitWriter -> Word32 -> Int -> BitWriter
writeLsb bw value nbits =
    let bw1 = bw { bwBuf  = bwBuf bw .|. (fromIntegral value `shiftL` bwBits bw)
                 , bwBits = bwBits bw + nbits
                 }
    in flushBytes bw1

-- | Write a Huffman code: reverse the top @nbits@ bits, then write LSB-first.
writeHuffman :: BitWriter -> Word32 -> Int -> BitWriter
writeHuffman bw code nbits =
    -- Reverse all 32 bits, then shift right to get just the nbits we want.
    let reversed = reverseBits32 code `shiftR` (32 - nbits)
    in writeLsb bw reversed nbits

-- | Flush any complete bytes (8+ bits) from the register to the output.
flushBytes :: BitWriter -> BitWriter
flushBytes bw
    | bwBits bw >= 8 =
        let byte = fromIntegral (bwBuf bw .&. 0xFF) :: Word8
        in flushBytes bw { bwBuf  = bwBuf bw `shiftR` 8
                         , bwBits = bwBits bw - 8
                         , bwOut  = byte : bwOut bw
                         }
    | otherwise = bw

-- | Align the writer to the next byte boundary (discard any partial-byte bits).
alignWriter :: BitWriter -> BitWriter
alignWriter bw
    | bwBits bw > 0 =
        let byte = fromIntegral (bwBuf bw .&. 0xFF) :: Word8
        in bw { bwBuf  = 0
              , bwBits = 0
              , bwOut  = byte : bwOut bw
              }
    | otherwise = bw

-- | Finish writing: return the accumulated bytes in forward order.
finishWriter :: BitWriter -> [Word8]
finishWriter bw = reverse (bwOut (alignWriter bw))

-- | Reverse all 32 bits of a 'Word32'.
reverseBits32 :: Word32 -> Word32
reverseBits32 w0 =
    -- Parallel bit-reversal using swap-and-shift.
    let w1 = ((w0 .&. 0xAAAAAAAA) `shiftR` 1) .|. ((w0 .&. 0x55555555) `shiftL` 1)
        w2 = ((w1 .&. 0xCCCCCCCC) `shiftR` 2) .|. ((w1 .&. 0x33333333) `shiftL` 2)
        w3 = ((w2 .&. 0xF0F0F0F0) `shiftR` 4) .|. ((w2 .&. 0x0F0F0F0F) `shiftL` 4)
        w4 = ((w3 .&. 0xFF00FF00) `shiftR` 8) .|. ((w3 .&. 0x00FF00FF) `shiftL` 8)
    in (w4 `shiftR` 16) .|. (w4 `shiftL` 16)

-- =============================================================================
-- RFC 1951 DEFLATE — Fixed Huffman Tables
-- =============================================================================
--
-- RFC 1951 §3.2.6 defines fixed (pre-agreed) Huffman code lengths so that
-- neither encoder nor decoder needs to transmit code tables. Using BTYPE=01
-- (fixed Huffman) achieves real compression via LZ77 without the overhead of
-- dynamic Huffman (BTYPE=10).
--
-- Literal/Length code lengths (and base codes):
--   Symbols   0–143: 8-bit codes, base  48 (0x30)
--   Symbols 144–255: 9-bit codes, base 400 (0x190)
--   Symbols 256–279: 7-bit codes, base   0 (0x00)   ← EOB symbol 256 = 7-bit 0
--   Symbols 280–287: 8-bit codes, base 192 (0xC0)
--
-- Distance codes:
--   Symbols 0–29: 5-bit codes equal to the symbol number.

-- | Return the RFC 1951 fixed Huffman (code, bit-width) for a LL symbol 0-287.
--
-- The codes are canonical: each range is a consecutive run starting at a
-- base value. Think of them as a sorted list of (length, symbol) pairs.
fixedLLEncode :: Word16 -> (Word32, Int)
fixedLLEncode sym
    | sym <= 143 = (0x30 + fromIntegral sym,             8)
    | sym <= 255 = (0x190 + fromIntegral (sym - 144),    9)
    | sym <= 279 = (fromIntegral (sym - 256),            7)
    | sym <= 287 = (0xC0 + fromIntegral (sym - 280),     8)
    | otherwise  = error ("fixedLLEncode: invalid symbol " ++ show sym)

-- =============================================================================
-- RFC 1951 DEFLATE — Length / Distance Tables
-- =============================================================================
--
-- Match lengths (3-255) map to LL symbols 257-284 plus extra bits.
-- Match distances (1-32768) map to distance codes 0-29 plus extra bits.
--
-- The tables encode (base, extra_bits) pairs:
--   actual_length   = base + extra_value
--   actual_distance = base + extra_value

-- | (base_length, extra_bits) for LL symbols 257..284 (index = symbol - 257).
lengthTable :: Array.Array Int (Int, Int)
lengthTable = Array.listArray (0, 27)
    [ (3,  0), (4,  0), (5,  0), (6,  0), (7,  0), (8,  0), (9,  0), (10, 0)  -- 257-264
    , (11, 1), (13, 1), (15, 1), (17, 1)                                         -- 265-268
    , (19, 2), (23, 2), (27, 2), (31, 2)                                         -- 269-272
    , (35, 3), (43, 3), (51, 3), (59, 3)                                         -- 273-276
    , (67, 4), (83, 4), (99, 4), (115, 4)                                        -- 277-280
    , (131, 5), (163, 5), (195, 5), (227, 5)                                     -- 281-284
    ]

-- | (base_distance, extra_bits) for distance codes 0..29.
distTable :: Array.Array Int (Int, Int)
distTable = Array.listArray (0, 29)
    [ (1,  0), (2,  0), (3,  0), (4,  0)
    , (5,  1), (7,  1), (9,  2), (13, 2)
    , (17, 3), (25, 3), (33, 4), (49, 4)
    , (65, 5), (97, 5), (129, 6), (193, 6)
    , (257, 7), (385, 7), (513, 8), (769, 8)
    , (1025, 9), (1537, 9), (2049, 10), (3073, 10)
    , (4097, 11), (6145, 11), (8193, 12), (12289, 12)
    , (16385, 13), (24577, 13)
    ]

-- | Map a match length (3-255) to (LL symbol, base, extra bits).
--
-- We scan the table from largest base to smallest so the first match wins.
encodeLength :: Int -> (Word16, Int, Int)
encodeLength len =
    let pairs = Array.assocs lengthTable
        go [] = error ("encodeLength: no entry for " ++ show len)
        go ((i, (base, extra)):rest)
            | len >= base = (257 + fromIntegral i, base, extra)
            | otherwise   = go rest
    in go (reverse pairs)

-- | Map a match offset (1-32768) to (distance code, base, extra bits).
encodeDistance :: Int -> (Int, Int, Int)
encodeDistance dist =
    let pairs = Array.assocs distTable
        go [] = error ("encodeDistance: no entry for " ++ show dist)
        go ((code, (base, extra)):rest)
            | dist >= base = (code, base, extra)
            | otherwise    = go rest
    in go (reverse pairs)

-- =============================================================================
-- RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
-- =============================================================================
--
-- Strategy:
--   1. If input is empty, emit a stored block (BFINAL=1, BTYPE=00, LEN=0).
--   2. Otherwise, run LZ77 match-finding via Lzss.encode (window=32768,
--      max_match=255, min_match=3).
--   3. Emit a single BTYPE=01 (fixed Huffman) block:
--      - Header: BFINAL=1, BTYPE=01 (3 bits)
--      - For each token:
--          Literal → write fixed LL Huffman code
--          Match   → write length LL code + extra bits + distance code + extra
--      - End-of-block symbol 256 → fixed LL Huffman code
--   4. Flush any remaining bits to a final byte.

-- | Compress @bs@ to a raw RFC 1951 DEFLATE bit-stream.
-- The output starts directly with the 3-bit block header — no zlib wrapper.
deflateCompress :: ByteString -> ByteString
deflateCompress bs
    | BS.null bs =
        -- Empty stored block: BFINAL=1 BTYPE=00 + 2-byte LEN=0 + 2-byte NLEN.
        --   BFINAL=1 BTYPE=00 → bit pattern (LSB-first) = 0b00000001 = 0x01
        --   LEN  = 0x0000
        --   NLEN = 0xFFFF
        BS.pack [0x01, 0x00, 0x00, 0xFF, 0xFF]
    | otherwise =
        -- Use LZSS to find LZ77 matches, then encode with fixed Huffman.
        -- window=32768, maxMatch=255, minMatch=3 — maps into RFC 1951 tables.
        let tokens = encode 32768 255 3 bs
            bw0    = newBitWriter
            -- Block header: BFINAL=1 (last block), BTYPE=01 (fixed Huffman).
            -- Written LSB-first: bit0=BFINAL, bit1-2=BTYPE.
            --   BFINAL=1, BTYPE=01 → value=0b011 over 3 bits.
            bw1    = writeLsb (writeLsb bw0 1 1) 1 2
            bw2    = foldl' encodeToken bw1 tokens
            -- End-of-block symbol 256.
            (eobCode, eobBits) = fixedLLEncode 256
            bw3    = writeHuffman bw2 eobCode eobBits
        in BS.pack (finishWriter bw3)

-- | Encode a single LZSS token into the BitWriter.
encodeToken :: BitWriter -> Token -> BitWriter
encodeToken bw (Literal b) =
    -- Literal byte: emit its fixed Huffman code.
    let (code, bits) = fixedLLEncode (fromIntegral b)
    in writeHuffman bw code bits
encodeToken bw (Match off len) =
    -- Back-reference: length LL code + extra bits + 5-bit distance code + extra.
    let (lSym, lBase, lExtra) = encodeLength len
        (lCode, lBits)        = fixedLLEncode lSym
        bw1 = writeHuffman bw lCode lBits
        bw2 = if lExtra > 0
                  then writeLsb bw1 (fromIntegral (len - lBase)) lExtra
                  else bw1
        (dCode, dBase, dExtra) = encodeDistance off
        -- Distance codes are 5-bit, written with bit reversal (Huffman style).
        bw3 = writeHuffman bw2 (fromIntegral dCode) 5
        bw4 = if dExtra > 0
                  then writeLsb bw3 (fromIntegral (off - dBase)) dExtra
                  else bw3
    in bw4

-- =============================================================================
-- RFC 1951 DEFLATE — BitReader
-- =============================================================================
--
-- Mirrors the BitWriter but reads bits from a ByteString, LSB-first.

-- | State for the bit-unpacking reader.
data BitReader = BitReader
    { brData :: ByteString  -- ^ Source data
    , brPos  :: !Int        -- ^ Current byte position
    , brBuf  :: !Word64     -- ^ Bit register
    , brBits :: !Int        -- ^ Number of valid bits in brBuf
    }

-- | Create a 'BitReader' over the given bytes.
newBitReader :: ByteString -> BitReader
newBitReader bs = BitReader bs 0 0 0

-- | Fill the register until at least @need@ bits are available.
-- Returns 'False' if the source is exhausted before enough bits are available.
fillBits :: BitReader -> Int -> (Bool, BitReader)
fillBits br need
    | brBits br >= need = (True, br)
    | brPos br >= BS.length (brData br) = (False, br)
    | otherwise =
        let byte = fromIntegral (BS.index (brData br) (brPos br)) :: Word64
            br'  = br { brBuf  = brBuf br .|. (byte `shiftL` brBits br)
                      , brBits = brBits br + 8
                      , brPos  = brPos br + 1
                      }
        in fillBits br' need

-- | Read @nbits@ bits LSB-first. Returns @(Just val, br')@ or @(Nothing, br)@.
readLsb :: BitReader -> Int -> (Maybe Word32, BitReader)
readLsb br 0     = (Just 0, br)
readLsb br nbits =
    let (ok, br1) = fillBits br nbits
    in if not ok
        then (Nothing, br)
        else
            let mask = (1 `shiftL` nbits) - 1
                val  = fromIntegral (brBuf br1 .&. mask) :: Word32
                br2  = br1 { brBuf  = brBuf br1 `shiftR` nbits
                           , brBits = brBits br1 - nbits
                           }
            in (Just val, br2)

-- | Read @nbits@ bits and bit-reverse them (for decoding MSB-first Huffman codes).
readMsb :: BitReader -> Int -> (Maybe Word32, BitReader)
readMsb br nbits =
    case readLsb br nbits of
        (Nothing, br') -> (Nothing, br')
        (Just v,  br') -> (Just (reverseBits32 v `shiftR` (32 - nbits)), br')

-- | Align the reader to the next byte boundary.
alignReader :: BitReader -> BitReader
alignReader br =
    let discard = brBits br `mod` 8
    in if discard > 0
        then br { brBuf  = brBuf br `shiftR` discard
                , brBits = brBits br - discard
                }
        else br

-- =============================================================================
-- RFC 1951 DEFLATE — Fixed Huffman Decode
-- =============================================================================
--
-- We decode the fixed LL codes by reading bits incrementally, shortest codes
-- first, and dispatching on ranges.
--
-- Code length map:
--   7-bit codes: symbols 256-279 (base code 0)
--   8-bit codes: symbols 0-143 (base 48) and 280-287 (base 192)
--   9-bit codes: symbols 144-255 (base 400)
--
-- Read 7 bits first. If the value is in 0-23, it's a 7-bit code (sym = v+256).
-- Otherwise read one more bit (making 8):
--   48-191 → symbol v - 48        (literals 0-143)
--   192-199 → symbol v + 88       (symbols 280-287)
-- Otherwise read one more bit (making 9):
--   400-511 → symbol v - 256      (literals 144-255)
-- Anything else is malformed.

-- | Decode one fixed Huffman LL symbol from @br@.
fixedLLDecode :: BitReader -> (Maybe Word16, BitReader)
fixedLLDecode br =
    case readMsb br 7 of
        (Nothing, br') -> (Nothing, br')
        (Just v7, br1) ->
            if v7 <= 23
                -- 7-bit code: symbols 256-279
                then (Just (fromIntegral v7 + 256), br1)
                else case readLsb br1 1 of
                    (Nothing, br') -> (Nothing, br')
                    (Just bit, br2) ->
                        let v8 = (v7 `shiftL` 1) .|. bit
                        in if v8 >= 48 && v8 <= 191
                            -- literals 0-143
                            then (Just (fromIntegral v8 - 48), br2)
                            else if v8 >= 192 && v8 <= 199
                                -- symbols 280-287
                                then (Just (fromIntegral v8 + 88), br2)
                                else case readLsb br2 1 of
                                    (Nothing, br') -> (Nothing, br')
                                    (Just bit2, br3) ->
                                        let v9 = (v8 `shiftL` 1) .|. bit2
                                        in if v9 >= 400 && v9 <= 511
                                            -- literals 144-255
                                            then (Just (fromIntegral v9 - 256), br3)
                                            else (Nothing, br3)

-- =============================================================================
-- RFC 1951 DEFLATE — Decompress
-- =============================================================================
--
-- Handles stored blocks (BTYPE=00) and fixed Huffman blocks (BTYPE=01).
-- Dynamic Huffman blocks (BTYPE=10) return an error.
--
-- After decompression, the output is capped at 256 MB to guard against
-- decompression-bomb attacks (crafted archives that expand to huge sizes).

-- | Decompress a raw RFC 1951 DEFLATE bit-stream.
-- Returns @Left msg@ on malformed or unsupported (BTYPE=10) input.
deflateDecompress :: ByteString -> Either String ByteString
deflateDecompress bs =
    go (newBitReader bs) []
  where
    maxOut = 256 * 1024 * 1024  -- 256 MB safety cap

    go br acc =
        case readLsb br 1 of
            (Nothing, _)       -> Left "deflate: unexpected EOF reading BFINAL"
            (Just bfinal, br1) ->
                case readLsb br1 2 of
                    (Nothing, _)      -> Left "deflate: unexpected EOF reading BTYPE"
                    (Just btype, br2) ->
                        case btype of
                            0 -> handleStored bfinal br2 acc
                            1 -> handleFixed  bfinal br2 acc
                            2 -> Left "deflate: dynamic Huffman (BTYPE=10) not supported"
                            _ -> Left "deflate: reserved BTYPE=11"

    handleStored bfinal br acc =
        -- Stored block: discard partial byte, read LEN + NLEN, copy bytes.
        let br1 = alignReader br
        in case readLsb br1 16 of
            (Nothing, _) -> Left "deflate: EOF reading stored LEN"
            (Just len, br2) ->
                case readLsb br2 16 of
                    (Nothing, _)     -> Left "deflate: EOF reading stored NLEN"
                    (Just nlen, br3) ->
                        if (nlen `xor` 0xFFFF) /= len
                            then Left "deflate: stored LEN/NLEN mismatch"
                            else let n = fromIntegral len
                                 in if length acc + n > maxOut
                                        then Left "deflate: output size limit exceeded"
                                        else copyStored (fromIntegral n) br3 acc bfinal

    copyStored 0 br acc bfinal =
        if bfinal == 1
            then Right (BS.pack (reverse acc))
            else go br acc
    copyStored n br acc bfinal =
        case readLsb br 8 of
            (Nothing, _) -> Left "deflate: EOF inside stored block"
            (Just b, br') ->
                if length acc >= maxOut
                    then Left "deflate: output size limit exceeded"
                    else copyStored (n-1) br' (fromIntegral b : acc) bfinal

    handleFixed bfinal br acc = decodeFixedSymbols bfinal br acc

    decodeFixedSymbols bfinal br acc =
        case fixedLLDecode br of
            (Nothing, _)    -> Left "deflate: EOF decoding Huffman symbol"
            (Just sym, br1) ->
                case sym of
                    256 ->
                        -- End-of-block
                        if bfinal == 1
                            then Right (BS.pack (reverse acc))
                            else go br1 acc
                    s | s <= 255 ->
                        -- Literal byte
                        if length acc >= maxOut
                            then Left "deflate: output size limit exceeded"
                            else decodeFixedSymbols bfinal br1 (fromIntegral s : acc)
                    s | s >= 257 && s <= 285 ->
                        -- Back-reference
                        let idx = fromIntegral s - 257
                        in if idx >= snd (Array.bounds lengthTable) + 1
                            then Left ("deflate: invalid length symbol " ++ show s)
                            else let (lBase, lExtra) = lengthTable Array.! idx
                                 in case readLsb br1 lExtra of
                                    (Nothing, _) -> Left "deflate: EOF reading length extra"
                                    (Just lv, br2) ->
                                        let len = lBase + fromIntegral lv
                                        in case readMsb br2 5 of
                                            (Nothing, _)      -> Left "deflate: EOF reading distance code"
                                            (Just dc, br3) ->
                                                if fromIntegral dc >= snd (Array.bounds distTable) + 1
                                                    then Left ("deflate: invalid distance code " ++ show dc)
                                                    else let (dBase, dExtra) = distTable Array.! fromIntegral dc
                                                         in case readLsb br3 dExtra of
                                                            (Nothing, _)   -> Left "deflate: EOF reading distance extra"
                                                            (Just dv, br4) ->
                                                                let dist = dBase + fromIntegral dv
                                                                    outLen = length acc
                                                                in if dist > outLen
                                                                    then Left ("deflate: back-ref offset " ++ show dist ++ " > output len " ++ show outLen)
                                                                    else if outLen + len > maxOut
                                                                        then Left "deflate: output size limit exceeded"
                                                                        else let copied = copyBackRef acc dist len
                                                                             in decodeFixedSymbols bfinal br4 (reverse copied ++ acc)
                    _ -> Left ("deflate: invalid LL symbol " ++ show sym)

-- | Copy @len@ bytes starting @dist@ positions back in the output buffer
-- (which is stored in reverse order in @acc@). Returns the copied bytes in
-- forward order.
--
-- The copy is done byte-by-byte so overlapping matches work correctly:
-- e.g. dist=1, len=6 applied to @[A]@ yields @[A, A, A, A, A, A, A]@.
copyBackRef :: [Word8] -> Int -> Int -> [Word8]
copyBackRef acc dist len = go acc dist len []
  where
    -- Each step: index into the current acc, append to result, then add that
    -- byte to acc so future steps see it (for overlapping matches).
    go _ _ 0 result = result
    go a d n result =
        -- acc is reversed, so position @outLen - dist@ from the front
        -- corresponds to index @dist - 1@ from the back of acc (index 0 of acc).
        let b = a !! (d - 1)  -- d-1 because acc[0] = output[outLen-1], acc[d-1] = output[outLen-d]
        in go (b : a) d (n - 1) (result ++ [b])

-- =============================================================================
-- ZIP Entry type
-- =============================================================================

-- | A single entry in a ZIP archive.
--
-- Entries can be files (any name) or directory markers (name ending with @\/@).
data ZipEntry = ZipEntry
    { entryName :: !ByteString  -- ^ Filename (UTF-8 encoded)
    , entryData :: !ByteString  -- ^ Decompressed content (empty for directories)
    } deriving (Show, Eq)

-- =============================================================================
-- Central Directory record (internal)
-- =============================================================================

-- | Metadata collected per entry during 'writeZip', used to build the Central
-- Directory that follows all the Local File Headers.
data CdRecord = CdRecord
    { cdName            :: !ByteString
    , cdMethod          :: !Word16
    , cdCrc             :: !Word32
    , cdCompressedSize  :: !Word32
    , cdUncompressedSize :: !Word32
    , cdLocalOffset     :: !Word32
    , cdExternalAttrs   :: !Word32
    }

-- =============================================================================
-- ZIP Write
-- =============================================================================

-- | Build a ZIP archive from a list of @(name, data, compress)@ triples.
--
-- * @name@ — UTF-8 filename. A name ending with @\/@  is a directory entry
--   (no data, method=0).
-- * @data@ — raw file bytes.
-- * @compress@ — if 'True', DEFLATE is attempted; we fall back to Stored if
--   DEFLATE does not reduce the size.
--
-- The archive is built in memory and returned as a single 'ByteString'.
writeZip :: [(ByteString, ByteString, Bool)] -> ByteString
writeZip entries =
    let (localBytes, cdRecords) = buildEntries entries 0 [] []
        cdOffset  = fromIntegral (BS.length localBytes)
        cdSection = concatMap buildCDHeader cdRecords
        cdBytes   = BS.pack cdSection
        cdSize    = fromIntegral (BS.length cdBytes)
        numEntries = fromIntegral (length cdRecords) :: Word16
        eocd = buildEOCD numEntries cdSize cdOffset
    in BS.concat [localBytes, cdBytes, BS.pack eocd]

-- | Recursively build Local File Headers + data for each entry.
-- Returns (concatenated bytes, list of CdRecords) in order.
buildEntries
    :: [(ByteString, ByteString, Bool)]
    -> Int          -- ^ Current byte offset in the archive
    -> [Word8]      -- ^ Accumulated local section bytes (reversed)
    -> [CdRecord]   -- ^ Accumulated CD records (reversed)
    -> (ByteString, [CdRecord])
buildEntries [] _ accBytes accCd =
    (BS.pack (reverse accBytes), reverse accCd)
buildEntries ((name, dat, compress):rest) offset accBytes accCd =
    let isDir       = not (BS.null name) && BS.last name == fromIntegral (fromEnum '/')
        crcVal      = crc32 dat 0
        uncompSize  = fromIntegral (BS.length dat) :: Word32
        -- Compress if requested and not a directory, fall back to Stored if
        -- it doesn't help.
        (method, fileData, externalAttrs)
            | isDir     = (0, BS.empty, 0o040755 `shiftL` 16)
            | compress && not (BS.null dat) =
                let compressed = deflateCompress dat
                in if BS.length compressed < BS.length dat
                    then (8, compressed, 0o100644 `shiftL` 16)
                    else (0, dat, 0o100644 `shiftL` 16)
            | otherwise = (0, dat, 0o100644 `shiftL` 16)
        compSize    = fromIntegral (BS.length fileData) :: Word32
        versionNeeded = if method == 8 then versionDeflate else versionStored
        localOffset = fromIntegral offset :: Word32
        -- Build Local File Header bytes.
        lfh = concat
            [ leWord32 localSig
            , leWord16 versionNeeded
            , leWord16 gpFlags
            , leWord16 method
            , leWord16 (fromIntegral (dosEpoch .&. 0xFFFF))         -- mod_time
            , leWord16 (fromIntegral ((dosEpoch `shiftR` 16) .&. 0xFFFF))  -- mod_date
            , leWord32 crcVal
            , leWord32 compSize
            , leWord32 uncompSize
            , leWord16 (fromIntegral (BS.length name))
            , leWord16 0                                             -- extra_len=0
            ]
        entryBytes  = lfh ++ BS.unpack name ++ BS.unpack fileData
        newOffset   = offset + length entryBytes
        cdRec = CdRecord
            { cdName             = name
            , cdMethod           = method
            , cdCrc              = crcVal
            , cdCompressedSize   = compSize
            , cdUncompressedSize = uncompSize
            , cdLocalOffset      = localOffset
            , cdExternalAttrs    = externalAttrs
            }
    in buildEntries rest newOffset (reverse entryBytes ++ accBytes) (cdRec : accCd)

-- | Build the Central Directory Header bytes for one entry.
buildCDHeader :: CdRecord -> [Word8]
buildCDHeader cd =
    let versionNeeded = if cdMethod cd == 8 then versionDeflate else versionStored
    in concat
        [ leWord32 cdSig
        , leWord16 versionMadeBy
        , leWord16 versionNeeded
        , leWord16 gpFlags
        , leWord16 (cdMethod cd)
        , leWord16 (fromIntegral (dosEpoch .&. 0xFFFF))           -- mod_time
        , leWord16 (fromIntegral ((dosEpoch `shiftR` 16) .&. 0xFFFF))  -- mod_date
        , leWord32 (cdCrc cd)
        , leWord32 (cdCompressedSize cd)
        , leWord32 (cdUncompressedSize cd)
        , leWord16 (fromIntegral (BS.length (cdName cd)))
        , leWord16 0                                               -- extra_len=0
        , leWord16 0                                               -- comment_len=0
        , leWord16 0                                               -- disk_start=0
        , leWord16 0                                               -- internal_attrs=0
        , leWord32 (cdExternalAttrs cd)
        , leWord32 (cdLocalOffset cd)
        ]
        ++ BS.unpack (cdName cd)

-- | Build the End of Central Directory record (22 bytes).
buildEOCD :: Word16 -> Word32 -> Word32 -> [Word8]
buildEOCD numEntries cdSize cdOffset =
    concat
        [ leWord32 eocdSig
        , leWord16 0              -- disk_number=0
        , leWord16 0              -- cd_disk=0
        , leWord16 numEntries     -- entries this disk
        , leWord16 numEntries     -- entries total
        , leWord32 cdSize
        , leWord32 cdOffset
        , leWord16 0              -- comment_len=0
        ]

-- =============================================================================
-- ZIP Read
-- =============================================================================

-- | Parse all entries from a ZIP archive using the EOCD-first strategy.
--
-- The reader:
--
-- 1. Scans backwards for the EOCD signature @PK\\x05\\x06@.
-- 2. Reads the CD offset and size from EOCD.
-- 3. Parses all Central Directory headers.
-- 4. For each file entry, reads and decompresses the data from the Local Header.
-- 5. Verifies CRC-32 for each file entry.
--
-- Returns @Left msg@ on structural errors or CRC mismatches.
readZip :: ByteString -> Either String [ZipEntry]
readZip bs = do
    eocdOff  <- maybe (Left "zip: no EOCD record found") Right (findEOCD bs)
    cdOffset <- maybe (Left "zip: EOCD truncated (cd_offset)") (Right . fromIntegral)
                    (readLE32M bs (eocdOff + 16))
    cdSize   <- maybe (Left "zip: EOCD truncated (cd_size)") (Right . fromIntegral)
                    (readLE32M bs (eocdOff + 12))
    if cdOffset + cdSize > BS.length bs
        then Left "zip: Central Directory out of bounds"
        else parseCentralDirectory bs cdOffset cdSize

-- | Parse all Central Directory headers starting at @cdOffset@.
parseCentralDirectory :: ByteString -> Int -> Int -> Either String [ZipEntry]
parseCentralDirectory bs cdOffset cdSize =
    go cdOffset []
  where
    cdEnd = cdOffset + cdSize

    go pos acc
        | pos + 4 > cdEnd = Right (reverse acc)
        | otherwise =
            let sig = readLE32 bs pos
            in if sig /= cdSig
                then Right (reverse acc)  -- end of CD (or padding)
                else do
                    entry <- parseCDEntry bs pos
                    let nextPos = pos + 46
                                  + fromIntegral (readLE16 bs (pos + 28))  -- name_len
                                  + fromIntegral (readLE16 bs (pos + 30))  -- extra_len
                                  + fromIntegral (readLE16 bs (pos + 32))  -- comment_len
                    go nextPos (entry : acc)

-- | Parse one Central Directory entry and read its file data.
parseCDEntry :: ByteString -> Int -> Either String ZipEntry
parseCDEntry bs pos = do
    let method         = readLE16 bs (pos + 10)
        storedCrc      = readLE32 bs (pos + 16)
        compSize       = fromIntegral (readLE32 bs (pos + 20)) :: Int
        uncompSize     = fromIntegral (readLE32 bs (pos + 24)) :: Int
        nameLen        = fromIntegral (readLE16 bs (pos + 28)) :: Int
        localOffset    = fromIntegral (readLE32 bs (pos + 42)) :: Int
        nameStart      = pos + 46
        nameBytes      = BS.take nameLen (BS.drop nameStart bs)
    -- Read data via the Local File Header.
    fileData <- readLocalData bs localOffset compSize uncompSize method
    -- Verify CRC-32.
    let actualCrc = crc32 fileData 0
    if actualCrc /= storedCrc && uncompSize > 0
        then Left ("zip: CRC-32 mismatch for '" ++ show nameBytes
                   ++ "': expected " ++ showHex storedCrc
                   ++ ", got " ++ showHex actualCrc)
        else Right (ZipEntry nameBytes fileData)

-- | Show a Word32 as uppercase hex (for error messages).
showHex :: Word32 -> String
showHex w = "0x" ++ map toHexChar [28,24,20,16,12,8,4,0]
  where
    toHexChar shift =
        let nibble = fromIntegral ((w `shiftR` shift) .&. 0xF) :: Int
        in "0123456789ABCDEF" !! nibble

-- | Read and decompress file data for one entry.
-- Uses the Local File Header to find the data start (accounting for
-- variable-length name and extra fields).
readLocalData :: ByteString -> Int -> Int -> Int -> Word16 -> Either String ByteString
readLocalData bs localOffset compSize uncompSize method = do
    -- The Local File Header has its own name_len and extra_len fields which may
    -- differ from the Central Directory. We read them to skip to the data.
    let lhNameLen  = fromIntegral (readLE16 bs (localOffset + 26)) :: Int
        lhExtraLen = fromIntegral (readLE16 bs (localOffset + 28)) :: Int
        dataStart  = localOffset + 30 + lhNameLen + lhExtraLen
        dataEnd    = dataStart + compSize
    if dataEnd > BS.length bs
        then Left "zip: entry data out of bounds"
        else do
            let compressed = BS.take compSize (BS.drop dataStart bs)
            case method of
                0 -> Right compressed   -- Stored: verbatim
                8 -> case deflateDecompress compressed of
                        Left err -> Left ("zip: DEFLATE: " ++ err)
                        Right d  -> Right (BS.take uncompSize d)
                m -> Left ("zip: unsupported method " ++ show m)

-- | Read a specific file by name from a ZIP archive.
-- Returns @Left msg@ if not found or on error.
readEntry :: ByteString  -- ^ Archive bytes
          -> ByteString  -- ^ Filename to find (UTF-8)
          -> Either String ByteString
readEntry archive name = do
    entries <- readZip archive
    case filter (\e -> entryName e == name) entries of
        []    -> Left ("zip: entry not found: " ++ show name)
        (e:_) -> Right (entryData e)

-- =============================================================================
-- EOCD finder
-- =============================================================================
--
-- The End of Central Directory record sits at the very end of the ZIP file.
-- Its exact offset varies because the EOCD may have a variable-length comment
-- (0-65535 bytes). We scan backwards from the end for the 4-byte signature
-- 0x06054B50 and validate that the comment_len field (at EOCD+20) matches
-- the remaining bytes.

-- | Scan backwards for the EOCD signature. Returns the byte offset if found.
findEOCD :: ByteString -> Maybe Int
findEOCD bs
    | BS.length bs < 22 = Nothing
    | otherwise =
        let scanStart = max 0 (BS.length bs - 22 - 65535)
            candidates = [BS.length bs - 22, BS.length bs - 23 .. scanStart]
        in foldr checkPos Nothing candidates
  where
    checkPos i acc =
        case readLE32M bs i of
            Just sig | sig == eocdSig ->
                case readLE16M bs (i + 20) of
                    Just commentLen
                        | i + 22 + fromIntegral commentLen == BS.length bs -> Just i
                    _ -> acc
            _ -> acc

-- =============================================================================
-- Convenience functions
-- =============================================================================

-- | Build a ZIP archive from a list of @(name, data)@ pairs.
-- Each file is compressed with DEFLATE if that reduces size; otherwise Stored.
--
-- Note: named @zip'@ (with prime) to avoid shadowing Prelude's 'zip'.
zip' :: [(ByteString, ByteString)] -> ByteString
zip' pairs = writeZip [(n, d, True) | (n, d) <- pairs]

-- | Decompress all file entries from a ZIP archive.
-- Returns @Right [(name, data)]@ in Central Directory order, skipping
-- directory entries (names ending with @\/@).
--
-- Note: named @unzip'@ (with prime) to avoid shadowing Prelude's 'unzip'.
unzip' :: ByteString -> Either String [(ByteString, ByteString)]
unzip' archive = do
    entries <- readZip archive
    Right [ (entryName e, entryData e)
          | e <- entries
          , not (isDirectory e)
          ]

-- | True if this entry is a directory (name ends with @\/@).
isDirectory :: ZipEntry -> Bool
isDirectory e = not (BS.null (entryName e))
             && BS.last (entryName e) == fromIntegral (fromEnum '/')
