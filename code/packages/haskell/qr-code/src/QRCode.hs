-- | QRCode — ISO/IEC 18004:2015 QR Code encoder.
--
-- == Overview
--
-- This module encodes any UTF-8 string into a scannable QR Code symbol. The
-- output is a 'ModuleGrid' (abstract boolean grid) that can be passed to
-- 'barcode-2d's 'layout' function for pixel rendering.
--
-- == Encoding pipeline
--
-- @
-- input string
--   → mode selection    (numeric / alphanumeric / byte)
--   → version selection (smallest v1–40 that fits at the ECC level)
--   → bit stream        (mode indicator + char count + data + padding)
--   → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
--   → interleave        (data CWs round-robin, then ECC CWs)
--   → grid init         (finder × 3, separators, timing, alignment, format, dark)
--   → zigzag placement  (two-column snake from bottom-right)
--   → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
--   → finalize          (format info + version info v7+)
--   → ModuleGrid
-- @
--
-- == QR Code Symbol Structure
--
-- A QR Code is a square grid of modules — dark (True) or light (False) cells.
-- The grid size is (4V + 17) × (4V + 17) where V is the version from 1 to 40.
--
-- @
--   Version 1:  21×21  modules
--   Version 2:  25×25  modules
--   Version 40: 177×177 modules
-- @
--
-- == Reed-Solomon Error Correction
--
-- QR uses Reed-Solomon over GF(256) with the primitive polynomial:
-- @p(x) = x^8 + x^4 + x^3 + x^2 + 1  (0x11D = 285)@
--
-- The generator polynomial convention is b=0:
-- @g(x) = (x + α^0)(x + α^1)...(x + α^{n-1})@
-- where α = 2 is the primitive element.
module QRCode
  ( -- * Error correction level
    EccLevel (..)

    -- * Encoding modes (exposed for testing)
  , EncodingMode (..)

    -- * Public API
  , encode
  , encodeAndLayout
  , renderSvg

    -- * Error type
  , QRCodeError (..)

    -- * Internal functions (exposed for testing)
  , eccIndicator
  , symbolSize
  , numRawDataModules
  , numDataCodewords
  , numBlocks
  , eccCwPerBlock
  , buildGenerator
  , rsEncode
  , selectMode
  , buildDataCodewords
  , Block (..)
  , computeBlocks
  , interleaveBlocks
  , computeFormatBits
  , computeVersionBits
  ) where

import Data.Bits (xor, shiftR, shiftL, (.&.), (.|.))
import Data.Char (ord, isDigit)
import Data.List (minimumBy)
import Data.Ord  (comparing)
import qualified Data.Vector as V

import GF256 (gfMul, gfPow)
import CodingAdventures.Barcode2D
  ( ModuleGrid (..)
  , ModuleShape (..)
  , Barcode2DLayoutConfig (..)
  , emptyGrid
  , setModule
  , layout
  )
import CodingAdventures.PaintInstructions (PaintScene)

-- ---------------------------------------------------------------------------
-- Public types
-- ---------------------------------------------------------------------------

-- | Error correction level.
--
-- Higher levels recover from more damage but hold less data.
--
-- @
-- L → ~7%  of codewords recoverable
-- M → ~15% of codewords recoverable  (common default for URLs)
-- Q → ~25% of codewords recoverable
-- H → ~30% of codewords recoverable  (most redundant)
-- @
data EccLevel = L | M | Q | H
  deriving (Show, Eq, Ord)

-- | Errors produced by the QR encoder.
data QRCodeError
  = InputTooLong String
    -- ^ Input does not fit in any QR version at the chosen ECC level.
  | LayoutError String
    -- ^ Layout configuration was invalid.
  deriving (Show)

-- ---------------------------------------------------------------------------
-- ECC level constants
-- ---------------------------------------------------------------------------

-- | 2-bit ECC level indicator (per ISO 18004 Table C.1).
--
-- Note the unintuitive mapping: L and M swap their expected bit patterns.
--
-- @
--   L → 01,  M → 00,  Q → 11,  H → 10
-- @
eccIndicator :: EccLevel -> Int
eccIndicator L = 1   -- 0b01
eccIndicator M = 0   -- 0b00
eccIndicator Q = 3   -- 0b11
eccIndicator H = 2   -- 0b10

-- | Index into the capacity tables (L=0, M=1, Q=2, H=3).
eccIdx :: EccLevel -> Int
eccIdx L = 0
eccIdx M = 1
eccIdx Q = 2
eccIdx H = 3

-- ---------------------------------------------------------------------------
-- ISO 18004:2015 — ECC codewords per block table (Table 9)
-- ---------------------------------------------------------------------------
--
-- The table has 4 ECC levels × 40 versions. Index 0 is a sentinel (-1)
-- because QR version numbers are 1-based.
--
-- These values determine how many Reed-Solomon error correction codewords are
-- computed for each data block.

-- | ECC codewords per block, indexed [eccIdx][version].
--
-- Version 0 is a dummy -1 sentinel to make 1-based indexing natural.
eccCwPerBlock :: [[Int]]
eccCwPerBlock =
  [ -- L:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [ -1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30]
    -- M:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  , [ -1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28]
    -- Q:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  , [ -1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30]
    -- H:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  , [ -1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30]
  ]

-- | Number of error correction blocks, indexed [eccIdx][version].
--
-- Multiple blocks improve burst-error resilience: a contiguous damaged region
-- will destroy at most one or two blocks, leaving the others recoverable.
numBlocks :: [[Int]]
numBlocks =
  [ -- L:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [ -1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25]
    -- M:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  , [ -1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49]
    -- Q:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  , [ -1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68]
    -- H:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  , [ -1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80]
  ]

-- ---------------------------------------------------------------------------
-- Alignment pattern positions table
-- ---------------------------------------------------------------------------
--
-- Per ISO/IEC 18004:2015 Annex E. Each entry lists the center coordinates
-- for alignment patterns. The actual alignment pattern grid is every pair
-- (r, c) from these coordinates, minus any that overlap finder patterns.
--
-- Version 1 has no alignment patterns. Version 2 has one at center (18, 18).

