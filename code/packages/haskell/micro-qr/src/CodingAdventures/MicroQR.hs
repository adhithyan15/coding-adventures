-- | CodingAdventures.MicroQR — ISO\/IEC 18004:2015 Annex E Micro QR Code encoder.
--
-- == What is Micro QR Code?
--
-- Micro QR Code is the compact variant of QR Code, designed for applications
-- where even the smallest standard QR (21×21 at version 1) is too large.
-- Common use cases include surface-mount component labels, circuit board
-- markings, and miniature industrial tags.
--
-- == Symbol sizes
--
-- @
--   M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
--   formula: size = 2 × version_number + 9
-- @
--
-- == Key differences from regular QR Code
--
-- * __Single finder pattern__ at top-left only (one 7×7 square, not three).
-- * __Timing at row 0 / col 0__ (not row 6 / col 6).
-- * __Only 4 mask patterns__ (not 8).
-- * __Format XOR mask 0x4445__ (not 0x5412).
-- * __Single copy of format info__ (not two).
-- * __2-module quiet zone__ (not 4).
-- * __Narrower mode indicators__ (0–3 bits instead of 4).
-- * __Single block__ (no interleaving).
--
-- == Encoding pipeline
--
-- @
-- input string
--   → auto-select smallest symbol (M1..M4) and mode
--   → build bit stream (mode indicator + char count + data + terminator + padding)
--   → Reed-Solomon ECC (GF(256)\/0x11D, b=0, single block)
--   → initialize grid (finder, L-shaped separator, timing at row0\/col0,
--     format reserved)
--   → zigzag data placement (two-column snake from bottom-right)
--   → evaluate 4 mask patterns, pick lowest penalty
--   → write format information (15 bits, single copy, XOR 0x4445)
--   → ModuleGrid
-- @
module CodingAdventures.MicroQR
  ( -- * Public encode functions
    encode
  , encodeAt
  , encodeAndLayout

    -- * Symbol version constants
  , MicroQRVersion (..)

    -- * ECC level constants
  , MicroQREccLevel (..)

    -- * Error types
  , MicroQRError (..)

    -- * Re-exported grid type
  , ModuleGrid (..)

    -- * Version string
  , version
  ) where

