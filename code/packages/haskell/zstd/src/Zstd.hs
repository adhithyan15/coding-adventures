-- | Zstandard (ZStd) lossless compression algorithm — CMP07.
--
-- Zstandard (RFC 8878) is a high-ratio, fast compression format created by
-- Yann Collet at Facebook (2015). It combines:
--
-- * __LZ77 back-references__ via LZSS token generation: the same
--   \"copy from earlier output\" trick as DEFLATE, with a 32 KB window.
-- * __FSE (Finite State Entropy)__ coding for sequence descriptor symbols.
--   FSE is an asymmetric numeral system that approaches Shannon entropy
--   in a single pass.
-- * __Predefined decode tables__ (RFC 8878 Appendix B) so short frames
--   need no table description overhead.
--
-- == Frame layout (RFC 8878 §3)
--
-- @
-- ┌────────┬─────┬──────────────────────┬────────┐
-- │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
-- │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │
-- └────────┴─────┴──────────────────────┴────────┘
-- @
--
-- Each __block__ has a 3-byte header:
--
-- @
-- bit 0       = Last_Block flag
-- bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
-- bits [23:3] = Block_Size
-- @
--
-- == Compression strategy (this implementation)
--
-- 1. Split data into 128 KB blocks (maxBlockSize).
-- 2. For each block, try:
--    a. __RLE__ — all bytes identical → 4 bytes total.
--    b. __Compressed__ (LZ77 + FSE) — if output < input length.
--    c. __Raw__ — verbatim copy as fallback.
--
-- == Series
--
-- @
-- CMP00 (LZ77)     — Sliding-window back-references
-- CMP01 (LZ78)     — Explicit dictionary (trie)
-- CMP02 (LZSS)     — LZ77 + flag bits
-- CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
-- CMP04 (Huffman)  — Entropy coding
-- CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
-- CMP06 (Brotli)   — DEFLATE + context modelling + static dict
-- CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this package
-- @
--
-- == Example
--
-- @
-- import Zstd (compress, decompress)
-- import qualified Data.ByteString as BS
--
-- let bs = BS.pack [104,101,108,108,111]   -- "hello"
-- decompress (compress bs) == Right bs     -- True
-- @

{-# LANGUAGE ScopedTypeVariables #-}

module Zstd
    ( compress
    , decompress
    ) where

import Data.Array (Array, (!), listArray, assocs, bounds)
import Data.Bits
    ( (.&.), (.|.), shiftL, shiftR
    , countLeadingZeros
    )
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8, Word16, Word32, Word64)
import qualified Lzss

-- ─── Constants ────────────────────────────────────────────────────────────────

-- | ZStd magic number: @0xFD2FB528@ (little-endian: @28 B5 2F FD@).
--
-- Every valid ZStd frame starts with these 4 bytes. The value was chosen to
-- be unlikely to appear at the start of plaintext files.
magic :: Word32
magic = 0xFD2FB528

-- | Maximum block size: 128 KB.
--
-- ZStd allows blocks up to 128 KB. Larger inputs are split across multiple
-- blocks.
maxBlockSize :: Int
maxBlockSize = 128 * 1024

-- ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ────────────────────────────
--
-- These tables map a /code number/ to a (baseline, extra_bits) pair.
--
-- For example, LL code 17 means @literal_length = 18 + read(1 extra bit)@,
-- so it covers literal lengths 18 and 19.

-- | Literal Length code table: @(baseline, extra_bits)@ for codes 0..35.
--
-- Literal lengths 0..15 each have their own code (0 extra bits).
-- Larger lengths are grouped with increasing ranges.
llCodes :: Array Int (Word32, Word8)
llCodes = listArray (0, 35)
    [ (0, 0),  (1, 0),  (2, 0),  (3, 0),  (4, 0),  (5, 0)
    , (6, 0),  (7, 0),  (8, 0),  (9, 0),  (10, 0), (11, 0)
    , (12, 0), (13, 0), (14, 0), (15, 0)
    , (16, 1), (18, 1), (20, 1), (22, 1)
    , (24, 2), (28, 2)
    , (32, 3), (40, 3)
    , (48, 4), (64, 6)
    , (128, 7), (256, 8), (512, 9), (1024, 10), (2048, 11), (4096, 12)
    , (8192, 13), (16384, 14), (32768, 15), (65536, 16)
    ]

-- | Match Length code table: @(baseline, extra_bits)@ for codes 0..52.
--
-- Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
mlCodes :: Array Int (Word32, Word8)
mlCodes = listArray (0, 52)
    [ (3, 0),  (4, 0),  (5, 0),  (6, 0),  (7, 0),  (8, 0)
    , (9, 0),  (10, 0), (11, 0), (12, 0), (13, 0), (14, 0)
    , (15, 0), (16, 0), (17, 0), (18, 0), (19, 0), (20, 0)
    , (21, 0), (22, 0), (23, 0), (24, 0), (25, 0), (26, 0)
    , (27, 0), (28, 0), (29, 0), (30, 0), (31, 0), (32, 0)
    , (33, 0), (34, 0)
    , (35, 1), (37, 1),  (39, 1),  (41, 1)
    , (43, 2), (47, 2)
    , (51, 3), (59, 3)
    , (67, 4), (83, 4)
    , (99, 5), (131, 7)
    , (259, 8), (515, 9), (1027, 10), (2051, 11)
    , (4099, 12), (8195, 13), (16387, 14), (32771, 15), (65539, 16)
    ]

-- ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────────
--
-- \"Predefined_Mode\" means no per-frame table description is transmitted.
-- The decoder builds the same table from these fixed distributions.
--
-- Entries of -1 mean \"probability 1/table_size\" — these symbols get one slot
-- in the decode table and their encoder state never needs extra bits.

-- | Predefined normalised distribution for Literal Length FSE.
-- Table accuracy log = 6 → 64 slots.
llNorm :: [Int]
llNorm =
    [  4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1
    ,  2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1
    , -1, -1, -1, -1
    ]

-- | LL table accuracy log. Table size = 2^6 = 64.
llAccLog :: Int
llAccLog = 6

-- | Predefined normalised distribution for Match Length FSE.
-- Table accuracy log = 6 → 64 slots.
mlNorm :: [Int]
mlNorm =
    [  1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1
    ,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1
    ,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1
    , -1, -1, -1, -1, -1
    ]

-- | ML table accuracy log. Table size = 2^6 = 64.
mlAccLog :: Int
mlAccLog = 6

-- | Predefined normalised distribution for Offset FSE.
-- Table accuracy log = 5 → 32 slots.
ofNorm :: [Int]
ofNorm =
    [  1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1
    ,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1
    ]

-- | OF table accuracy log. Table size = 2^5 = 32.
ofAccLog :: Int
ofAccLog = 5

-- ─── FSE decode table entry ───────────────────────────────────────────────────