-- | Center coordinates for alignment patterns, indexed [version-1].
alignmentPositions :: [[Int]]
alignmentPositions =
  [ []                                          -- v1
  , [6, 18]                                     -- v2
  , [6, 22]                                     -- v3
  , [6, 26]                                     -- v4
  , [6, 30]                                     -- v5
  , [6, 34]                                     -- v6
  , [6, 22, 38]                                 -- v7
  , [6, 24, 42]                                 -- v8
  , [6, 26, 46]                                 -- v9
  , [6, 28, 50]                                 -- v10
  , [6, 30, 54]                                 -- v11
  , [6, 32, 58]                                 -- v12
  , [6, 34, 62]                                 -- v13
  , [6, 26, 46, 66]                             -- v14
  , [6, 26, 48, 70]                             -- v15
  , [6, 26, 50, 74]                             -- v16
  , [6, 30, 54, 78]                             -- v17
  , [6, 30, 56, 82]                             -- v18
  , [6, 30, 58, 86]                             -- v19
  , [6, 34, 62, 90]                             -- v20
  , [6, 28, 50, 72, 94]                         -- v21
  , [6, 26, 50, 74, 98]                         -- v22
  , [6, 30, 54, 78, 102]                        -- v23
  , [6, 28, 54, 80, 106]                        -- v24
  , [6, 32, 58, 84, 110]                        -- v25
  , [6, 30, 58, 86, 114]                        -- v26
  , [6, 34, 62, 90, 118]                        -- v27
  , [6, 26, 50, 74, 98, 122]                    -- v28
  , [6, 30, 54, 78, 102, 126]                   -- v29
  , [6, 26, 52, 78, 104, 130]                   -- v30
  , [6, 30, 56, 82, 108, 134]                   -- v31
  , [6, 34, 60, 86, 112, 138]                   -- v32
  , [6, 30, 58, 86, 114, 142]                   -- v33
  , [6, 34, 62, 90, 118, 146]                   -- v34
  , [6, 30, 54, 78, 102, 126, 150]              -- v35
  , [6, 24, 50, 76, 102, 128, 154]              -- v36
  , [6, 28, 54, 80, 106, 132, 158]              -- v37
  , [6, 32, 58, 84, 110, 136, 162]              -- v38
  , [6, 26, 54, 82, 110, 138, 166]              -- v39
  , [6, 30, 58, 86, 114, 142, 170]              -- v40
  ]

-- ---------------------------------------------------------------------------
-- Grid geometry helpers
-- ---------------------------------------------------------------------------

-- | Symbol size in modules: (4V + 17) × (4V + 17).
--
-- @
--   Version 1 → 21×21
--   Version 7 → 45×45
--   Version 40 → 177×177
-- @
symbolSize :: Int -> Int
symbolSize version = 4 * version + 17

-- | Total number of raw data+ECC bits in a symbol, before removing structural
-- modules.
--
-- Formula derived from Nayuki's reference implementation (public domain).
-- It counts all non-structural module positions.
numRawDataModules :: Int -> Int
numRawDataModules version =
  let v       = version
      base    = (16 * v + 128) * v + 64
      withAlign = if version >= 2
                  then let numAlign = (v `div` 7) + 2
                           alignPenalty = (25 * numAlign - 10) * numAlign - 55
                       in  base - alignPenalty
                  else base
      withVer = if version >= 7
                then withAlign - 36
                else withAlign
  in  withVer

-- | How many data codewords (bytes) are available for user data at a given
-- version and ECC level.
--
-- Total raw codewords minus ECC codewords.
numDataCodewords :: Int -> EccLevel -> Int
numDataCodewords version ecc =
  let e       = eccIdx ecc
      rawCw   = numRawDataModules version `div` 8
      eccCw   = (numBlocks !! e !! version) * (eccCwPerBlock !! e !! version)
  in  rawCw - eccCw

-- | Remainder bits — if (numRawDataModules version) is not a multiple of 8,
-- these extra zero-bits are appended after interleaving.
numRemainderBits :: Int -> Int
numRemainderBits version = numRawDataModules version `mod` 8

-- ---------------------------------------------------------------------------
-- Reed-Solomon encoder (b=0 convention)
-- ---------------------------------------------------------------------------
--
-- QR uses the generator polynomial:
--   g(x) = ∏(x + αⁱ) for i in 0..n-1
-- where n is the number of ECC codewords.
--
-- This is the b=0 convention (first root is α^0 = 1).

-- | Build the monic RS generator polynomial of degree @n@.
--
-- Starts with g = [1] (degree 0, the constant 1), then multiplies by each
-- linear factor (x + αⁱ) for i in 0..n-1.
--
-- The output list has n+1 elements: @[g_n, g_{n-1}, ..., g_0]@ where
-- @g_n = 1@ (monic coefficient is always first).
--
-- Example for n=2: g = (x + α^0)(x + α^1) = (x + 1)(x + 2) = x^2 + 3x + 2
-- In GF(256): [1, gfMul 1 2, gfMul 1 2] = [1, 3, 2]
buildGenerator :: Int -> [Int]
buildGenerator n = foldl multiplyByLinear [1] [0 .. n - 1]
  where
    -- Multiply current polynomial g by the linear factor (x + α^i).
    --
    -- Coefficients are stored highest-degree first: g = [g_k, ..., g_1, g_0].
    --
    -- (g) * (x + ai) = g*x + g*ai
    --   g*x:  append a trailing 0 → [g_k, ..., g_0, 0]     (shift right in array)
    --   g*ai: prepend a leading 0 → [0, ai*g_k, ..., ai*g_0]
    --
    -- XOR the two to get the degree-(k+1) result.
    multiplyByLinear g i =
      let ai   = gfPow 2 i  -- α^i in GF(256) where α = 2
          gx   = g ++ [0]           -- g multiplied by x (trailing 0)
          gai  = 0 : map (gfMul ai) g  -- g multiplied by ai (leading 0)
      in  zipWith xor gx gai

-- | Compute @n@ ECC bytes using LFSR polynomial division.
--
-- Computes the remainder of @D(x) · x^n mod G(x)@, where @D(x)@ is the
-- data polynomial and @G(x)@ is the generator polynomial from 'buildGenerator'.
--
-- The LFSR algorithm processes one data byte at a time:
--
-- @
-- remainder = [0] * n
-- for each byte b in data:
--     feedback = b XOR remainder[0]
--     remainder = remainder[1:] ++ [0]
--     for i in 0..n-1:
--         remainder[i] ^= generator[i+1] * feedback
-- @
--
-- The generator polynomial has n+1 coefficients; we use indices 1..n
-- (the non-leading coefficients) for the LFSR feedback computation.
rsEncode :: [Int]   -- ^ Data bytes (as Int values 0..255)
         -> [Int]   -- ^ Generator polynomial (from 'buildGenerator')
         -> [Int]   -- ^ ECC bytes