import Data.Bits (xor, shiftR, shiftL, (.&.), (.|.))
import Data.Char (ord, isDigit)
import Data.List (foldl', minimumBy)
import Data.Ord  (comparing)
import qualified Data.Vector as V

import CodingAdventures.Barcode2D
  ( ModuleGrid (..)
  , ModuleShape (..)
  , Barcode2DLayoutConfig (..)
  , emptyGrid
  , setModule
  , layout
  )

-- ---------------------------------------------------------------------------
-- Version
-- ---------------------------------------------------------------------------

-- | Library version string.
version :: String
version = "0.1.0"

-- ---------------------------------------------------------------------------
-- MicroQRVersion
-- ---------------------------------------------------------------------------

-- | Micro QR symbol designator.
--
-- Each step up adds two rows and columns (size = 2 × version_number + 9):
-- M1 = 11×11, M2 = 13×13, M3 = 15×15, M4 = 17×17.
data MicroQRVersion
  = M1  -- ^ 11×11, numeric only, detection ECC
  | M2  -- ^ 13×13, numeric + alphanumeric
  | M3  -- ^ 15×15, numeric + alphanumeric + byte
  | M4  -- ^ 17×17, all modes, supports Q ECC
  deriving (Show, Eq, Ord)

-- ---------------------------------------------------------------------------
-- MicroQREccLevel
-- ---------------------------------------------------------------------------

-- | Error correction level for Micro QR.
--
-- +------------+--------------+-----------------------------------+
-- | Level      | Available in | Recovery capability               |
-- +============+==============+===================================+
-- | Detection  | M1 only      | Detects errors only               |
-- | L          | M2, M3, M4   | ~7% of codewords recoverable      |
-- | M          | M2, M3, M4   | ~15% of codewords recoverable     |
-- | Q          | M4 only      | ~25% of codewords recoverable     |
-- +------------+--------------+-----------------------------------+
--
-- Level H is not available in Micro QR — the symbols are too small.
data MicroQREccLevel
  = Detection  -- ^ Error detection only; M1 only
  | L          -- ^ Low ECC (~7%); M2–M4
  | M          -- ^ Medium ECC (~15%); M2–M4
  | Q          -- ^ Quartile ECC (~25%); M4 only
  deriving (Show, Eq, Ord)

-- ---------------------------------------------------------------------------
-- Error types
-- ---------------------------------------------------------------------------

-- | All errors the Micro QR encoder can raise.
data MicroQRError
  = InputTooLong String
    -- ^ Input is too long to fit in any M1–M4 symbol at any ECC level.
    --   Maximum capacity is 35 numeric characters in M4-L.
  | ECCNotAvailable String
    -- ^ The requested ECC level is not available for the chosen symbol.
    --   For example, Q is only available in M4; H is not available at all.
  | UnsupportedMode String
    -- ^ The requested encoding mode is not available for the chosen symbol.
    --   For example, byte mode requires M3 or M4.
  | InvalidConfiguration String
    -- ^ The (version, ecc) combination is not valid.
  deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- Symbol configuration table
-- ---------------------------------------------------------------------------
--
-- All 8 valid (version, ECC) combinations with their compile-time constants.
-- Listed in ascending size order so the first config that fits wins in
-- auto-selection.

-- | One (version, ECC) configuration with all compile-time constants.
data SymbolConfig = SymbolConfig
  { scVersion           :: !MicroQRVersion
  , scEcc               :: !MicroQREccLevel
  , scSymbolIndicator   :: !Int   -- ^ 3-bit value in format information
  , scSize              :: !Int   -- ^ Symbol side length (11, 13, 15, 17)
  , scDataCw            :: !Int   -- ^ Number of data codewords
  , scEccCw             :: !Int   -- ^ Number of ECC codewords
  , scNumericCap        :: !Int   -- ^ Max numeric chars (0 = unsupported)
  , scAlphaCap          :: !Int   -- ^ Max alphanumeric chars (0 = unsupported)
  , scByteCap           :: !Int   -- ^ Max byte chars (0 = unsupported)
  , scTerminatorBits    :: !Int   -- ^ Length of terminator in bits
  , scModeIndicatorBits :: !Int   -- ^ Width of mode indicator field in bits
  , scCcBitsNumeric     :: !Int   -- ^ Char count field width for numeric mode
  , scCcBitsAlpha       :: !Int   -- ^ Char count field width for alpha mode (0=unsupported)
  , scCcBitsByte        :: !Int   -- ^ Char count field width for byte mode (0=unsupported)
  , scM1HalfCw          :: !Bool  -- ^ True only for M1: last data codeword is 4 bits
  } deriving (Show)

-- | The 8 valid symbol configurations from ISO 18004:2015 Annex E.
--
-- Listed in ascending size/capacity order for correct auto-selection behavior:
-- M1/Detection → M2/L → M2/M → M3/L → M3/M → M4/L → M4/M → M4/Q.
symbolConfigs :: [SymbolConfig]
symbolConfigs =
  -- M1 / Detection — 11×11, numeric only, error detection.
  -- M1 is the smallest Micro QR. Its unusual 20-bit data capacity
  -- (3 codewords where the last is only 4 bits) is a consequence of
  -- limited grid space.
  [ SymbolConfig M1 Detection 0 11  3  2  5  0  0  3 0  3 0 0 True
  -- M2 / L — 13×13, adds alphanumeric.
  , SymbolConfig M2 L         1 13  5  5 10  6  4  5 1  4 3 4 False
  -- M2 / M — 13×13, more ECC, less data.
  , SymbolConfig M2 M         2 13  4  6  8  5  3  5 1  4 3 4 False
  -- M3 / L — 15×15, adds byte mode.
  , SymbolConfig M3 L         3 15 11  6 23 14  9  7 2  5 4 4 False
  -- M3 / M — 15×15, more ECC.
  , SymbolConfig M3 M         4 15  9  8 18 11  7  7 2  5 4 4 False
  -- M4 / L — 17×17, largest, all modes.
  , SymbolConfig M4 L         5 17 16  8 35 21 15  9 3  6 5 5 False
  -- M4 / M — 17×17, more ECC.
  , SymbolConfig M4 M         6 17 14 10 30 18 13  9 3  6 5 5 False
  -- M4 / Q — 17×17, highest ECC (M4 only).
  , SymbolConfig M4 Q         7 17 10 14 21 13  9  9 3  6 5 5 False
  ]

-- ---------------------------------------------------------------------------
-- Pre-computed format information table
-- ---------------------------------------------------------------------------
--
-- All 32 format words (8 symbol_indicators × 4 mask patterns), pre-computed
-- and XOR-masked with 0x4445 as required by Micro QR.
--
-- Format word structure (15 bits):
--   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
-- then XOR with 0x4445 (Micro QR-specific, NOT 0x5412 like regular QR).
--
-- The XOR ensures a Micro QR symbol cannot be confused with a regular QR
-- symbol by a scanner — the format bits look distinct.
--
-- Indexed as formatTable !! symbol_indicator !! mask_pattern.

-- | Pre-computed 15-bit format words, indexed [symbol_indicator][mask_pattern].
formatTable :: [[Int]]
formatTable =
  [ [0x4445, 0x4172, 0x4E2B, 0x4B1C]  -- M1 / Detection (symbol_indicator=0)
  , [0x5528, 0x501F, 0x5F46, 0x5A71]  -- M2-L (symbol_indicator=1)
  , [0x6649, 0x637E, 0x6C27, 0x6910]  -- M2-M (symbol_indicator=2)
  , [0x7764, 0x7253, 0x7D0A, 0x783D]  -- M3-L (symbol_indicator=3)
  , [0x06DE, 0x03E9, 0x0CB0, 0x0987]  -- M3-M (symbol_indicator=4)
  , [0x17F3, 0x12C4, 0x1D9D, 0x18AA]  -- M4-L (symbol_indicator=5)
  , [0x24B2, 0x2185, 0x2EDC, 0x2BEB]  -- M4-M (symbol_indicator=6)
  , [0x359F, 0x30A8, 0x3FF1, 0x3AC6]  -- M4-Q (symbol_indicator=7)
  ]

-- ---------------------------------------------------------------------------
-- Encoding mode constants
-- ---------------------------------------------------------------------------

-- | The 45-character set used by alphanumeric mode.
-- Same as regular QR Code: digits, uppercase A–Z, and 9 special chars.
alphanumChars :: String
alphanumChars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

data EncodingMode = ModeNumeric | ModeAlphanumeric | ModeByte
  deriving (Show, Eq)

-- | Select the most compact encoding mode for the input and symbol config.
--
-- Priority: numeric > alphanumeric > byte.
-- Returns Nothing if no mode is available.
selectMode :: String -> SymbolConfig -> Maybe EncodingMode
selectMode input cfg
  | isNumericOnly && scCcBitsNumeric cfg > 0  = Just ModeNumeric
  | isAlphaOnly   && scCcBitsAlpha cfg > 0   = Just ModeAlphanumeric
  | scCcBitsByte cfg > 0                      = Just ModeByte
  | otherwise                                 = Nothing
  where
    isNumericOnly = all isDigit input || null input
    isAlphaOnly   = all (`elem` alphanumChars) input

-- | Return the mode indicator value for (mode, config).
--
-- Micro QR uses narrower mode indicators than regular QR:
-- M1: 0 bits (implicit numeric — only one mode available)
-- M2: 1 bit  (0=numeric, 1=alphanumeric)
-- M3: 2 bits (00=numeric, 01=alphanumeric, 10=byte)
-- M4: 3 bits (000=numeric, 001=alphanumeric, 010=byte)
modeIndicatorValue :: EncodingMode -> SymbolConfig -> Int
modeIndicatorValue mode cfg =
  case scModeIndicatorBits cfg of
    0 -> 0  -- M1: no indicator
    1 -> case mode of { ModeNumeric -> 0; _ -> 1 }
    2 -> case mode of { ModeNumeric -> 0; ModeAlphanumeric -> 1; ModeByte -> 2 }
    _ -> case mode of { ModeNumeric -> 0; ModeAlphanumeric -> 1; ModeByte -> 2 }

-- | Return the char count field width for a given mode and config.
charCountBitsFor :: EncodingMode -> SymbolConfig -> Int
charCountBitsFor ModeNumeric      cfg = scCcBitsNumeric cfg
charCountBitsFor ModeAlphanumeric cfg = scCcBitsAlpha cfg
charCountBitsFor ModeByte         cfg = scCcBitsByte cfg

-- ---------------------------------------------------------------------------
-- GF(256)/0x11D — shared with regular QR Code
-- ---------------------------------------------------------------------------
--
-- Micro QR uses GF(256) with primitive polynomial 0x11D:
--   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
--
-- This is the SAME polynomial as regular QR Code — the RS encoder is shared.
--
-- We build exp/log tables at module load time for O(1) multiplication.

-- | Build (exp, log) tables for GF(256) over 0x11D.
buildQrTables :: (V.Vector Int, V.Vector Int)
buildQrTables = (expVec, logVec)
  where
    go :: [Int] -> Int -> Int -> [Int]
    go acc val 255 = acc ++ [val]
    go acc val i =
      let val' = if val `shiftL` 1 >= 0x100
                 then (val `shiftL` 1) `xor` 0x11D
                 else val `shiftL` 1
      in  go (acc ++ [val]) val' (i + 1)

    expList = go [] 1 0
    expVec  = V.fromList (take 256 (expList ++ [0]))
    logVec  = V.replicate 256 0 V.//
                [ (expList !! i, i) | i <- [0 .. 254] ]

qrExp :: V.Vector Int
qrExp = fst buildQrTables

qrLog :: V.Vector Int
qrLog = snd buildQrTables

-- | Multiply two GF(256)/0x11D field elements.
qrMul :: Int -> Int -> Int
qrMul a b
  | a == 0 || b == 0 = 0
  | otherwise        = qrExp V.! ((qrLog V.! a + qrLog V.! b) `mod` 255)

-- ---------------------------------------------------------------------------
-- RS encoder (b=0 convention)
-- ---------------------------------------------------------------------------
--
-- Micro QR uses the b=0 convention: roots are α^0, α^1, …, α^{n-1}.
-- g(x) = (x + α^0)(x + α^1) ··· (x + α^{n-1}).
-- This is identical to regular QR Code's RS encoder.

-- | Pre-computed monic RS generator polynomials for b=0, GF(256)/0x11D.
--
-- Only the ECC codeword counts used in Micro QR are included:
-- {2, 5, 6, 8, 10, 14}.
rsGenerators :: [(Int, [Int])]
rsGenerators =
  -- Coefficients highest-degree first (monic), length = ecc_cw + 1.
  [ (2,  [0x01, 0x03, 0x02])
  , (5,  [0x01, 0x1F, 0xF6, 0x44, 0xD9, 0x68])
  , (6,  [0x01, 0x3F, 0x4E, 0x17, 0x9B, 0x05, 0x37])
  , (8,  [0x01, 0x63, 0x0D, 0x60, 0x6D, 0x5B, 0x10, 0xA2, 0xA3])
  , (10, [0x01, 0xF6, 0x75, 0xA8, 0xD0, 0xC3, 0xE3, 0x36, 0xE1, 0x3C, 0x45])
  , (14, [0x01, 0xF6, 0x9A, 0x60, 0x97, 0x8A, 0xF1, 0xA4, 0xA1, 0x8E,
          0xFC, 0x7A, 0x52, 0xAD, 0xAC])
  ]

-- | Compute n ECC bytes using LFSR polynomial division (b=0 convention).
--
-- Computes the remainder of D(x)·x^n mod G(x) over GF(256)/0x11D.
rsEncode :: [Int]  -- ^ Data bytes
         -> Int    -- ^ Number of ECC bytes
         -> [Int]  -- ^ ECC bytes
rsEncode dataBytes n =
  let gen = case lookup n rsGenerators of
              Just g  -> g
              Nothing -> error ("MicroQR.rsEncode: no generator for n=" ++ show n)
      genCoeffs = drop 1 gen  -- skip leading monic 1
      initRem   = replicate n 0
      step rem b =
        case rem of
          []     -> []
          (r0:rs) ->
            let feedback = b `xor` r0
                rem'     = rs ++ [0]
            in  if feedback == 0
                then rem'
                else zipWith xor rem' (map (qrMul feedback) genCoeffs)
  in  foldl' step initRem dataBytes

-- ---------------------------------------------------------------------------
-- Bit stream builder
-- ---------------------------------------------------------------------------

-- | Append count bits from value (MSB first) to a bit list.
writeBits :: [Int]  -- ^ Existing bit list
          -> Int    -- ^ Value to write (MSB first)
          -> Int    -- ^ Number of bits to write
          -> [Int]
writeBits acc value count =
  acc ++ [ (value `shiftR` i) .&. 1 | i <- [count - 1, count - 2 .. 0] ]

-- | Convert a bit list to a byte list (zero-padded to byte boundary).
bitsToBytes :: [Int] -> [Int]
bitsToBytes bits =
  let rem8  = length bits `mod` 8
      padded = bits ++ replicate (if rem8 == 0 then 0 else 8 - rem8) 0
      go []  = []
      go bs  = let (chunk, rest) = splitAt 8 bs
               in  foldl (\acc b -> acc * 2 + b) 0 chunk : go rest
  in  go padded

-- ---------------------------------------------------------------------------
-- Encoding helpers
-- ---------------------------------------------------------------------------

-- | Encode digits in numeric mode.
-- Groups of 3 → 10 bits, pairs → 7 bits, singles → 4 bits.
encodeNumeric :: String -> [Int] -> [Int]
encodeNumeric input acc = go input acc
  where
    go []         a = a
    go [d]        a = writeBits a (ord d - ord '0') 4
    go [d1, d2]   a = writeBits a ((ord d1 - ord '0') * 10 + (ord d2 - ord '0')) 7
    go (d1:d2:d3:rest) a =
      let v = (ord d1 - ord '0') * 100 + (ord d2 - ord '0') * 10 + (ord d3 - ord '0')
      in  go rest (writeBits a v 10)

-- | Encode in alphanumeric mode.
-- Pairs → 11 bits (first*45 + second), singles → 6 bits.
encodeAlphanumeric :: String -> [Int] -> [Int]
encodeAlphanumeric input acc = go input acc
  where
    idx c = case lookup c (zip alphanumChars [0..]) of
              Just i  -> i
              Nothing -> 0
    go []         a = a
    go [c]        a = writeBits a (idx c) 6
    go (c1:c2:rest) a = go rest (writeBits a (idx c1 * 45 + idx c2) 11)

-- | Encode in byte mode (raw UTF-8 bytes, 8 bits each).
encodeByte :: String -> [Int] -> [Int]
encodeByte input acc =
  foldl' (\a b -> writeBits a b 8) acc (map ord input)

-- ---------------------------------------------------------------------------
-- Data codeword assembly
-- ---------------------------------------------------------------------------

-- | Build the complete data codeword byte sequence.
--
-- For all symbols except M1:
--   [mode indicator] [char count] [data bits]
--   [terminator] [byte-align] [0xEC/0x11 fill]
--   → exactly cfg.data_cw bytes.
--
-- For M1 (m1HalfCw = True):
--   Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
--   The RS encoder receives 3 bytes where byte[2] has data in the upper
--   4 bits and forced zeros in the lower 4 bits.
--
-- Terminator: up to terminatorBits zero bits, truncated if capacity exhausted.
-- Padding:    alternating 0xEC, 0x11 fill bytes to reach capacity.
buildDataCodewords :: String -> SymbolConfig -> EncodingMode -> [Int]
buildDataCodewords input cfg mode =
  let totalBits = if scM1HalfCw cfg
                  then scDataCw cfg * 8 - 4  -- M1: 20 bits total
                  else scDataCw cfg * 8

      -- Mode indicator (0..3 bits depending on symbol version).
      bits0 = if scModeIndicatorBits cfg > 0
              then writeBits [] (modeIndicatorValue mode cfg) (scModeIndicatorBits cfg)
              else []

      -- Character count.
      charCount = case mode of
                    ModeByte -> length (encodeUtf8 input)  -- count UTF-8 bytes
                    _        -> length input
      bits1 = writeBits bits0 charCount (charCountBitsFor mode cfg)

      -- Encoded data bits.
      bits2 = case mode of
                ModeNumeric      -> encodeNumeric input bits1
                ModeAlphanumeric -> encodeAlphanumeric input bits1
                ModeByte         -> encodeByte (encodeUtf8 input) bits1

      -- Terminator: up to terminatorBits zero bits.
      remaining = totalBits - length bits2
      termLen   = min (scTerminatorBits cfg) (max 0 remaining)
      bits3     = writeBits bits2 0 termLen

  in  if scM1HalfCw cfg
      then buildM1Codewords bits3
      else buildStandardCodewords bits3 cfg

-- | Build M1 codewords from a bit stream.
-- M1 uses 20 data bits: byte0 (8 bits), byte1 (8 bits), nibble_byte (4 bits upper, 4 zero lower).
buildM1Codewords :: [Int] -> [Int]
buildM1Codewords bits =
  let padded = take 20 (bits ++ replicate 20 0)
      toInt bs = foldl' (\acc b -> acc * 2 + b) 0 bs
      b0 = toInt (take 8 padded)
      b1 = toInt (take 8 (drop 8 padded))
      -- Upper nibble = data bits 16-19; lower nibble = forced zeros.
      b2 = toInt (take 4 (drop 16 padded)) `shiftL` 4
  in  [b0, b1, b2]

-- | Build standard (non-M1) codewords from a bit stream.
buildStandardCodewords :: [Int] -> SymbolConfig -> [Int]
buildStandardCodewords bits cfg =
  let -- Pad to byte boundary with zero bits.
      rem8   = length bits `mod` 8
      padded = if rem8 == 0 then bits else bits ++ replicate (8 - rem8) 0
      cws    = bitsToBytes padded
      -- Fill remaining with alternating 0xEC / 0x11.
      fillers = cycle [0xEC, 0x11]
      filled  = take (scDataCw cfg) (cws ++ fillers)
  in  filled

-- | Encode a Haskell String to its UTF-8 byte sequence (as a String of bytes).
--
-- Each Haskell Char is a Unicode code point.  UTF-8 maps code points to
-- sequences of 1–4 bytes:
--
-- @
-- U+0000–U+007F    →  1 byte:  0xxxxxxx
-- U+0080–U+07FF   →  2 bytes: 110xxxxx 10xxxxxx
-- U+0800–U+FFFF   →  3 bytes: 1110xxxx 10xxxxxx 10xxxxxx
-- U+10000–U+10FFFF → 4 bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
-- @
encodeUtf8 :: String -> String
encodeUtf8 = concatMap encodeChar
  where
    encodeChar c
      | code < 0x80    = [toEnum code]
      | code < 0x800   = [ toEnum (0xC0 .|. (code `shiftR` 6))
                          , toEnum (0x80 .|. (code .&. 0x3F)) ]
      | code < 0x10000 = [ toEnum (0xE0 .|. (code `shiftR` 12))
                          , toEnum (0x80 .|. ((code `shiftR` 6) .&. 0x3F))
                          , toEnum (0x80 .|. (code .&. 0x3F)) ]
      | otherwise      = [ toEnum (0xF0 .|. (code `shiftR` 18))
                          , toEnum (0x80 .|. ((code `shiftR` 12) .&. 0x3F))
                          , toEnum (0x80 .|. ((code `shiftR` 6)  .&. 0x3F))
                          , toEnum (0x80 .|. (code .&. 0x3F)) ]
      where code = ord c

-- ---------------------------------------------------------------------------
-- Working grid type
-- ---------------------------------------------------------------------------

-- | Mutable-style working grid with a parallel reservation map.
data WorkGrid = WorkGrid
  { wgSize     :: !Int
  , wgModules  :: V.Vector Bool  -- ^ True = dark
  , wgReserved :: V.Vector Bool  -- ^ True = structural
  }

-- | Create an empty (all-light, all-unreserved) working grid.
newWorkGrid :: Int -> WorkGrid
newWorkGrid sz = WorkGrid
  { wgSize     = sz
  , wgModules  = V.replicate (sz * sz) False
  , wgReserved = V.replicate (sz * sz) False
  }

-- | Flat index for (row, col).
wgIdx :: WorkGrid -> Int -> Int -> Int
wgIdx g r c = r * wgSize g + c

-- | Set a module value and optionally mark it as reserved.
wgSet :: WorkGrid -> Int -> Int -> Bool -> Bool -> WorkGrid
wgSet g r c dark resv =
  let i    = wgIdx g r c
      mods = wgModules g V.// [(i, dark)]
      resvd = if resv then wgReserved g V.// [(i, True)] else wgReserved g
  in  g { wgModules = mods, wgReserved = resvd }

-- | Get whether a module is reserved.
wgIsReserved :: WorkGrid -> Int -> Int -> Bool
wgIsReserved g r c = wgReserved g V.! wgIdx g r c

-- ---------------------------------------------------------------------------
-- Structural pattern placement
-- ---------------------------------------------------------------------------

-- | Place the 7×7 finder pattern at the top-left corner.
--
-- In Micro QR there is only ONE finder pattern (top-left), not three.
-- Pattern (1=dark, 0=light):
--
-- @
--   1 1 1 1 1 1 1
--   1 0 0 0 0 0 1
--   1 0 1 1 1 0 1
--   1 0 1 1 1 0 1
--   1 0 1 1 1 0 1
--   1 0 0 0 0 0 1
--   1 1 1 1 1 1 1
-- @
placeFinder :: WorkGrid -> WorkGrid
placeFinder g = foldl' setCell g
  [(dr, dc) | dr <- [0..6], dc <- [0..6]]
  where
    setCell acc (dr, dc) =
      let onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6
          inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
      in  wgSet acc dr dc (onBorder || inCore) True

-- | Place the L-shaped separator around the finder pattern.
--
-- Unlike regular QR (which surrounds all three finders), Micro QR only
-- needs separation on the BOTTOM (row 7, cols 0–7) and RIGHT (col 7, rows 0–7)
-- sides of its single finder. All separator modules are always light.
placeSeparator :: WorkGrid -> WorkGrid
placeSeparator g = foldl' (\acc i ->
    wgSet (wgSet acc 7 i False True) i 7 False True)
  g [0..7]