-- | One cell in the FSE decode table.
--
-- To decode a symbol from state @s@:
--
-- 1. @sym@ is the output symbol.
-- 2. Read @nb@ bits from the bitstream as @bits@.
-- 3. New state = @base + bits@.
data FseDe = FseDe
    { fdSym  :: !Word8   -- ^ decoded symbol
    , fdNb   :: !Word8   -- ^ number of extra bits to read for next state
    , fdBase :: !Word16  -- ^ base value for next state computation
    } deriving (Show)

-- | Build an FSE decode table from a normalised probability distribution.
--
-- This exactly mirrors the Rust reference implementation.
--
-- The algorithm:
--
-- 1. Place symbols with probability -1 (very rare) at the top of the table.
--    They get exactly 1 slot each.
-- 2. Spread remaining symbols using a step function:
--    @step = (sz >> 1) + (sz >> 3) + 3@
--    Two-pass: count > 1 first, then count == 1.
-- 3. Assign @nb@ and @base@ to each slot (sym_next starts at norm count,
--    increments per slot).
buildDecodeTable :: [Int] -> Int -> Array Int FseDe
buildDecodeTable norm accLog =
    let sz    = 1 `shiftL` accLog
        step  = (sz `shiftR` 1) + (sz `shiftR` 3) + 3
        nSym  = length norm

        emptyDe = FseDe { fdSym = 0, fdNb = 0, fdBase = 0 }
        tbl0    = replicate sz emptyDe

        -- Phase 1: place -1-probability symbols at the top (high indices).
        -- The Rust code iterates forward but works from the top index downward.
        -- sym_next[s] = 1 for these symbols.
        (tbl1, highIdx, sn0) =
            foldl phase1Step (tbl0, sz - 1, replicate nSym 0) (zip [0..] norm)

        phase1Step (t, h, sn) (s, c)
            | c == (-1) =
                let t'  = setListElem t h (emptyDe { fdSym = fromIntegral s })
                    h'  = if h > 0 then h - 1 else 0
                    sn' = setListElem sn s 1
                in (t', h', sn')
            | otherwise = (t, h, sn)

        -- idxLimit = highest free slot for phase-2 spreading
        idxLimit = highIdx

        -- Phase 2: spread remaining symbols, two passes.
        -- Pass 0: symbols with count > 1 (multi-slot symbols first).
        -- Pass 1: symbols with count == 1 (single-slot symbols second).
        -- For each spread symbol, set sym_next[s] = count.
        (tbl2, _pos2, sn1) =
            foldl
                (\(t, p, sn) (passN :: Int) ->
                    foldl
                        (\(t', p', sn') (s, c) ->
                            if c <= 0 then (t', p', sn')
                            else
                                let wantPass = (passN == 0) == (c > 1)
                                in if not wantPass then (t', p', sn')
                                else
                                    let sn'' = setListElem sn' s c
                                        (t'', p'') = spreadSym t' p' s c step sz idxLimit
                                    in (t'', p'', sn'')
                        )
                        (t, p, sn)
                        (zip [0..] norm)
                )
                (tbl1, 0, sn0)
                [0, 1 :: Int]

        -- Phase 3: assign nb and base using sym_next.
        -- For each slot i (index order), sym_next[s] is the "ns" value:
        --   nb   = accLog - floor(log2(ns))
        --   base = ns * (1 << nb) - sz
        -- Then increment sym_next[s].
        (tbl3, _sn2) =
            foldl (assignNbBase sz accLog) (tbl2, sn1) [0..sz-1]

    in listArray (0, sz - 1) tbl3

-- | Place one symbol @cnt@ times into the table using the step function.
-- Positions above @idxLimit@ are skipped (they are reserved for -1 symbols).
spreadSym :: [FseDe] -> Int -> Int -> Int -> Int -> Int -> Int -> ([FseDe], Int)
spreadSym tbl pos sym cnt step sz idxLimit =
    go tbl pos cnt
  where
    emptyDe = FseDe { fdSym = 0, fdNb = 0, fdBase = 0 }
    go t p 0 = (t, p)
    go t p remaining =
        let t' = setListElem t p (emptyDe { fdSym = fromIntegral sym })
            p1 = (p + step) .&. (sz - 1)
            p' = skipAbove p1 step sz idxLimit
        in go t' p' (remaining - 1)

    skipAbove p st sz_ lim
        | p <= lim  = p
        | otherwise = skipAbove ((p + st) .&. (sz_ - 1)) st sz_ lim

-- | Assign nb and base to one decode table slot.
assignNbBase :: Int -> Int -> ([FseDe], [Int]) -> Int -> ([FseDe], [Int])
assignNbBase sz accLog (t, sn) i =
    let s   = fromIntegral (fdSym (t !! i)) :: Int
        ns  = sn !! s
        sn' = setListElem sn s (ns + 1)
        -- floor(log2(ns)) — same as 31 - countLeadingZeros(ns) for ns >= 1
        logNs = 31 - fromIntegral (countLeadingZeros (fromIntegral (max 1 ns) :: Word32))
        nb    = accLog - logNs
        -- base = ns * (1 << nb) - sz
        -- Note: this may wrap around in Word16 for small nb; that is correct,
        -- the decoder adds the read bits to recover a valid state index.
        base  = fromIntegral ((ns `shiftL` nb) - sz) :: Word16
        cell  = FseDe
            { fdSym  = fdSym (t !! i)
            , fdNb   = fromIntegral nb
            , fdBase = base
            }
    in (setListElem t i cell, sn')

-- ─── FSE encode symbol table entry ───────────────────────────────────────────

-- | Encode transform for one symbol.
--
-- Given encoder state @S@ for symbol @s@:
--
-- @
-- nb_out  = (S + delta_nb) >> 16
-- emit low nb_out bits of S
-- new_S   = state_tbl[(S >> nb_out) + delta_fs]
-- @
data FseEe = FseEe
    { eeDeltaNb :: !Word32  -- ^ @(max_bits_out << 16) - (count << max_bits_out)@
    , eeDeltaFs :: !Int     -- ^ @cumulative_count_before_sym - count@
    } deriving (Show)

-- | Build FSE encode tables from a normalised distribution.
--
-- Returns @(ee, st)@: the 'FseEe' transform for each symbol, and the encoder
-- state table (slot → output state in @[sz, 2*sz)@).
--
-- The encode and decode tables are duals: encoding a symbol from state @E@
-- produces bits that, when read by the decoder, reconstruct @E@.
buildEncodeTable :: [Int] -> Int -> ([FseEe], Array Int Word16)
buildEncodeTable norm accLog =
    let sz    = 1 `shiftL` accLog
        step  = (sz `shiftR` 1) + (sz `shiftR` 3) + 3
        nSym  = length norm

        -- Count for each symbol: 1 if norm == -1, else max(0, norm).
        cnt :: Int -> Int
        cnt s = if (norm !! s) == (-1) then 1 else max 0 (norm !! s)

        -- Cumulative counts (before symbol s).
        cumuls :: [Int]
        cumuls = scanl (\acc s -> acc + cnt s) 0 [0..nSym-1]

        cumul :: Int -> Int
        cumul s = cumuls !! s

        emptyDe = FseDe { fdSym = 0, fdNb = 0, fdBase = 0 }

        -- Build the spread table using the same algorithm as buildDecodeTable.
        spread0 = replicate sz emptyDe

        -- Phase 1: -1 symbols at high end
        (spread1, highIdx, _sn0) =
            foldl
                (\(sp, h, sn) (s, c) ->
                    if c == (-1)
                    then
                        let sp' = setListElem sp h (emptyDe { fdSym = fromIntegral s })
                            h'  = if h > 0 then h - 1 else 0
                            sn' = setListElem sn s 1
                        in (sp', h', sn')
                    else (sp, h, sn)
                )
                (spread0, sz - 1, replicate nSym 0)
                (zip [0..] norm)

        idxLimit = highIdx

        -- Phase 2: spread remaining symbols
        (spread2, _pos2, _sn1) =
            foldl
                (\(sp, p, sn) (passN :: Int) ->
                    foldl
                        (\(sp', p', sn') (s, c) ->
                            if c <= 0 then (sp', p', sn')
                            else
                                let wantPass = (passN == 0) == (c > 1)
                                in if not wantPass then (sp', p', sn')
                                else
                                    let sn'' = setListElem sn' s c
                                        (sp'', p'') = spreadSym sp' p' s c step sz idxLimit
                                    in (sp'', p'', sn'')
                        )
                        (sp, p, sn)
                        (zip [0..] norm)
                )
                (spread1, 0, replicate nSym 0)
                [0, 1 :: Int]

        -- Build the state table by iterating spread in index order.
        -- For each index i, sym s = spread[i].
        -- occurrence j = how many times we've seen s before index i.
        -- encode_slot = cumul[s] + j
        -- encoder output state = i + sz
        (st, _symOcc) =
            foldl
                (\(stAcc, occAcc) i ->
                    let s    = fromIntegral (fdSym (spread2 !! i)) :: Int
                        j    = occAcc !! s
                        slot = cumul s + j
                        stAcc'  = setListElem stAcc slot (fromIntegral (i + sz) :: Word16)
                        occAcc' = setListElem occAcc s (j + 1)
                    in (stAcc', occAcc')
                )
                (replicate sz 0, replicate nSym 0)
                [0..sz-1]

        -- Build FseEe entries for each symbol.
        -- mbo = max_bits_out = accLog - floor(log2(cnt)) for cnt > 1, else accLog
        -- delta_nb = (mbo << 16) - (cnt << mbo)
        -- delta_fs = cumul[s] - cnt
        mkEe :: Int -> FseEe
        mkEe s =
            let c   = cnt s
                mbo = if c == 1
                      then accLog
                      else accLog - (31 - fromIntegral (countLeadingZeros (fromIntegral c :: Word32)))
                deltaNb = fromIntegral ((mbo `shiftL` 16) - (c `shiftL` mbo)) :: Word32
                deltaFs = cumul s - c
            in if c == 0
               then FseEe { eeDeltaNb = 0, eeDeltaFs = 0 }
               else FseEe { eeDeltaNb = deltaNb, eeDeltaFs = deltaFs }

        ee  = map mkEe [0..nSym-1]
        stA = listArray (0, sz - 1) st

    in (ee, stA)

-- ─── Reverse bit-writer ───────────────────────────────────────────────────────
--
-- ZStd's sequence bitstream is written /backwards/ relative to the data flow:
-- the encoder writes bits that the decoder will read last, first. This allows
-- the decoder to read a forward-only stream while decoding sequences in order.
--
-- Byte layout: @[byte0, byte1, ..., byteN]@ where @byteN@ is the last byte
-- written and contains a __sentinel bit__ (the highest set bit) that marks the
-- end of meaningful data. The decoder initialises by finding this sentinel.
--
-- Bit layout within each byte: LSB = first bit written.

-- | Reverse bit-writer state.
data RevBitWriter = RevBitWriter
    { rbwBuf  :: ![Word8]  -- ^ accumulated bytes (in reverse order)
    , rbwReg  :: !Word64   -- ^ accumulation register (bits fill from LSB)
    , rbwBits :: !Int      -- ^ number of valid bits in reg
    }

-- | Create a new empty 'RevBitWriter'.
newRevBitWriter :: RevBitWriter
newRevBitWriter = RevBitWriter { rbwBuf = [], rbwReg = 0, rbwBits = 0 }

-- | Add @nb@ low-order bits of @val@ to the stream.
--
-- Bits accumulate in the register from LSB upward. When 8 or more bits are
-- present, the lowest 8 are flushed to the buffer as a byte.
addBits :: RevBitWriter -> Word64 -> Int -> RevBitWriter
addBits bw _   0  = bw
addBits bw val nb =
    let mask  = if nb >= 64 then maxBound else (1 `shiftL` nb) - 1 :: Word64
        reg'  = rbwReg bw .|. ((val .&. mask) `shiftL` rbwBits bw)
        bits' = rbwBits bw + nb
    in flushBytes bw { rbwReg = reg', rbwBits = bits' }
  where
    flushBytes b
        | rbwBits b >= 8 =
            let byte = fromIntegral (rbwReg b .&. 0xFF) :: Word8
            in flushBytes b { rbwReg  = rbwReg  b `shiftR` 8
                            , rbwBits = rbwBits b - 8
                            , rbwBuf  = byte : rbwBuf b
                            }
        | otherwise = b

-- | Flush remaining bits with a sentinel bit, then return the complete buffer.
--
-- The sentinel is a @1@ bit placed at position @rbwBits@ in the last byte.
-- The decoder locates it with @countLeadingZeros@ arithmetic.
flushRevBitWriter :: RevBitWriter -> [Word8]
flushRevBitWriter bw =
    let sentinel  = fromIntegral ((1 :: Word64) `shiftL` rbwBits bw) :: Word8
        lastByte  = fromIntegral (rbwReg bw .&. 0xFF) .|. sentinel
    in reverse (lastByte : rbwBuf bw)

-- ─── Reverse bit-reader ───────────────────────────────────────────────────────
--
-- Mirrors 'RevBitWriter': reads bits from the END of the buffer going backwards.
-- The stream is laid out so that the LAST bits written by the encoder are at
-- the END of the byte buffer. The reader initialises at the last byte and reads
-- backward toward byte 0.
--
-- Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
-- @readBits n@ extracts the top @n@ bits and shifts the register left by @n@.

-- | Reverse bit-reader state.
data RevBitReader = RevBitReader
    { rbrData :: !ByteString  -- ^ full bitstream buffer
    , rbrReg  :: !Word64      -- ^ shift register, valid bits packed at the TOP
    , rbrBits :: !Int         -- ^ how many valid bits are loaded
    , rbrPos  :: !Int         -- ^ index of next byte to load (decrements toward 0)
    }

-- | Initialise a 'RevBitReader' from a bitstream buffer.
--
-- Finds the sentinel bit in the last byte and initialises the register with
-- the valid data bits below the sentinel.
newRevBitReader :: ByteString -> Either String RevBitReader
newRevBitReader bs
    | BS.null bs = Left "empty bitstream"
    | otherwise  =
        let lastByte = BS.index bs (BS.length bs - 1)
        in if lastByte == 0
           then Left "bitstream last byte is zero (no sentinel)"
           else
               -- sentinel_pos = bit index of the highest set bit
               let sentinelPos = 7 - fromIntegral (countLeadingZeros (fromIntegral lastByte :: Word32))
                                   + (32 - 8 :: Int)  -- adjust for Word32 width
                   -- Actually: countLeadingZeros on Word32 includes the leading 24 zero bits.
                   -- For a Word8 value in Word32: clz(x) = 24 + clz8(x).
                   -- The highest set bit in the byte is at position (7 - clz8(x)).
                   -- clz8(x) = clz32(x) - 24
                   -- So: highest bit = 7 - (clz32(x) - 24) = 31 - clz32(x)
                   -- But sentinel_pos in the byte is 31 - clz32(x) which might be > 7.
                   -- We want the bit position within the byte (0-7).
                   -- Let's use: sentPos = 7 - (fromIntegral (countLeadingZeros (fromIntegral lastByte :: Word8)))
                   -- but Word8 countLeadingZeros... let's just compute it directly.
                   sentPos     = highBit lastByte
                   validBits   = sentPos  -- bits below the sentinel
                   mask :: Word64
                   mask        = if validBits == 0 then 0
                                 else (1 `shiftL` validBits) - 1
                   reg :: Word64
                   reg         = if validBits == 0 then 0
                                 else (fromIntegral lastByte .&. mask)
                                     `shiftL` (64 - validBits)
                   r = RevBitReader
                       { rbrData = bs
                       , rbrReg  = reg
                       , rbrBits = validBits
                       , rbrPos  = BS.length bs - 1
                       }
               in Right (reloadRbr r)

-- | Find the bit position (0-indexed from LSB) of the highest set bit in a Word8.
-- Returns 0 if no bit is set (though we guard against lastByte == 0 before calling).
highBit :: Word8 -> Int
highBit 0 = 0
highBit b =
    -- countLeadingZeros on Word32 includes the 24 leading zeros for a Word8 value
    7 - (fromIntegral (countLeadingZeros (fromIntegral b :: Word32)) - 24)

-- | Load more bytes into the register from the stream going backward.
--
-- Each new byte is placed just BELOW the currently loaded bits in the
-- left-aligned register.
reloadRbr :: RevBitReader -> RevBitReader
reloadRbr r
    | rbrBits r > 56 = r
    | rbrPos  r <= 0 = r
    | otherwise      =
        let pos'   = rbrPos r - 1
            byte   = fromIntegral (BS.index (rbrData r) pos') :: Word64
            shift  = 64 - rbrBits r - 8
            reg'   = rbrReg r .|. (byte `shiftL` shift)
            bits'  = rbrBits r + 8
            r'     = r { rbrReg = reg', rbrBits = bits', rbrPos = pos' }
        in reloadRbr r'

-- | Read @nb@ bits from the top of the register.
--
-- Returns the most recently written bits first (mirroring the encoder's
-- backward order). Returns @(0, reader)@ if @nb == 0@.
readBits :: RevBitReader -> Int -> (Word64, RevBitReader)
readBits r 0  = (0, r)
readBits r nb =
    let val   = rbrReg r `shiftR` (64 - nb)
        reg'  = if nb >= 64 then 0 else rbrReg r `shiftL` nb
        bits' = max 0 (rbrBits r - nb)
        r'    = r { rbrReg = reg', rbrBits = bits' }
        r''   = if bits' < 24 then reloadRbr r' else r'
    in (val, r'')

-- ─── FSE encode/decode helpers ────────────────────────────────────────────────

-- | Encode one symbol into the backward bitstream, updating the FSE state.
--
-- The encoder maintains state in @[sz, 2*sz)@. To emit symbol @sym@:
--
-- 1. Compute how many bits to flush: @nb = (state + delta_nb) >> 16@
-- 2. Write the low @nb@ bits of @state@ to the bitstream.
-- 3. New state = @st[(state >> nb) + delta_fs]@
fseEncodeSym
    :: Word32           -- ^ current FSE state
    -> Int              -- ^ symbol to encode
    -> [FseEe]          -- ^ encode entry table (indexed by symbol)
    -> Array Int Word16 -- ^ state table (indexed by slot)
    -> RevBitWriter     -- ^ bit writer
    -> (Word32, RevBitWriter)
fseEncodeSym state sym ee st bw =
    let e    = ee !! sym
        nb   = fromIntegral ((state + eeDeltaNb e) `shiftR` 16) :: Int
        bw'  = addBits bw (fromIntegral state) nb
        slot = fromIntegral (state `shiftR` fromIntegral nb) + eeDeltaFs e
        newState = fromIntegral (st ! max 0 slot)
    in (newState, bw')

-- | Decode one symbol from the backward bitstream, updating the FSE state.
--
-- 1. Look up @de[state]@ to get @sym@, @nb@, and @base@.
-- 2. New state = @base + read(nb bits)@.
fseDecodeSym
    :: Word16           -- ^ current FSE state
    -> Array Int FseDe  -- ^ decode table
    -> RevBitReader     -- ^ bit reader
    -> (Word8, Word16, RevBitReader)
fseDecodeSym state de br =
    let e           = de ! fromIntegral state
        sym         = fdSym e
        (bits, br') = readBits br (fromIntegral (fdNb e))
        newState    = fdBase e + fromIntegral bits
    in (sym, newState, br')

-- ─── LL/ML/OF code number computation ────────────────────────────────────────

-- | Map a literal length value to its LL code number (0..35).
--
-- Codes 0..15 are identity; codes 16+ cover ranges via lookup.
-- We scan the table and return the last code whose baseline ≤ ll.
llToCode :: Word32 -> Int
llToCode ll =
    fst $ foldl
        (\(best, _) (i, (base, _)) ->
            if base <= ll then (i, base) else (best, base))
        (0, 0 :: Word32)
        (assocs llCodes)

-- | Map a match length value to its ML code number (0..52).
mlToCode :: Word32 -> Int
mlToCode ml =
    fst $ foldl
        (\(best, _) (i, (base, _)) ->
            if base <= ml then (i, base) else (best, base))
        (0, 0 :: Word32)
        (assocs mlCodes)

-- ─── Sequence type ───────────────────────────────────────────────────────────

-- | One ZStd sequence: @(literal_length, match_length, match_offset)@.
--
-- A sequence means: emit @ll@ literal bytes from the literals section,
-- then copy @ml@ bytes starting @off@ positions back in the output buffer.
-- After all sequences, any remaining literals are appended.
data ZSeq = ZSeq
    { zLL  :: !Word32  -- ^ literal length (bytes before this match)
    , zML  :: !Word32  -- ^ match length (bytes to copy from history)
    , zOff :: !Word32  -- ^ match offset (1-indexed: 1 = last byte written)
    } deriving (Show)

-- | Convert LZSS tokens into ZStd sequences + a flat literals buffer.
--
-- LZSS produces @Literal(byte)@ and @Match{offset, length}@ tokens.
-- ZStd groups consecutive literals before each match into a single sequence.
-- Any trailing literals go into the literals buffer without a sequence entry.
tokensToSeqs :: [Lzss.Token] -> ([Word8], [ZSeq])
tokensToSeqs tokens =
    let (litsRev, seqsRev, _) =
            foldl step ([], [], 0) tokens
    in (reverse litsRev, reverse seqsRev)
  where
    step (lits, seqs, litRun) (Lzss.Literal b) =
        (b : lits, seqs, litRun + 1)
    step (lits, seqs, litRun) (Lzss.Match off len) =
        let s = ZSeq
                { zLL  = litRun
                , zML  = fromIntegral len
                , zOff = fromIntegral off
                }
        in (lits, s : seqs, 0)

-- ─── Literals section encoding ────────────────────────────────────────────────
--
-- ZStd literals can be Huffman-coded or raw. We use __Raw_Literals__ (type=0),
-- which is the simplest: no Huffman table, bytes are stored verbatim.
--
-- Header format depends on literal count:
--
-- @
-- n ≤ 31:    1-byte header = (n << 3) | 0x00   (size_format=00, type=00)
-- n ≤ 4095:  2-byte header = (n << 4) | 0x04   (size_format=01, type=00)
-- else:       3-byte header = (n << 4) | 0x0C   (size_format=11, type=00)
-- @

-- | Encode the literals section as Raw_Literals.
encodeLiteralsSection :: [Word8] -> [Word8]
encodeLiteralsSection lits =
    let n = length lits
        header
            | n <= 31   =
                -- 1-byte header: size_format=00, type=00
                [ fromIntegral (n `shiftL` 3) :: Word8 ]
            | n <= 4095 =
                -- 2-byte header: size_format=01 (0x4), type=00
                -- Total tag = 0x04
                let hdr = (n `shiftL` 4) .|. 4 :: Int
                in [ fromIntegral (hdr .&. 0xFF)
                   , fromIntegral ((hdr `shiftR` 8) .&. 0xFF)
                   ]
            | otherwise =
                -- 3-byte header: size_format=11 (0xC), type=00
                -- Total tag = 0x0C
                let hdr = (n `shiftL` 4) .|. 0x0C :: Int
                in [ fromIntegral (hdr .&. 0xFF)
                   , fromIntegral ((hdr `shiftR` 8) .&. 0xFF)
                   , fromIntegral ((hdr `shiftR` 16) .&. 0xFF)
                   ]
    in header ++ lits

-- | Decode the literals section, returning @(literals, bytes_consumed)@.
decodeLiteralsSection :: ByteString -> Either String ([Word8], Int)
decodeLiteralsSection bs
    | BS.null bs = Left "empty literals section"
    | otherwise  =
        let b0    = BS.index bs 0
            ltype = fromIntegral b0 .&. (3 :: Int)
        in if ltype /= 0
           then Left ("unsupported literals type " ++ show ltype)
           else
               let sizeFmt = (fromIntegral b0 `shiftR` 2) .&. (3 :: Int)
               in case sizeFmt of
                   0 -> parseHdr1 bs
                   2 -> parseHdr1 bs
                   1 -> parseHdr2 bs
                   3 -> parseHdr3 bs
                   _ -> Left "impossible size_format"
  where
    parseHdr1 bs' =
        let b0 = BS.index bs' 0
            n  = fromIntegral (fromIntegral b0 `shiftR` (3 :: Int) :: Int) :: Int
            end = 1 + n
        in if end > BS.length bs'
           then Left ("literals truncated: need " ++ show end)
           else Right (BS.unpack (BS.take n (BS.drop 1 bs')), end)

    parseHdr2 bs'
        | BS.length bs' < 2 = Left "truncated literals header (2-byte)"
        | otherwise =
            let b0 = fromIntegral (BS.index bs' 0) :: Int
                b1 = fromIntegral (BS.index bs' 1) :: Int
                n  = (b0 `shiftR` 4) .|. (b1 `shiftL` 4)
                end = 2 + n
            in if end > BS.length bs'
               then Left ("literals truncated: need " ++ show end)
               else Right (BS.unpack (BS.take n (BS.drop 2 bs')), end)

    parseHdr3 bs'
        | BS.length bs' < 3 = Left "truncated literals header (3-byte)"
        | otherwise =
            let b0 = fromIntegral (BS.index bs' 0) :: Int
                b1 = fromIntegral (BS.index bs' 1) :: Int
                b2 = fromIntegral (BS.index bs' 2) :: Int
                n  = (b0 `shiftR` 4) .|. (b1 `shiftL` 4) .|. (b2 `shiftL` 12)
                end = 3 + n
            in if end > BS.length bs'
               then Left ("literals truncated: need " ++ show end)
               else Right (BS.unpack (BS.take n (BS.drop 3 bs')), end)

-- ─── Sequences section encoding ───────────────────────────────────────────────
--
-- Layout:
--
-- @
-- [sequence_count:          1-3 bytes]
-- [symbol_compression_modes: 1 byte]   (0x00 = all Predefined)
-- [FSE bitstream:           variable]
-- @
--
-- The FSE bitstream is a backward bit-stream:
--   - Sequences are encoded in REVERSE ORDER (last first).
--   - For each sequence: OF extra bits, ML extra bits, LL extra bits,
--     then FSE encode ML, OF, LL symbols.
--   - After all sequences, flush final states:
--       (state_of - sz_of) as OF_ACC_LOG bits
--       (state_ml - sz_ml) as ML_ACC_LOG bits
--       (state_ll - sz_ll) as LL_ACC_LOG bits
--   - Add sentinel and flush.
--
-- Decoder reads in the mirror order:
--   Read LL_ACC_LOG bits → state_ll, ML_ACC_LOG → state_ml, OF_ACC_LOG → state_of
--   For each sequence:
--     decode LL, OF, ML symbols (state transitions)
--     read LL extra bits, ML extra bits, OF extra bits

-- | Encode the sequence count as 1, 2, or 3 bytes.
encodeSeqCount :: Int -> [Word8]
encodeSeqCount 0   = [0]
encodeSeqCount n
    | n < 128    = [fromIntegral n]
    | n < 0x7FFF =
        let v = fromIntegral n .|. 0x8000 :: Word16
        in [ fromIntegral (v .&. 0xFF)
           , fromIntegral (v `shiftR` 8)
           ]
    | otherwise  =
        let r = n - 0x7F00
        in [ 0xFF
           , fromIntegral (r .&. 0xFF)
           , fromIntegral ((r `shiftR` 8) .&. 0xFF)
           ]

-- | Decode the sequence count from 1, 2, or 3 bytes starting at @pos@.
decodeSeqCount :: ByteString -> Int -> Either String (Int, Int)
decodeSeqCount bs pos
    | pos >= BS.length bs = Left "empty sequence count"
    | otherwise =
        let b0 = BS.index bs pos
        in if b0 < 128
           then Right (fromIntegral b0, 1)
           else if b0 < 0xFF
           then if pos + 1 >= BS.length bs
                then Left "truncated sequence count (2-byte)"
                else
                    let b1 = BS.index bs (pos + 1)
                        v  = fromIntegral b0 .|. (fromIntegral b1 `shiftL` 8) :: Word16
                        n  = fromIntegral (v .&. 0x7FFF) :: Int
                    in Right (n, 2)
           else if pos + 2 >= BS.length bs
                then Left "truncated sequence count (3-byte)"
                else
                    let b1 = fromIntegral (BS.index bs (pos + 1)) :: Int
                        b2 = fromIntegral (BS.index bs (pos + 2)) :: Int
                        n  = 0x7F00 + b1 + (b2 `shiftL` 8)
                    in Right (n, 3)

-- | Encode the sequences section using predefined FSE tables.
encodeSequencesSection :: [ZSeq] -> [Word8]
encodeSequencesSection seqs =
    let (eeLl, stLl) = buildEncodeTable llNorm llAccLog
        (eeMl, stMl) = buildEncodeTable mlNorm mlAccLog
        (eeOf, stOf) = buildEncodeTable ofNorm ofAccLog

        szLl = 1 `shiftL` llAccLog :: Word32
        szMl = 1 `shiftL` mlAccLog :: Word32
        szOf = 1 `shiftL` ofAccLog :: Word32

        -- Encode sequences in reverse order, starting states at sz.
        (finalStLl, finalStMl, finalStOf, bwFinal) =
            foldl
                (encodeOneSeq eeLl stLl eeMl stMl eeOf stOf)
                (szLl, szMl, szOf, newRevBitWriter)
                (reverse seqs)

        -- Flush final FSE states.
        bw1 = addBits bwFinal (fromIntegral (finalStOf - szOf)) ofAccLog
        bw2 = addBits bw1     (fromIntegral (finalStMl - szMl)) mlAccLog
        bw3 = addBits bw2     (fromIntegral (finalStLl - szLl)) llAccLog

    in flushRevBitWriter bw3

-- | Encode one sequence into the backward bitstream.
encodeOneSeq
    :: [FseEe] -> Array Int Word16   -- LL encode tables
    -> [FseEe] -> Array Int Word16   -- ML encode tables
    -> [FseEe] -> Array Int Word16   -- OF encode tables
    -> (Word32, Word32, Word32, RevBitWriter)
    -> ZSeq
    -> (Word32, Word32, Word32, RevBitWriter)
encodeOneSeq eeLl stLl eeMl stMl eeOf stOf (stL, stM, stO, bw) sq =
    let llCode = llToCode (zLL sq)
        mlCode = mlToCode (zML sq)

        -- Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
        -- code = floor(log2(raw)); extra = raw - (1 << code)
        rawOff  = zOff sq + 3
        ofCode  = if rawOff <= 1 then 0
                  else 31 - fromIntegral (countLeadingZeros (fromIntegral rawOff :: Word32))
        ofExtra = rawOff - (1 `shiftL` ofCode)

        -- Write extra bits: OF, ML, LL (in this order in the backward stream).
        (_, llExBits) = llCodes ! llCode
        (_, mlExBits) = mlCodes ! mlCode
        llExVal = zLL sq - fst (llCodes ! llCode)
        mlExVal = zML sq - fst (mlCodes ! mlCode)

        bw1 = addBits bw  (fromIntegral ofExtra) ofCode
        bw2 = addBits bw1 (fromIntegral mlExVal) (fromIntegral mlExBits)
        bw3 = addBits bw2 (fromIntegral llExVal) (fromIntegral llExBits)

        -- FSE encode symbols. Decode order: LL, OF, ML.
        -- Encode order (reversed): ML, OF, LL.
        (stM', bw4) = fseEncodeSym stM mlCode eeMl stMl bw3
        (stO', bw5) = fseEncodeSym stO ofCode eeOf stOf bw4
        (stL', bw6) = fseEncodeSym stL llCode eeLl stLl bw5

    in (stL', stM', stO', bw6)

-- ─── Block-level compress ─────────────────────────────────────────────────────

-- | Compress one block into ZStd compressed block format.
--
-- Returns 'Nothing' if the compressed form would not be smaller than the
-- original (caller should fall back to a Raw block).
compressBlock :: ByteString -> Maybe [Word8]
compressBlock block =
    let tokens          = Lzss.encode 32768 255 3 block
        (lits, seqs)    = tokensToSeqs tokens
    in if null seqs
       then Nothing
       else
           let litSection  = encodeLiteralsSection lits
               seqCountB   = encodeSeqCount (length seqs)
               modesByte   = [0x00 :: Word8]  -- all Predefined
               bitstream   = encodeSequencesSection seqs
               out         = litSection ++ seqCountB ++ modesByte ++ bitstream
           in if length out >= BS.length block
              then Nothing
              else Just out

-- | Decompress one ZStd compressed block into output.
decompressBlock :: ByteString -> [Word8] -> Either String [Word8]
decompressBlock bs out0 = do
    -- Parse literals section.
    (lits, litConsumed) <- decodeLiteralsSection bs
    let pos0 = litConsumed
    if pos0 >= BS.length bs
        then return (out0 ++ lits)   -- only literals, no sequences
        else do
            (nSeqs, scBytes) <- decodeSeqCount bs pos0
            let pos1 = pos0 + scBytes
            if nSeqs == 0
                then return (out0 ++ lits)
                else do
                    if pos1 >= BS.length bs
                        then Left "missing symbol compression modes byte"
                        else do
                            let modesByte = BS.index bs pos1
                                pos2 = pos1 + 1
                                llMode = (fromIntegral modesByte `shiftR` 6) .&. (3 :: Int)
                                ofMode = (fromIntegral modesByte `shiftR` 4) .&. (3 :: Int)
                                mlMode = (fromIntegral modesByte `shiftR` 2) .&. (3 :: Int)
                            if llMode /= 0 || ofMode /= 0 || mlMode /= 0
                                then Left ("unsupported FSE modes: LL="
                                           ++ show llMode ++ " OF=" ++ show ofMode
                                           ++ " ML=" ++ show mlMode)
                                else do
                                    let bitstream = BS.drop pos2 bs
                                    br <- newRevBitReader bitstream
                                    let dtLl = buildDecodeTable llNorm llAccLog
                                        dtMl = buildDecodeTable mlNorm mlAccLog
                                        dtOf = buildDecodeTable ofNorm ofAccLog
                                        (sLl, br1) = readBits br llAccLog
                                        (sMl, br2) = readBits br1 mlAccLog
                                        (sOf, br3) = readBits br2 ofAccLog
                                    applySeqs dtLl dtMl dtOf
                                              (fromIntegral sLl)
                                              (fromIntegral sMl)
                                              (fromIntegral sOf)
                                              br3
                                              lits 0
                                              out0
                                              nSeqs

-- | Apply @n@ ZStd sequences to the output buffer.
--
-- For each sequence: emit @ll@ literals, then copy @ml@ bytes from @off@
-- positions back in the output.
applySeqs
    :: Array Int FseDe
    -> Array Int FseDe
    -> Array Int FseDe
    -> Word16           -- ^ LL FSE state
    -> Word16           -- ^ ML FSE state
    -> Word16           -- ^ OF FSE state
    -> RevBitReader
    -> [Word8]          -- ^ literals buffer
    -> Int              -- ^ current position in literals buffer
    -> [Word8]          -- ^ output buffer
    -> Int              -- ^ remaining sequences
    -> Either String [Word8]
applySeqs _ _ _ _ _ _ _ lits litPos out 0 =
    Right (out ++ drop litPos lits)
applySeqs dtLl dtMl dtOf sLl sMl sOf br lits litPos out n = do
    let (llCode, sLl', br1) = fseDecodeSym sLl dtLl br
        (ofCode, sOf', br2) = fseDecodeSym sOf dtOf br1
        (mlCode, sMl', br3) = fseDecodeSym sMl dtMl br2

    let llIdx = fromIntegral llCode :: Int
        mlIdx = fromIntegral mlCode :: Int
    if llIdx > snd (bounds llCodes)
        then Left ("invalid LL code " ++ show llCode)
        else if mlIdx > snd (bounds mlCodes)
             then Left ("invalid ML code " ++ show mlCode)
             else do
                 let (llBase, llExBits) = llCodes ! llIdx
                     (mlBase, mlExBits) = mlCodes ! mlIdx

                     (llExVal, br4) = readBits br3 (fromIntegral llExBits)
                     (mlExVal, br5) = readBits br4 (fromIntegral mlExBits)
                     (ofExVal, br6) = readBits br5 (fromIntegral ofCode)

                     ll     = llBase + fromIntegral llExVal
                     ml     = mlBase + fromIntegral mlExVal
                     ofCode' = fromIntegral ofCode :: Int
                     ofRaw  = (1 `shiftL` ofCode' :: Word32) .|. fromIntegral ofExVal

                 offset <- case safeSub ofRaw 3 of
                               Just v  -> Right v
                               Nothing -> Left ("offset underflow: raw=" ++ show ofRaw)

                 let litEnd = litPos + fromIntegral ll
                 if litEnd > length lits
                     then Left ("literal overrun: pos=" ++ show litPos
                                ++ " ll=" ++ show ll
                                ++ " buf=" ++ show (length lits))
                     else do
                         let newLits = take (fromIntegral ll) (drop litPos lits)
                             out'    = out ++ newLits
                         if fromIntegral offset == 0 || fromIntegral offset > length out'
                             then Left ("bad match offset " ++ show offset
                                        ++ " (output len " ++ show (length out') ++ ")")
                             else do
                                 let copyStart = length out' - fromIntegral offset
                                     copied    = copyBytes out' copyStart (fromIntegral ml)
                                     out''     = out' ++ copied
                                 applySeqs dtLl dtMl dtOf sLl' sMl' sOf' br6
                                           lits litEnd out'' (n - 1)

-- | Safe subtraction: returns 'Nothing' if a < b.
safeSub :: Word32 -> Word32 -> Maybe Word32
safeSub a b
    | a >= b    = Just (a - b)
    | otherwise = Nothing

-- | Copy @len@ bytes starting at @start@ from the output buffer.
-- Done byte-by-byte to handle overlapping matches correctly.
copyBytes :: [Word8] -> Int -> Int -> [Word8]
copyBytes _   _     0   = []
copyBytes out start len =
    let b    = out !! start
        out' = out ++ [b]
    in b : copyBytes out' (start + 1) (len - 1)

-- ─── List manipulation helpers ────────────────────────────────────────────────

-- | Set one element in a list at index @i@. O(n).
setListElem :: [a] -> Int -> a -> [a]
setListElem []     _ _ = []
setListElem (_:xs) 0 v = v : xs
setListElem (x:xs) n v = x : setListElem xs (n - 1) v

-- ─── Little-endian helpers ────────────────────────────────────────────────────

-- | Encode a 'Word32' as 4 little-endian bytes.
leWord32 :: Word32 -> [Word8]
leWord32 w =
    [ fromIntegral (w .&. 0xFF)
    , fromIntegral ((w `shiftR` 8)  .&. 0xFF)
    , fromIntegral ((w `shiftR` 16) .&. 0xFF)
    , fromIntegral ((w `shiftR` 24) .&. 0xFF)
    ]

-- | Encode a 'Word64' as 8 little-endian bytes.
leWord64 :: Word64 -> [Word8]
leWord64 w =
    [ fromIntegral (w .&. 0xFF)
    , fromIntegral ((w `shiftR` 8)  .&. 0xFF)
    , fromIntegral ((w `shiftR` 16) .&. 0xFF)
    , fromIntegral ((w `shiftR` 24) .&. 0xFF)
    , fromIntegral ((w `shiftR` 32) .&. 0xFF)
    , fromIntegral ((w `shiftR` 40) .&. 0xFF)
    , fromIntegral ((w `shiftR` 48) .&. 0xFF)
    , fromIntegral ((w `shiftR` 56) .&. 0xFF)
    ]

-- | Decode a 3-byte little-endian value from 'ByteString' at offset @i@.
leWord24 :: ByteString -> Int -> Word32
leWord24 bs i =
    fromIntegral (BS.index bs i)
    .|. (fromIntegral (BS.index bs (i + 1)) `shiftL` 8)
    .|. (fromIntegral (BS.index bs (i + 2)) `shiftL` 16)

-- ─── Public API ───────────────────────────────────────────────────────────────

-- | Compress @input@ to ZStd format (RFC 8878).
--
-- Produces a valid ZStd frame decompressible by the @zstd@ CLI tool or any
-- conforming implementation.
--
-- == Frame header layout
--
-- @
-- [0..3]   Magic number: 0xFD2FB528 (LE)
-- [4]      FHD = 0xE0: FCS_flag=11 (8-byte FCS), Single_Segment=1
-- [5..12]  Frame_Content_Size (8 bytes LE)
-- [13..]   Blocks
-- @
--
-- == Example
--
-- @
-- let bs = Data.ByteString.Char8.pack "hello world"
-- decompress (compress bs) == Right bs
-- @
compress :: ByteString -> ByteString
compress input =
    BS.pack (frameHeader ++ encodeBlocks 0)
  where
    frameHeader =
        leWord32 magic                               -- 4 bytes magic
        ++ [0xE0]                                    -- FHD: FCS=11, SingleSeg=1
        ++ leWord64 (fromIntegral (BS.length input)) -- 8 bytes FCS

    encodeBlocks :: Int -> [Word8]
    encodeBlocks offset
        | BS.null input && offset == 0 =
            -- Empty input: one empty raw block.
            -- Last=1, Type=Raw(00), Size=0 → header = 0x01 0x00 0x00
            take 3 (leWord32 1)
        | offset >= BS.length input = []
        | otherwise =
            let end    = min (offset + maxBlockSize) (BS.length input)
                block  = BS.take (end - offset) (BS.drop offset input)
                isLast = end == BS.length input
            in encodeOneBlock block isLast ++ encodeBlocks end

    encodeOneBlock :: ByteString -> Bool -> [Word8]
    encodeOneBlock block isLast
        -- Try RLE: all bytes identical
        | not (BS.null block) && BS.all (== BS.head block) block =
            let sz  = BS.length block
                -- Block header: (size << 3) | (type=01 << 1) | last
                hdr = ((sz `shiftL` 3) .|. (1 `shiftL` 1) .|. lastBit) :: Int
            in take 3 (leWord32 (fromIntegral hdr)) ++ [BS.head block]
        -- Try compressed
        | otherwise =
            case compressBlock block of
                Just compressed ->
                    let sz  = length compressed
                        hdr = ((sz `shiftL` 3) .|. (2 `shiftL` 1) .|. lastBit) :: Int
                    in take 3 (leWord32 (fromIntegral hdr)) ++ compressed
                Nothing ->
                    -- Raw block fallback
                    let sz  = BS.length block
                        hdr = ((sz `shiftL` 3) .|. (0 `shiftL` 1) .|. lastBit) :: Int
                    in take 3 (leWord32 (fromIntegral hdr)) ++ BS.unpack block
      where
        lastBit = if isLast then 1 else 0 :: Int

-- | Decompress a ZStd frame, returning the original data or an error.
--
-- Accepts any valid ZStd frame with:
-- * Single-segment or multi-segment layout
-- * Raw, RLE, or Compressed blocks
-- * Predefined FSE modes (no per-frame table description)
--
-- Returns @Left msg@ if the input is truncated, has a bad magic number,
-- or contains unsupported features.
--
-- == Example
--
-- @
-- let original = Data.ByteString.Char8.pack "hello, world!"
-- decompress (compress original) == Right original
-- @
decompress :: ByteString -> Either String ByteString
decompress bs
    | BS.length bs < 5 = Left "frame too short"
    | otherwise =
        let magic' = fromIntegral (BS.index bs 0)
                   .|. (fromIntegral (BS.index bs 1) `shiftL` 8)
                   .|. (fromIntegral (BS.index bs 2) `shiftL` 16)
                   .|. (fromIntegral (BS.index bs 3) `shiftL` 24) :: Word32
        in if magic' /= magic
           then Left ("bad magic: " ++ show magic')
           else parseFrame bs 4

-- | Parse a ZStd frame header and dispatch to block parsing.
parseFrame :: ByteString -> Int -> Either String ByteString
parseFrame bs pos0 =
    let fhd       = BS.index bs pos0
        fcsFlag   = (fromIntegral fhd `shiftR` 6) .&. (3 :: Int)
        singleSeg = (fromIntegral fhd `shiftR` 5) .&. (1 :: Int)
        dictFlag  = fromIntegral fhd .&. (3 :: Int)
        pos1      = pos0 + 1

        -- Skip Window_Descriptor if Single_Segment = 0
        pos2 = if singleSeg == 0 then pos1 + 1 else pos1

        -- Skip Dict ID
        dictIdBytes = [0, 1, 2, 4] !! dictFlag
        pos3        = pos2 + dictIdBytes

        -- Skip Frame Content Size
        fcsBytes = case fcsFlag of
                     0 -> if singleSeg == 1 then 1 else 0
                     1 -> 2
                     2 -> 4
                     3 -> 8
                     _ -> 0
        pos4 = pos3 + fcsBytes

    in parseBlocks bs pos4 []

-- | Parse all blocks, accumulating output.
parseBlocks :: ByteString -> Int -> [Word8] -> Either String ByteString
parseBlocks bs pos out
    | pos + 3 > BS.length bs = Left "truncated block header"
    | otherwise =
        let hdr    = leWord24 bs pos
            isLast = (hdr .&. 1) /= 0
            btype  = fromIntegral ((hdr `shiftR` 1) .&. 3) :: Int
            bsize  = fromIntegral (hdr `shiftR` 3) :: Int
            pos'   = pos + 3
        in case btype of
               0 ->  -- Raw block: bsize bytes verbatim
                   if pos' + bsize > BS.length bs
                   then Left ("raw block truncated: need " ++ show bsize)
                   else
                       let newOut = out ++ BS.unpack (BS.take bsize (BS.drop pos' bs))
                       in if isLast then Right (BS.pack newOut)
                          else parseBlocks bs (pos' + bsize) newOut

               1 ->  -- RLE block: 1 byte repeated bsize times
                   if pos' >= BS.length bs
                   then Left "RLE block missing byte"
                   else
                       let byte   = BS.index bs pos'
                           newOut = out ++ replicate bsize byte
                       in if isLast then Right (BS.pack newOut)
                          else parseBlocks bs (pos' + 1) newOut

               2 ->  -- Compressed block
                   if pos' + bsize > BS.length bs
                   then Left ("compressed block truncated: need " ++ show bsize)
                   else
                       case decompressBlock (BS.take bsize (BS.drop pos' bs)) out of
                           Left  e    -> Left e
                           Right out' ->
                               if isLast then Right (BS.pack out')
                               else parseBlocks bs (pos' + bsize) out'

               3 -> Left "reserved block type 3"
               _ -> Left "impossible block type"