rsEncode dataBytes generator =
  let n         = length generator - 1
      -- Gen coefficients at indices 1..n (skip the leading monic coefficient)
      genCoeffs = drop 1 generator
      initRem   = replicate n 0
      step remainder b =
        case remainder of
          []     -> []
          (r0:rs) ->
            let feedback = b `xor` r0
                rem'     = rs ++ [0]
            in  if feedback == 0
                then rem'
                else zipWith xor rem' (map (gfMul feedback) genCoeffs)
  in  foldl step initRem dataBytes

-- ---------------------------------------------------------------------------
-- Data encoding modes
-- ---------------------------------------------------------------------------

-- | The 45-character QR alphanumeric alphabet, in index order.
--
-- Characters 0-9 get indices 0-9, A-Z get 10-35, then:
-- space=36, $=37, %=38, *=39, +=40, -=41, .=42, /=43, :=44
alphanumChars :: String
alphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

-- | The three encoding modes the encoder supports.
--
-- Numeric encodes only 0-9 digits. Alphanumeric encodes digits, uppercase
-- letters, and 9 special characters. Byte encodes arbitrary UTF-8 bytes.
data EncodingMode = Numeric | Alphanumeric | Byte
  deriving (Show, Eq)

-- | Select the most compact mode that covers the entire input.
--
-- Priority order (most compact first):
-- 1. Numeric — if every character is 0-9
-- 2. Alphanumeric — if every character is in the 45-char QR set
-- 3. Byte — universal fallback, handles all UTF-8
selectMode :: String -> EncodingMode
selectMode input
  | all isDigit input               = Numeric
  | all (`elem` alphanumChars) input = Alphanumeric
  | otherwise                       = Byte

-- | 4-bit mode indicator, per ISO 18004 Table 2.
modeIndicator :: EncodingMode -> Int
modeIndicator Numeric      = 1   -- 0b0001
modeIndicator Alphanumeric = 2   -- 0b0010
modeIndicator Byte         = 4   -- 0b0100

-- | Character count indicator bit width, per ISO 18004 Table 3.
--
-- The width depends on both the mode and the version:
-- @
--                Versions 1-9  |  Versions 10-26  |  Versions 27-40
-- Numeric            10                12                  14
-- Alphanumeric        9                11                  13
-- Byte                8                16                  16
-- @
charCountBits :: EncodingMode -> Int -> Int
charCountBits Numeric      v | v <= 9    = 10
                              | v <= 26   = 12
                              | otherwise = 14
charCountBits Alphanumeric v | v <= 9    = 9
                              | v <= 26   = 11
                              | otherwise = 13
charCountBits Byte         v | v <= 9    = 8
                              | otherwise = 16

-- ---------------------------------------------------------------------------
-- Bit stream builder
-- ---------------------------------------------------------------------------
--
-- The QR encoder builds a bit stream, then converts to bytes. We represent
-- the stream as a list of Int values (each 0 or 1), then group into bytes.

-- | Append @count@ bits from @value@ (MSB first) to a bit list.
writeBits :: [Int]  -- ^ Existing bit list (accumulated so far)
          -> Int    -- ^ Value to write (MSB first)
          -> Int    -- ^ Number of bits to write
          -> [Int]
writeBits acc value count =
  acc ++ [ (value `shiftR` i) .&. 1 | i <- [count - 1, count - 2 .. 0] ]

-- | Convert a bit list to a byte list.
--
-- Groups bits into groups of 8. If the last group is short, it is zero-padded
-- on the right (which should not happen after proper terminator padding).
bitsToBytes :: [Int] -> [Int]
bitsToBytes bits =
  let padded = bits ++ replicate ((8 - length bits `mod` 8) `mod` 8) 0
      chunks = chunksOf 8 padded
  in  map toByteVal chunks
  where
    toByteVal = foldl (\acc b -> acc * 2 + b) 0
    chunksOf _ [] = []
    chunksOf k xs = let (a, b) = splitAt k xs in a : chunksOf k b

-- | Encode numeric mode data bits.
--
-- Groups of 3 digits → 10 bits, groups of 2 → 7 bits, single → 4 bits.
-- Example: "01234567" → groups "012"(10b), "345"(10b), "67"(7b) = 27 bits
encodeNumeric :: String -> [Int] -> [Int]
encodeNumeric input acc = go input acc
  where
    go []         a = a
    go [d]        a = writeBits a (ord d - ord '0') 4
    go [d1, d2]   a = writeBits a ((ord d1 - ord '0') * 10 + (ord d2 - ord '0')) 7
    go (d1:d2:d3:rest) a =
      let v = (ord d1 - ord '0') * 100 + (ord d2 - ord '0') * 10 + (ord d3 - ord '0')
      in  go rest (writeBits a v 10)

-- | Encode alphanumeric mode data bits.
--
-- Pairs of characters → 11 bits using formula: first*45 + second.
-- Single trailing character → 6 bits.
encodeAlphanumeric :: String -> [Int] -> [Int]
encodeAlphanumeric input acc = go input acc
  where
    idx c = case lookup c (zip alphanumChars [0 ..]) of
              Just i  -> i
              Nothing -> 0  -- should never happen: caller guarantees valid mode
    go []         a = a
    go [c]        a = writeBits a (idx c) 6
    go (c1:c2:rest) a =
      let v = idx c1 * 45 + idx c2
      in  go rest (writeBits a v 11)

-- | Encode byte mode data bits (raw UTF-8 bytes, 8 bits each).
encodeByteMode :: String -> [Int] -> [Int]
encodeByteMode input acc =
  foldl (\a b -> writeBits a b 8) acc (map ord input)