-- | Place timing pattern extensions along row 0 and col 0.
--
-- Timing patterns alternate dark/light starting with dark at position 0.
-- In Micro QR, timing runs along the OUTER edges (row 0 and col 0),
-- unlike regular QR where timing is at row 6 and col 6.
--
-- Positions 0–6 are already covered by the finder pattern.
-- Position 7 is the separator (always light, already reserved).
-- We place timing starting at position 8 outward.
placeTiming :: WorkGrid -> WorkGrid
placeTiming g =
  let sz   = wgSize g
      setH acc c = wgSet acc 0 c (c `mod` 2 == 0) True
      setV acc r = wgSet acc r 0 (r `mod` 2 == 0) True
  in  foldl' setV (foldl' setH g [8..sz-1]) [8..sz-1]

-- | Reserve the 15 format information module positions (placeholder: all light).
--
-- Format information occupies an L-shaped strip adjacent to the separator:
-- - Row 8, cols 1–8:  8 modules (bits f14 down to f7, MSB first)
-- - Col 8, rows 1–7:  7 modules (bits f6 down to f0)
--
-- Reserved now, written with actual format bits AFTER mask selection.
reserveFormatInfo :: WorkGrid -> WorkGrid
reserveFormatInfo g =
  let setRow acc c = wgSet acc 8 c False True
      setCol acc r = wgSet acc r 8 False True
  in  foldl' setCol (foldl' setRow g [1..8]) [1..7]