-- | Build the complete data codeword sequence for one data segment.
--
-- Assembles the bit stream:
-- 1. Mode indicator (4 bits)
-- 2. Character count (mode- and version-dependent width)
-- 3. Encoded data bits
-- 4. Terminator (up to 4 zero bits, fewer if at capacity)
-- 5. Zero padding to byte boundary
-- 6. Alternating 0xEC/0x11 fill bytes to reach capacity
buildDataCodewords :: String -> Int -> EccLevel -> [Int]
buildDataCodewords input version ecc =
  let mode     = selectMode input
      capacity = numDataCodewords version ecc
      charCount = if mode == Byte then length (concatMap utf8Bytes input)
                  else length input

      -- Step 1: mode indicator (4 bits)
      bits0 = writeBits [] (modeIndicator mode) 4

      -- Step 2: character count
      bits1 = writeBits bits0 charCount (charCountBits mode version)

      -- Step 3: encoded data
      bits2 = case mode of
                Numeric      -> encodeNumeric input bits1
                Alphanumeric -> encodeAlphanumeric input bits1
                Byte         -> encodeByteMode (encodeUtf8 input) bits1

      -- Step 4: terminator (up to 4 zero bits)
      avail   = capacity * 8
      termLen = min 4 (avail - length bits2)
      bits3   = if termLen > 0 then writeBits bits2 0 termLen else bits2

      -- Step 5: zero-pad to byte boundary
      rem8  = length bits3 `mod` 8
      bits4 = if rem8 /= 0 then writeBits bits3 0 (8 - rem8) else bits3

      -- Step 6: convert to bytes and fill with 0xEC/0x11
      bytes = bitsToBytes bits4
      pads  = cycle [0xEC, 0x11]
      filled = take capacity (bytes ++ pads)
  in  filled

-- | Encode a Haskell String to its UTF-8 bytes.
--
-- Haskell's String is a list of Unicode code points. QR byte mode requires
-- the raw UTF-8 byte sequence. Most modern scanners interpret QR byte-mode
-- data as UTF-8.
encodeUtf8 :: String -> String
encodeUtf8 s = concatMap encodeChar s
  where
    encodeChar c
      | code < 0x80  = [toEnum code]
      | code < 0x800 =
          [ toEnum (0xC0 .|. (code `shiftR` 6))
          , toEnum (0x80 .|. (code .&. 0x3F))
          ]
      | code < 0x10000 =
          [ toEnum (0xE0 .|. (code `shiftR` 12))
          , toEnum (0x80 .|. ((code `shiftR` 6) .&. 0x3F))
          , toEnum (0x80 .|. (code .&. 0x3F))
          ]
      | otherwise =
          [ toEnum (0xF0 .|. (code `shiftR` 18))
          , toEnum (0x80 .|. ((code `shiftR` 12) .&. 0x3F))
          , toEnum (0x80 .|. ((code `shiftR` 6) .&. 0x3F))
          , toEnum (0x80 .|. (code .&. 0x3F))
          ]
      where code = ord c

-- | Get the UTF-8 byte list for a character (for character counting).
utf8Bytes :: Char -> [Int]
utf8Bytes c = map ord (encodeChar c)
  where
    encodeChar ch
      | code < 0x80    = [toEnum code]
      | code < 0x800   = [ toEnum (0xC0 .|. (code `shiftR` 6))
                          , toEnum (0x80 .|. (code .&. 0x3F))]
      | code < 0x10000 = [ toEnum (0xE0 .|. (code `shiftR` 12))
                          , toEnum (0x80 .|. ((code `shiftR` 6) .&. 0x3F))
                          , toEnum (0x80 .|. (code .&. 0x3F))]
      | otherwise      = [ toEnum (0xF0 .|. (code `shiftR` 18))
                          , toEnum (0x80 .|. ((code `shiftR` 12) .&. 0x3F))
                          , toEnum (0x80 .|. ((code `shiftR` 6) .&. 0x3F))
                          , toEnum (0x80 .|. (code .&. 0x3F))]
      where code = ord ch

-- ---------------------------------------------------------------------------
-- Block structure and interleaving
-- ---------------------------------------------------------------------------
--
-- For resilience against burst errors, data codewords are split across
-- multiple "blocks". Each block gets its own Reed-Solomon ECC computation.
--
-- The block structure uses two "groups": group 1 blocks are slightly shorter
-- than group 2 blocks (by 1 codeword) when the total doesn't divide evenly.

-- | One data block with its RS ECC bytes.
data Block = Block
  { blockData :: [Int]  -- ^ Data codewords
  , blockEcc  :: [Int]  -- ^ ECC codewords computed by Reed-Solomon
  }

-- | Split data codewords into blocks and compute RS ECC for each.
computeBlocks :: [Int] -> Int -> EccLevel -> [Block]
computeBlocks dataBytes version ecc =
  let e          = eccIdx ecc
      totalBlocks = numBlocks !! e !! version
      eccLen      = eccCwPerBlock !! e !! version
      totalData   = numDataCodewords version ecc
      shortLen    = totalData `div` totalBlocks
      numLong     = totalData `mod` totalBlocks  -- blocks with shortLen+1 data cw
      gen         = buildGenerator eccLen
      g1Count     = totalBlocks - numLong
      -- Split into g1Count short blocks, then numLong long blocks
      g1Total     = g1Count * shortLen
      g1Blocks    = map (\i -> take shortLen (drop (i * shortLen) dataBytes)) [0 .. g1Count - 1]
      g2Blocks    = map (\i -> take (shortLen + 1) (drop (g1Total + i * (shortLen + 1)) dataBytes))
                         [0 .. numLong - 1]
      allBlocks   = g1Blocks ++ g2Blocks
  in  map (\d -> Block { blockData = d, blockEcc = rsEncode d gen }) allBlocks

-- | Interleave blocks to form the final codeword sequence.
--
-- Interleaving spreads adjacent data/ECC bytes across different blocks,
-- so burst damage only destroys a portion of each block rather than
-- wiping out an entire block.
--
-- Algorithm:
-- 1. Take codeword index 0 from each block in sequence
-- 2. Then codeword index 1 from each block
-- ...continuing until all data codewords are emitted...
-- Then repeat for ECC codewords.
interleaveBlocks :: [Block] -> [Int]
interleaveBlocks blocks =
  let maxData = maximum (map (length . blockData) blocks)
      maxEcc  = maximum (map (length . blockEcc)  blocks)
      dataPart = concatMap (\i -> concatMap (\b -> take 1 (drop i (blockData b))) blocks)
                           [0 .. maxData - 1]
      eccPart  = concatMap (\i -> concatMap (\b -> take 1 (drop i (blockEcc b))) blocks)
                           [0 .. maxEcc - 1]
  in  dataPart ++ eccPart

-- ---------------------------------------------------------------------------
-- Working grid (mutable-style using arrays via Data.Vector)
-- ---------------------------------------------------------------------------
--
-- The grid has two parallel boolean arrays: 'modules' holds the actual dark/light
-- values, and 'reserved' marks structural (non-data) modules that must not be
-- overwritten during zigzag placement or masking.