-- | Build the initial structural grid.
buildEmptyGrid :: SymbolConfig -> WorkGrid
buildEmptyGrid cfg =
  let g0 = newWorkGrid (scSize cfg)
      g1 = placeFinder g0
      g2 = placeSeparator g1
      g3 = placeTiming g2
      g4 = reserveFormatInfo g3
  in  g4

-- ---------------------------------------------------------------------------
-- Bit-to-bit expansion
-- ---------------------------------------------------------------------------

-- | Expand a codeword list to individual bits (MSB first per codeword).
--
-- M1 special case: the last data codeword contributes only 4 bits
-- (the upper nibble — the lower nibble is forced zero and NOT placed).
-- All ECC codewords contribute 8 bits even in M1.
cwsToBits :: [Int]  -- ^ Data codewords
          -> [Int]  -- ^ ECC codewords
          -> Bool   -- ^ True if M1 (last data codeword is 4-bit nibble)
          -> Int    -- ^ Number of data codewords
          -> [Bool]
cwsToBits dataCws eccCws isM1 numDataCws =
  let allCws = dataCws ++ eccCws
      bitsFor cwIdx cw =
        let isHalf = isM1 && cwIdx == numDataCws - 1
            n      = if isHalf then 4 else 8
            start  = 8 - n  -- = 4 for half-codeword, 0 for full
        in  [ ((cw `shiftR` (b + start)) .&. 1 == 1)
            | b <- [n-1, n-2 .. 0] ]
  in  concatMap (uncurry bitsFor) (zip [0..] allCws)