-- | A working grid used during encoding.
--
-- We represent it as a pair of flat Vectors (row-major) for efficient random
-- access. Index = row * size + col.
data WorkGrid = WorkGrid
  { wgSize     :: Int
  , wgModules  :: V.Vector Bool  -- ^ True = dark
  , wgReserved :: V.Vector Bool  -- ^ True = structural (not data)
  }

-- | Create an empty (all-light, all-unreserved) working grid.
newWorkGrid :: Int -> WorkGrid
newWorkGrid size = WorkGrid
  { wgSize     = size
  , wgModules  = V.replicate (size * size) False
  , wgReserved = V.replicate (size * size) False
  }

-- | Compute the flat index for (row, col).
wgIdx :: WorkGrid -> Int -> Int -> Int
wgIdx g r c = r * wgSize g + c

-- | Set a module's dark/light value and optionally mark it as reserved.
wgSet :: WorkGrid -> Int -> Int -> Bool -> Bool -> WorkGrid
wgSet g r c dark resv =
  let i    = wgIdx g r c
      mods = wgModules  g V.// [(i, dark)]
      resvd = if resv
              then wgReserved g V.// [(i, True)]
              else wgReserved g
  in  g { wgModules = mods, wgReserved = resvd }

-- | Get whether the module at (row, col) is dark.
wgGet :: WorkGrid -> Int -> Int -> Bool
wgGet g r c = wgModules g V.! wgIdx g r c

-- | Check if the module at (row, col) is reserved.
wgIsReserved :: WorkGrid -> Int -> Int -> Bool
wgIsReserved g r c = wgReserved g V.! wgIdx g r c

-- ---------------------------------------------------------------------------
-- Structural pattern placement
-- ---------------------------------------------------------------------------

-- | Place a 7×7 finder pattern at (top, left).
--
-- The finder pattern is the most recognizable element of a QR code — the three
-- identical 7×7 square-ring patterns at three corners. A scanner detects the
-- distinctive 1:1:3:1:1 dark-light ratio to locate and orient the symbol.
--
-- Pattern (row 0=top, 0=light, 1=dark):
-- @
-- 1 1 1 1 1 1 1
-- 1 0 0 0 0 0 1
-- 1 0 1 1 1 0 1
-- 1 0 1 1 1 0 1
-- 1 0 1 1 1 0 1
-- 1 0 0 0 0 0 1
-- 1 1 1 1 1 1 1
-- @
placeFinder :: WorkGrid -> Int -> Int -> WorkGrid
placeFinder g top left =
  foldl setCell g [(dr, dc) | dr <- [0..6], dc <- [0..6]]
  where
    setCell acc (dr, dc) =
      let onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6
          inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
          dark     = onBorder || inCore
      in  wgSet acc (top + dr) (left + dc) dark True

-- | Place a 5×5 alignment pattern centered at (row, col).
--
-- Alignment patterns help scanners correct for perspective distortion in larger
-- symbols. Each is a 5×5 version of the finder pattern (outer ring + dark center):
--
-- @
-- 1 1 1 1 1
-- 1 0 0 0 1
-- 1 0 1 0 1
-- 1 0 0 0 1
-- 1 1 1 1 1
-- @
placeAlignment :: WorkGrid -> Int -> Int -> WorkGrid
placeAlignment g row col =
  foldl setCell g [(dr, dc) | dr <- [-2..2], dc <- [-2..2]]
  where
    setCell acc (dr, dc) =
      let onBorder = abs dr == 2 || abs dc == 2
          isCenter = dr == 0 && dc == 0
          dark     = onBorder || isCenter
      in  wgSet acc (row + dr) (col + dc) dark True

-- | Place all alignment patterns for a given version.
--
-- For each pair (r, c) from the version's alignment position list, place an
-- alignment pattern — unless that position overlaps a finder pattern (which
-- is already reserved).
placeAllAlignments :: WorkGrid -> Int -> WorkGrid
placeAllAlignments g version =
  let positions = alignmentPositions !! (version - 1)
      pairs     = [(r, c) | r <- positions, c <- positions]
  in  foldl placePair g pairs
  where
    placePair acc (r, c) =
      if wgIsReserved acc r c
      then acc
      else placeAlignment acc r c

-- | Place the horizontal and vertical timing strips.
--
-- Two alternating dark/light strips running between the finder patterns:
-- - Horizontal: row 6, columns 8 to (size-9)
-- - Vertical:   col 6, rows 8 to (size-9)
-- Both start and end dark (even-indexed positions are dark).
placeTiming :: WorkGrid -> WorkGrid
placeTiming g =
  let sz   = wgSize g
      hCols = [8 .. sz - 9]
      vRows = [8 .. sz - 9]
      setH acc c = wgSet acc 6 c (even c) True
      setV acc r = wgSet acc r 6 (even r) True
  in  foldl setV (foldl setH g hCols) vRows

-- | Reserve the format information modules without writing actual data.
--
-- Format info occupies two L-shaped strips. We reserve these positions so
-- the zigzag data placer skips them. Actual format data is written after
-- mask selection.
reserveFormatInfo :: WorkGrid -> WorkGrid
reserveFormatInfo g =
  let sz   = wgSize g
      -- Copy 1: row 8 (cols 0-5, 7-8) and col 8 (rows 0-8 excluding timing)
      -- Copy 2: row 8 (cols sz-8..sz-1) and col 8 (rows sz-7..sz-1)
      r8c   = [0..5] ++ [7, 8]          -- row 8, left side
      c8r   = [0..5] ++ [7, 8]          -- col 8, top side
      r8r   = [(8, c) | c <- r8c]
      c8top = [(r, 8) | r <- c8r]
      copy2c = [(8, c) | c <- [sz - 8 .. sz - 1]]
      copy2r = [(r, 8) | r <- [sz - 7 .. sz - 1]]
      allPos = r8r ++ c8top ++ copy2c ++ copy2r
      -- Filter: timing row/col at r=6 or c=6 are already reserved as timing
      markReserved acc (r, c) = g { wgReserved = wgReserved acc V.// [(wgIdx acc r c, True)] }
  in  foldl markReserved g allPos

-- | Reserve the version information modules (versions 7+).
--
-- The 18-bit version string is placed in two 6×3 blocks:
-- - Near top-right finder: rows 0-5, cols size-11 to size-9
-- - Near bottom-left finder: rows size-11 to size-9, cols 0-5
reserveVersionInfo :: WorkGrid -> Int -> WorkGrid
reserveVersionInfo g version
  | version < 7 = g
  | otherwise   =
      let sz  = wgSize g
          pos = [ (r, sz - 11 + dc) | r <- [0..5], dc <- [0..2] ] ++
                [ (sz - 11 + dr, c) | dr <- [0..2], c <- [0..5] ]
          markR acc (r, c) = g { wgReserved = wgReserved acc V.// [(wgIdx acc r c, True)] }
      in  foldl markR g pos

-- | Place the always-dark module at position (4V+9, 8).
--
-- The dark module is always set to dark (True). It is not part of data and
-- is never masked. Its purpose is to ensure at least one dark module is present
-- adjacent to the bottom-left format information strip.
placeDarkModule :: WorkGrid -> Int -> WorkGrid
placeDarkModule g version = wgSet g (4 * version + 9) 8 True True

-- ---------------------------------------------------------------------------
-- Zigzag data placement
-- ---------------------------------------------------------------------------
--
-- The interleaved message stream is placed into non-reserved modules using a
-- two-column zigzag scan starting from the bottom-right corner:
--
--   current_col = size-1    (rightmost column)
--   direction = upward (-1)
--   for each 2-column strip:
--     for row in current direction:
--       for sub_col in [col, col-1]:
--         if sub_col == 6: skip (timing column)
--         if reserved: skip
--         place next bit
--     flip direction, move 2 columns left

-- | Place the message bit stream into the grid using the zigzag scan.
placeBits :: WorkGrid -> [Int] -> Int -> WorkGrid
placeBits g codewords version =
  let sz   = wgSize g
      -- Expand codewords to individual bits (MSB first for each byte)
      cwBits = concatMap (\b -> [ (b `shiftR` i) .&. 1 | i <- [7, 6 .. 0]]) codewords
      -- Append remainder bits (zero-padding)
      allBits = cwBits ++ replicate (numRemainderBits version) 0

      -- Process each 2-column strip
      process startCol goingUp bitIdx curG
        | startCol < 0 = curG
        | otherwise    =
            let colPair = [startCol, startCol - 1]
                rows    = if goingUp then [sz-1, sz-2 .. 0] else [0 .. sz-1]
                positions = [ (r, c) | r <- rows, c <- colPair
                             , c /= 6
                             , not (wgIsReserved curG r c) ]
                (newG, newIdx) = foldl placeBit (curG, bitIdx) positions
                nextCol = let nc = startCol - 2
                          in  if nc == 6 then nc - 1 else nc
            in  process nextCol (not goingUp) newIdx newG

      placeBit (acc, idx) (r, c) =
        let dark = idx < length allBits && (allBits !! idx == 1)
        in  (wgSet acc r c dark False, idx + 1)

  in  process (sz - 1) True 0 g

-- ---------------------------------------------------------------------------
-- Build the initial grid (before data placement)
-- ---------------------------------------------------------------------------

-- | Build the initial working grid with all structural elements placed.
--
-- Sequence:
-- 1. Three finder patterns at the three corners
-- 2. Separators (1-module light borders around finders)
-- 3. Timing strips
-- 4. Alignment patterns (version ≥ 2)
-- 5. Reserve format information positions
-- 6. Reserve version information positions (version ≥ 7)
-- 7. Dark module
buildGrid :: Int -> WorkGrid
buildGrid version =
  let sz = symbolSize version
      g0 = newWorkGrid sz

      -- Three finder patterns
      g1 = placeFinder (placeFinder (placeFinder g0 0 0) 0 (sz - 7)) (sz - 7) 0

      -- Separators: 1-module-wide light border around each finder pattern.
      -- These isolate the finder patterns from the data area to prevent
      -- false positive detection in the data region.
      setSep acc (r, c) = wgSet acc r c False True
      sepTL = [(7, c) | c <- [0..7]] ++ [(r, 7) | r <- [0..7]]
      sepTR = [(7, c) | c <- [sz-8..sz-1]] ++ [(r, sz-8) | r <- [0..7]]
      sepBL = [(sz-8, c) | c <- [0..7]] ++ [(r, 7) | r <- [sz-8..sz-1]]
      g2 = foldl setSep g1 (sepTL ++ sepTR ++ sepBL)

      -- Timing strips
      g3 = placeTiming g2

      -- Alignment patterns
      g4 = placeAllAlignments g3 version

      -- Format information reservation
      g5 = reserveFormatInfo g4

      -- Version information reservation
      g6 = reserveVersionInfo g5 version

      -- Dark module
      g7 = placeDarkModule g6 version

  in  g7

-- ---------------------------------------------------------------------------
-- Format information
-- ---------------------------------------------------------------------------

-- | Compute the 15-bit format information word.
--
-- Algorithm (ISO 18004 Annex C):
-- 1. 5-bit data = [ecc_indicator (2 bits)] [mask_pattern (3 bits)]
-- 2. Left-shift by 10 to make room for BCH remainder
-- 3. Polynomial division by G(x) = 0x537 to get 10-bit remainder
-- 4. XOR with 0x5412 (ensures format info is never all-zeros)
--
-- The BCH generator polynomial is:
-- @G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1 (0x537 = 10011100111 binary)@
computeFormatBits :: EccLevel -> Int -> Int
computeFormatBits ecc maskIdx =
  let eccBits  = eccIndicator ecc
      data5    = (eccBits `shiftL` 3) .|. maskIdx
      shifted  = data5 `shiftL` 10
      -- BCH division: generator polynomial 0x537
      rem'     = foldl divStep shifted [14, 13 .. 10]
      divStep r i = if (r `shiftR` i) .&. 1 == 1
                    then r `xor` (0x537 `shiftL` (i - 10))
                    else r
      result   = ((data5 `shiftL` 10) .|. (rem' .&. 0x3FF)) `xor` 0x5412
  in  result

-- | Write the 15-bit format information into its two copies in the grid.
--
-- Copy 1: adjacent to the top-left finder pattern (L-shaped strip)
--   - Row 8, cols 0-5 (f14 down to f9, MSB first)
--   - Row 8, col 7 (f8)
--   - Row 8, col 8 (f7)
--   - Col 8, row 7 (f6)
--   - Col 8, rows 0-5 (f5 down to f0, LSB at row 0)
--
-- Copy 2: along the top-right and bottom-left finders
--   - Row 8, cols size-1 down to size-8 (f0 to f7)
--   - Col 8, rows size-7 up to size-1 (f8 to f14)
writeFormatInfo :: WorkGrid -> Int -> WorkGrid
writeFormatInfo g fmt =
  let sz  = wgSize g
      -- Copy 1 positions (bit index, row, col) where bit = (fmt >> (14-i)) & 1
      copy1 =
        [ (14 - i, 8, i)   | i <- [0..5] ] ++  -- row 8, cols 0-5 → f14..f9
        [ (8,      8, 7)   ] ++                  -- row 8, col 7 → f8
        [ (7,      8, 8)   ] ++                  -- row 8, col 8 → f7
        [ (6,      7, 8)   ] ++                  -- col 8, row 7 → f6
        [ (i,      i, 8)   | i <- [0..5] ]       -- col 8, rows 0-5 → f5..f0

      -- Copy 2 positions
      copy2 =
        [ (i, 8, sz - 1 - i)    | i <- [0..7]  ] ++  -- row 8, right to left → f0..f7
        [ (i, sz - 15 + i, 8)   | i <- [8..14] ]     -- col 8, bottom-left → f8..f14

      setBit acc (bit, r, c) =
        let dark = (fmt `shiftR` bit) .&. 1 == 1
        in  g { wgModules = wgModules acc V.// [(wgIdx acc r c, dark)] }

  in  foldl setBit g (copy1 ++ copy2)

-- ---------------------------------------------------------------------------
-- Version information
-- ---------------------------------------------------------------------------

-- | Compute the 18-bit version information word (versions 7+).
--
-- Algorithm (ISO 18004 Annex D):
-- 1. 6-bit version number, left-shifted 12
-- 2. BCH division by G(x) = 0x1F25 to get 12-bit remainder
-- 3. Concatenate version bits + remainder
--
-- @G(x) = x^12 + x^11 + x^10 + x^9 + x^8 + x^5 + x^2 + 1 = 0x1F25@
computeVersionBits :: Int -> Int
computeVersionBits version =
  let v     = version
      shifted = v `shiftL` 12
      rem'    = foldl divStep shifted [17, 16 .. 12]
      divStep r i = if (r `shiftR` i) .&. 1 == 1
                    then r `xor` (0x1F25 `shiftL` (i - 12))
                    else r
  in  (v `shiftL` 12) .|. (rem' .&. 0xFFF)

-- | Write version information into the two 6×3 blocks (versions 7+).
--
-- The 18 bits are numbered 0..17. Each bit i goes into:
-- - (5 - i/3, size-9 - i%3)  — near top-right finder
-- - (size-9 - i%3, 5 - i/3)  — near bottom-left finder (transpose)
writeVersionInfo :: WorkGrid -> Int -> WorkGrid
writeVersionInfo g version
  | version < 7 = g
  | otherwise   =
      let sz   = wgSize g
          bits = computeVersionBits version
          positions = [ (i, 5 - (i `div` 3), sz - 9 - (i `mod` 3)) | i <- [0..17] ]
          setBit acc (i, r, c) =
            let dark = (bits `shiftR` i) .&. 1 == 1
                g1   = g { wgModules = wgModules acc V.// [(wgIdx acc r c, dark)] }
                -- Transposed copy
                g2   = g1 { wgModules = wgModules g1 V.// [(wgIdx g1 c r, dark)] }
            in  g2
      in  foldl setBit g positions

-- ---------------------------------------------------------------------------
-- Masking
-- ---------------------------------------------------------------------------
--
-- Masking prevents degenerate patterns in the data area that could confuse
-- scanners. Each of the 8 mask patterns defines a condition; if the condition
-- is true for a non-reserved module, that module is flipped.

-- | Evaluate the mask condition for pattern @m@ at position @(row, col)@.
--
-- The 8 patterns cover different mathematical relationships between row and
-- column, ensuring no single type of degenerate pattern can dominate.
maskCondition :: Int -> Int -> Int -> Bool
maskCondition m r c = case m of
  0 -> (r + c) `mod` 2 == 0
  1 -> r `mod` 2 == 0
  2 -> c `mod` 3 == 0
  3 -> (r + c) `mod` 3 == 0
  4 -> (r `div` 2 + c `div` 3) `mod` 2 == 0
  5 -> (r * c) `mod` 2 + (r * c) `mod` 3 == 0
  6 -> ((r * c) `mod` 2 + (r * c) `mod` 3) `mod` 2 == 0
  7 -> ((r + c) `mod` 2 + (r * c) `mod` 3) `mod` 2 == 0
  _ -> False

-- | Apply mask pattern @m@ to all non-reserved modules.
--
-- Returns a new module vector with appropriate bits flipped.
applyMask :: V.Vector Bool -> V.Vector Bool -> Int -> Int -> V.Vector Bool
applyMask modules reserved sz m =
  V.imap (\i val ->
    let r = i `div` sz
        c = i `mod` sz
    in  if reserved V.! i
        then val
        else val /= maskCondition m r c  -- XOR via (/=) for Bool
  ) modules

-- ---------------------------------------------------------------------------
-- Penalty scoring
-- ---------------------------------------------------------------------------
--
-- Four penalty rules determine the "badness" of a masked symbol. We evaluate
-- all 8 masks and pick the one with the lowest total penalty.

-- | Compute the penalty score for a given module grid.
--
-- Rules:
-- 1. Runs of ≥5 same-color modules in a row/column: score += run_length - 2
-- 2. 2×2 same-color blocks: score += 3
-- 3. Finder-pattern-like sequences: score += 40 each
-- 4. Dark-module proportion deviation from 50%: score += (deviation/5)*10
computePenalty :: V.Vector Bool -> Int -> Int
computePenalty modules sz =
  penalty1 + penalty2 + penalty3 + penalty4
  where
    getM r c = modules V.! (r * sz + c)

    -- Rule 1: runs of ≥5 same color in rows and columns
    penalty1 = sum (map scoreRow [0..sz-1]) + sum (map scoreCol [0..sz-1])

    scoreSeq vals =
      let runs = groupRuns vals
      in  sum [len - 2 | len <- runs, len >= 5]

    groupRuns [] = []
    groupRuns (x:xs) =
      let (same, rest) = span (== x) xs
      in  (1 + length same) : groupRuns rest

    scoreRow r = scoreSeq [getM r c | c <- [0..sz-1]]
    scoreCol c = scoreSeq [getM r c | r <- [0..sz-1]]

    -- Rule 2: 2×2 same-color blocks
    penalty2 = 3 * length
      [ ()
      | r <- [0..sz-2], c <- [0..sz-2]
      , let d = getM r c
      , d == getM r (c+1) && d == getM (r+1) c && d == getM (r+1) (c+1)
      ]

    -- Rule 3: finder-pattern-like sequences (1-0-1-1-1-0-1-0-0-0-0 or reverse)
    pat1, pat2 :: [Bool]
    pat1 = map (== 1) [1,0,1,1,1,0,1,0,0,0,0 :: Int]
    pat2 = map (== 1) [0,0,0,0,1,0,1,1,1,0,1 :: Int]

    penalty3 = 40 * count3

    count3 = length [ ()
      | a <- [0..sz-1]
      , b <- [0..sz-12]   -- need 11 elements
      , let rowSeq = [getM a (b+k) | k <- [0..10]]
            colSeq = [getM (b+k) a | k <- [0..10]]
      , rowSeq == pat1 || rowSeq == pat2
        || colSeq == pat1 || colSeq == pat2
      ]

    -- Rule 4: proportion of dark modules
    dark   = V.length (V.filter id modules)
    total  = sz * sz
    ratio  = fromIntegral dark / fromIntegral total * 100.0 :: Double
    prev5  = (floor (ratio / 5.0) :: Int) * 5
    aVal   = abs (prev5 - 50)
    bVal   = abs (prev5 + 5 - 50)
    penalty4 = (min aVal bVal `div` 5) * 10

-- ---------------------------------------------------------------------------
-- Version selection
-- ---------------------------------------------------------------------------

-- | Select the minimum QR version that fits the input at the given ECC level.
--
-- Tries versions 1 through 40 in order. For each version, computes the number
-- of bits required and compares against the available data capacity.
--
-- Returns an error if even version 40 cannot hold the input.
selectVersion :: String -> EccLevel -> Either QRCodeError Int
selectVersion input ecc =
  let mode     = selectMode input
      byteInput = encodeUtf8 input
      byteLen  = length byteInput

      -- Compute bit count required for this mode
      dataBits = case mode of
        Byte ->
          fromIntegral byteLen * 8
        Numeric ->
          let n = length input
          in  (n * 10 + 2) `div` 3  -- ceil(n*10/3)
        Alphanumeric ->
          let n = length input
          in  (n * 11 + 1) `div` 2  -- ceil(n*11/2)

      tryVersion v =
        let capacity  = numDataCodewords v ecc
            ccBits    = charCountBits mode v
            bitsNeeded = 4 + ccBits + dataBits
            cwNeeded  = (bitsNeeded + 7) `div` 8
        in  cwNeeded <= capacity

  in  case filter tryVersion [1..40] of
        (v:_) -> Right v
        []    -> Left (InputTooLong
                  ("Input (" ++ show byteLen ++
                   " bytes, ECC=" ++ show ecc ++
                   ") exceeds version-40 capacity."))

-- ---------------------------------------------------------------------------
-- Main encode function
-- ---------------------------------------------------------------------------

-- | Encode a UTF-8 string into a QR Code 'ModuleGrid'.
--
-- Returns a @(4V+17) × (4V+17)@ boolean grid where 'True' = dark module.
-- Automatically selects the minimum version that fits the input at the
-- given ECC level.
--
-- == Example
--
-- @
-- case encode "HELLO WORLD" M of
--   Left err   -> putStrLn ("Error: " ++ show err)
--   Right grid -> print (mgRows grid)  -- 21 (version 1)
-- @
--
-- == Error
--
-- Returns 'InputTooLong' if the input exceeds version-40 capacity.
encode :: String -> EccLevel -> Either QRCodeError ModuleGrid
encode input ecc = do
  version <- selectVersion input ecc

  let sz         = symbolSize version
      -- Build data codewords and blocks
      dataCws    = buildDataCodewords input version ecc
      blocks     = computeBlocks dataCws version ecc
      interleaved = interleaveBlocks blocks

      -- Build structural grid
      g0         = buildGrid version

      -- Place data bits
      g1         = placeBits g0 interleaved version

      -- Evaluate all 8 mask patterns, pick lowest penalty
      (bestMask, _bestPenalty) = minimumBy (comparing snd) masksWithPenalties

      masksWithPenalties = map evalMask [0..7]
      evalMask m =
        let masked  = applyMask (wgModules g1) (wgReserved g1) sz m
            fmt     = computeFormatBits ecc m
            gWithFmt = writeFormatInfo g1 { wgModules = masked } fmt
        in  (m, computePenalty (wgModules gWithFmt) sz)

      -- Apply best mask and write final format + version info
      finalMods  = applyMask (wgModules g1) (wgReserved g1) sz bestMask
      gFinal0    = writeFormatInfo g1 { wgModules = finalMods }
                     (computeFormatBits ecc bestMask)
      gFinal     = writeVersionInfo gFinal0 version

      -- Convert working grid to ModuleGrid
      mgrid = foldr (\(r, c) acc -> setModule acc r c (wgGet gFinal r c))
                    (emptyGrid sz sz Square)
                    [(r, c) | r <- [0..sz-1], c <- [0..sz-1]]

  return mgrid

-- ---------------------------------------------------------------------------
-- encode + layout convenience
-- ---------------------------------------------------------------------------

-- | Encode and convert to a pixel-resolved 'PaintScene'.
--
-- Convenience function combining 'encode' and 'barcode-2d's 'layout'.
encodeAndLayout
  :: String
  -> EccLevel
  -> Barcode2DLayoutConfig
  -> Either QRCodeError PaintScene
encodeAndLayout input ecc cfg = do
  grid <- encode input ecc
  return (layout grid cfg)

-- | Placeholder: render to SVG string using barcode-2d.
--
-- Full SVG rendering requires the paint-vm-svg backend. This function
-- returns a placeholder message showing what would be rendered.
-- Connect to a paint-vm-svg backend for actual SVG output.
renderSvg :: String -> EccLevel -> Barcode2DLayoutConfig -> Either QRCodeError String
renderSvg input ecc cfg = do
  _scene <- encodeAndLayout input ecc cfg
  return "<svg><!-- QR Code rendered via paint-vm-svg backend --></svg>"