-- ---------------------------------------------------------------------------
-- Data placement — two-column zigzag
-- ---------------------------------------------------------------------------

-- | Place data and ECC bits into the grid via two-column zigzag.
--
-- Scans from the bottom-right corner, moving two columns left at a time,
-- alternating upward and downward sweeps.  Reserved modules are skipped.
--
-- Unlike regular QR Code, there is NO timing column at col 6 to hop over.
-- Micro QR's timing is at col 0, which is always reserved and skipped.
placeBits :: WorkGrid -> [Bool] -> WorkGrid
placeBits g bits =
  let sz      = wgSize g
      go acc col goingUp bitIdx
        | col < 1   = acc
        | otherwise =
            let rows = if goingUp then [sz-1, sz-2 .. 0] else [0..sz-1]
                (acc', bitIdx') = foldl' placeCell (acc, bitIdx) rows
            in  go acc' (col - 2) (not goingUp) bitIdx'
            where
              placeCell (a, idx) row =
                let process (ac, i) dc =
                      let c = col - dc
                      in  if wgIsReserved ac row c
                          then (ac, i)
                          else
                            let dark = i < length bits && bits !! i
                                ac'  = wgSet ac row c dark False
                            in  (ac', i + 1)
                in  foldl' process (a, idx) [0, 1]
  in  go g (sz - 1) True 0

-- ---------------------------------------------------------------------------
-- Masking
-- ---------------------------------------------------------------------------

-- | Test whether mask pattern m applies to module (row, col).
--
-- Micro QR uses only 4 mask patterns (vs. 8 in regular QR):
--
-- +-------+---------------------------------+
-- | Index | Condition (flip if true)        |
-- +=======+=================================+
-- | 0     | (row + col) mod 2 == 0          |
-- | 1     | row mod 2 == 0                  |
-- | 2     | col mod 3 == 0                  |
-- | 3     | (row + col) mod 3 == 0          |
-- +-------+---------------------------------+
maskCondition :: Int -> Int -> Int -> Bool
maskCondition m r c = case m of
  0 -> (r + c) `mod` 2 == 0
  1 -> r `mod` 2 == 0
  2 -> c `mod` 3 == 0
  _ -> (r + c) `mod` 3 == 0

-- | Apply mask pattern m to all non-reserved modules. Returns a new module vector.
applyMask :: V.Vector Bool -> V.Vector Bool -> Int -> Int -> V.Vector Bool
applyMask modules reserved sz m =
  V.imap (\i val ->
    let r = i `div` sz
        c = i `mod` sz
    in  if reserved V.! i
        then val
        else val /= maskCondition m r c
  ) modules

-- ---------------------------------------------------------------------------
-- Format information placement
-- ---------------------------------------------------------------------------

-- | Write a 15-bit format word into the format information positions.
--
-- Placement (MSB at row 8, col 1):
-- - Row 8, cols 1–8: bits f14 (MSB) down to f7
-- - Col 8, rows 7 down to 1: bits f6 down to f0 (LSB)
--
-- Micro QR has only ONE copy of the format information (unlike regular QR
-- which places two copies). This simplifies placement but means there is no
-- redundancy if format modules are damaged.
writeFormatInfo :: V.Vector Bool -> Int -> Int -> V.Vector Bool
writeFormatInfo mods sz fmt =
  let -- Row 8, cols 1–8: bits f14 down to f7 (8 bits, MSB first)
      rowBits = [ (8 * sz + (1 + i), (fmt `shiftR` (14 - i)) .&. 1 == 1)
                | i <- [0..7] ]
      -- Col 8, rows 7 down to 1: bits f6 down to f0 (7 bits)
      colBits = [ ((7 - i) * sz + 8, (fmt `shiftR` (6 - i)) .&. 1 == 1)
                | i <- [0..6] ]
  in  mods V.// (rowBits ++ colBits)

-- ---------------------------------------------------------------------------
-- Penalty scoring
-- ---------------------------------------------------------------------------

-- | Compute the 4-rule penalty score for a masked module grid.
--
-- Four rules (same as regular QR Code):
--
-- Rule 1 — Adjacent run penalty: runs of ≥5 same-colour modules → score += run - 2.
-- Rule 2 — 2×2 block penalty: each 2×2 same-colour block → score += 3.
-- Rule 3 — Finder-pattern-like sequences: each match → score += 40.
-- Rule 4 — Dark proportion deviation from 50%: scored in steps of 5%.
computePenalty :: V.Vector Bool -> Int -> Int
computePenalty mods sz =
  penalty1 + penalty2 + penalty3 + penalty4
  where
    getM r c = mods V.! (r * sz + c)

    -- Rule 1: runs of ≥5 same colour in rows and columns.
    penalty1 = sum (map scoreSeq allSeqs)
      where
        allSeqs = [ [getM r c | c <- [0..sz-1]] | r <- [0..sz-1] ]
               ++ [ [getM r c | r <- [0..sz-1]] | c <- [0..sz-1] ]
        scoreSeq [] = 0
        scoreSeq (x:xs) = go 1 x xs
          where
            go run _ []     = if run >= 5 then run - 2 else 0
            go run prev (y:ys)
              | y == prev  = go (run + 1) prev ys
              | otherwise  = (if run >= 5 then run - 2 else 0) + go 1 y ys

    -- Rule 2: 2×2 same-colour blocks.
    penalty2 = 3 * length
      [ ()
      | r <- [0..sz-2], c <- [0..sz-2]
      , let d = getM r c
      , d == getM r (c+1) && d == getM (r+1) c && d == getM (r+1) (c+1)
      ]

    -- Rule 3: finder-pattern-like sequences.
    pat1, pat2 :: [Bool]
    pat1 = map (== 1) [1,0,1,1,1,0,1,0,0,0,0 :: Int]
    pat2 = map (== 1) [0,0,0,0,1,0,1,1,1,0,1 :: Int]

    penalty3
      | sz < 11   = 0
      | otherwise = 40 * count
        where
          count = length
            [ ()
            | a <- [0..sz-1]
            , b <- [0..sz-12]
            , let rowSeq = [getM a (b+k) | k <- [0..10]]
                  colSeq = [getM (b+k) a | k <- [0..10]]
            , rowSeq == pat1 || rowSeq == pat2
              || colSeq == pat1 || colSeq == pat2
            ]

    -- Rule 4: dark proportion deviation from 50%.
    dark   = V.length (V.filter id mods)
    total  = sz * sz
    ratio  = fromIntegral dark / fromIntegral total * 100.0 :: Double
    prev5  = (floor (ratio / 5.0) :: Int) * 5
    aVal   = abs (prev5 - 50)
    bVal   = abs (prev5 + 5 - 50)
    penalty4 = (min aVal bVal `div` 5) * 10

-- ---------------------------------------------------------------------------
-- Symbol configuration selector
-- ---------------------------------------------------------------------------

-- | Find the smallest symbol configuration that can hold the input.
--
-- Iterates symbolConfigs in order (M1 → M4) and returns the first config
-- where:
-- 1. The version matches (if specified).
-- 2. The ECC level matches (if specified).
-- 3. A supported encoding mode exists for the input.
-- 4. The input length does not exceed the mode capacity.
selectConfig :: String
             -> Maybe MicroQRVersion
             -> Maybe MicroQREccLevel
             -> Either MicroQRError SymbolConfig
selectConfig input mVer mEcc =
  let candidates = filter (\cfg ->
                     (case mVer of Nothing -> True; Just v -> scVersion cfg == v) &&
                     (case mEcc of Nothing -> True; Just e -> scEcc cfg == e))
                   symbolConfigs
  in  if null candidates
      then Left $ InvalidConfiguration
             ("No Micro QR symbol supports the requested version/ECC combination.")
      else case filter (\cfg -> fitsConfig input cfg) candidates of
             (cfg:_) -> Right cfg
             []      -> Left $ InputTooLong
                          ("Input (length " ++ show (length input) ++
                           ") does not fit in any Micro QR symbol. " ++
                           "Maximum is 35 numeric characters in M4-L.")

-- | Check if the input fits in a given config.
fitsConfig :: String -> SymbolConfig -> Bool
fitsConfig input cfg =
  case selectMode input cfg of
    Nothing   -> False
    Just mode ->
      let inputLen = case mode of
                       ModeByte -> length (encodeUtf8 input)
                       _        -> length input
          cap = case mode of
                  ModeNumeric      -> scNumericCap cfg
                  ModeAlphanumeric -> scAlphaCap cfg
                  ModeByte         -> scByteCap cfg
      in  cap > 0 && inputLen <= cap

-- ---------------------------------------------------------------------------
-- Core encode function
-- ---------------------------------------------------------------------------

-- | Encode a string to a Micro QR Code 'ModuleGrid'.
--
-- Automatically selects the smallest symbol (M1..M4) and ECC level that
-- can hold the input. Pass version and ecc to override.
--
-- Returns @Right ModuleGrid@ on success, @Left MicroQRError@ on failure.
--
-- == Full pipeline
--
-- 1. Select the smallest (version, ECC) configuration.
-- 2. Select the most compact encoding mode (numeric > alphanumeric > byte).
-- 3. Build the data codeword byte sequence.
-- 4. Compute Reed-Solomon ECC using GF(256)\/0x11D.
-- 5. Flatten to a bit stream (MSB-first per codeword).
-- 6. Initialize grid with finder, separator, timing, and reserved format info.
-- 7. Place bits via two-column zigzag from bottom-right.
-- 8. Evaluate all 4 mask patterns, compute penalty for each.
-- 9. Apply the best mask (lowest penalty, ties broken by lower index).
-- 10. Write the 15-bit format information into the reserved positions.
-- 11. Return the final immutable 'ModuleGrid'.
--
-- == Example
--
-- @
-- case encode "1" Nothing Nothing of
--   Right grid -> print (mgRows grid)  -- 11 (M1 = 11×11)
--   Left err   -> print err
-- @
encode :: String
       -> Maybe MicroQRVersion  -- ^ Force a specific version (Nothing = auto)
       -> Maybe MicroQREccLevel -- ^ Force a specific ECC level (Nothing = auto)
       -> Either MicroQRError ModuleGrid
encode input mVer mEcc = do
  cfg  <- selectConfig input mVer mEcc
  mode <- case selectMode input cfg of
            Just m  -> Right m
            Nothing -> Left $ UnsupportedMode
                         ("No encoding mode available for input in " ++
                          show (scVersion cfg) ++ "/" ++ show (scEcc cfg))

  let sz = scSize cfg

  -- Step 3: Build data codewords.
  let dataCws = buildDataCodewords input cfg mode

  -- Step 4: Compute RS ECC.
  let eccCws = rsEncode dataCws (scEccCw cfg)

  -- Step 5: Flatten to bit stream.
  let bits = cwsToBits dataCws eccCws (scM1HalfCw cfg) (scDataCw cfg)

  -- Step 6: Initialize structural grid.
  let wg0 = buildEmptyGrid cfg

  -- Step 7: Place data bits.
  let wg1 = placeBits wg0 bits

  -- Steps 8–9: Evaluate 4 mask patterns, pick best (lowest penalty).
  let masksWithPenalties = map evalMask [0..3]
      evalMask m =
        let masked = applyMask (wgModules wg1) (wgReserved wg1) sz m
            fmt    = formatTable !! scSymbolIndicator cfg !! m
            final  = writeFormatInfo masked sz fmt
        in  (m, computePenalty final sz)

  let (bestMask, _) = minimumBy (comparing snd) masksWithPenalties

  -- Step 10: Apply best mask and write final format info.
  let finalMods0 = applyMask (wgModules wg1) (wgReserved wg1) sz bestMask
      finalFmt   = formatTable !! scSymbolIndicator cfg !! bestMask
      finalMods  = writeFormatInfo finalMods0 sz finalFmt

  -- Step 11: Build immutable ModuleGrid.
  let mgrid = foldl' (\g i ->
                  if finalMods V.! i
                  then let r = i `div` sz
                           c = i `mod` sz
                       in  setModule g r c True
                  else g)
                (emptyGrid sz sz CodingAdventures.Barcode2D.Square)
                [0 .. sz * sz - 1]

  return mgrid

-- | Encode to a specific version and ECC level.
--
-- Equivalent to @encode input (Just version) (Just ecc)@.
encodeAt :: String
         -> MicroQRVersion
         -> MicroQREccLevel
         -> Either MicroQRError ModuleGrid
encodeAt input ver ecc = encode input (Just ver) (Just ecc)

-- | Encode and convert to a @PaintScene@ in one call.
--
-- The concrete return type is @Either MicroQRError PaintScene@ where
-- @PaintScene@ is from the @paint-instructions@ package (transitive dep via
-- @barcode-2d@).  No explicit type signature is given here to avoid requiring
-- a direct @paint-instructions@ import; GHC infers the type from 'layout'.
encodeAndLayout input mVer mEcc cfg = do
  grid <- encode input mVer mEcc
  return (layout grid cfg)
