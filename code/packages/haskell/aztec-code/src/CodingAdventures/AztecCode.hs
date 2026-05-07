-- | CodingAdventures.AztecCode — ISO\/IEC 24778:2008 Aztec Code encoder.
--
-- == What is Aztec Code?
--
-- Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
-- published as a patent-free 2D barcode format. Unlike QR Code (which uses
-- three square finder patterns at three corners), Aztec Code places a single
-- __bullseye finder pattern at the centre__ of the symbol. A scanner finds the
-- centre first, then reads outward in a clockwise spiral — no large quiet zone
-- is needed.
--
-- Where Aztec Code is used today:
--
--   * __IATA boarding passes__ — the barcode on every airline boarding pass
--   * __Eurostar and Amtrak rail tickets__ — printed and on-screen
--   * __PostNL, Deutsche Post, La Poste__ — European postal routing
--   * __US military ID cards__
--
-- == Symbol variants
--
-- @
-- Compact: 1–4 layers,  size = 11 + 4 × layers  (15×15 to 27×27)
-- Full:    1–32 layers, size = 15 + 4 × layers  (19×19 to 143×143)
-- @
--
-- == Encoding pipeline (v0.1.0 — byte-mode only)
--
-- @
-- input string
--   → Binary-Shift codewords from Upper mode
--   → symbol size selection (smallest compact then full that fits at 23% ECC)
--   → pad to exact codeword count
--   → GF(256)\/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots α¹..αⁿ)
--   → bit stuffing (insert complement after 4 consecutive identical bits)
--   → GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
--   → ModuleGrid (bullseye → orientation marks → mode msg → data spiral)
-- @
--
-- == v0.1.0 simplifications
--
--   1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
--      Multi-mode (Digit\/Upper\/Lower\/Mixed\/Punct) optimisation is v0.2.0.
--   2. 8-bit codewords → GF(256) RS (same polynomial as Data Matrix: 0x12D).
--      GF(16) and GF(32) RS for 4-bit\/5-bit codewords are v0.2.0.
--   3. Default ECC = 23%.
--   4. Auto-select compact vs full (force-compact option is v0.2.0).
module CodingAdventures.AztecCode
  ( -- * Primary entry points
    encodeAztecCode
  , encodeWithOptions
  , encodeAndLayout

    -- * Options
  , AztecOptions (..)
  , defaultOptions

    -- * Error type
  , AztecError (..)

    -- * Re-exported grid type
  , ModuleGrid (..)

    -- * Version
  , version
  ) where

import Data.Bits  ((.&.), (.|.), shiftL, shiftR, xor)
import Data.Char  (ord)
import qualified Data.Vector as V

import qualified CodingAdventures.Barcode2D as B2D
import CodingAdventures.Barcode2D
  ( Barcode2DLayoutConfig (..)
  , ModuleGrid (..)
  , emptyGrid
  , layout
  , setModule
  )
import CodingAdventures.PaintInstructions (PaintScene)

-- ---------------------------------------------------------------------------
-- Version
-- ---------------------------------------------------------------------------

-- | Library version string.
version :: String
version = "0.1.0"

-- ---------------------------------------------------------------------------
-- Public options
-- ---------------------------------------------------------------------------

-- | Options for the Aztec Code encoder.
--
-- Use 'defaultOptions' as a starting point:
--
-- @
-- let opts = defaultOptions { azMinEccPercent = 33 }
-- @
data AztecOptions = AztecOptions
  { azMinEccPercent :: !Int
    -- ^ Minimum ECC percentage (default: 23, range: 10–90).
    -- The encoder allocates at least this fraction of symbol capacity to
    -- Reed-Solomon check symbols.
  } deriving (Show, Eq)

-- | Default options: 23% ECC, auto-select compact vs full.
defaultOptions :: AztecOptions
defaultOptions = AztecOptions { azMinEccPercent = 23 }

-- ---------------------------------------------------------------------------
-- Error types
-- ---------------------------------------------------------------------------

-- | All errors the Aztec Code encoder can raise.
data AztecError
  = InputTooLong String
    -- ^ Input data does not fit in any Aztec Code symbol up to 32 full layers.
    -- The largest full symbol (143×143, 32 layers) holds approximately 3471
    -- bytes at 23% ECC.
  deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- GF(16) arithmetic — for mode message Reed-Solomon
-- ---------------------------------------------------------------------------
--
-- GF(16) is the finite field with 16 elements, built from the primitive
-- polynomial:
--
--   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
--
-- Every non-zero element can be written as a power of the primitive element
-- alpha.  Since alpha is a root of p(x), we have alpha^4 = alpha + 1.
--
-- Powers of alpha (the discrete-log / antilog tables):
--
--   i  |  alpha^i  | binary  | decimal
--   ---+-----------+---------+--------
--    0  |  alpha^0  | 0001    |  1
--    1  |  alpha^1  | 0010    |  2
--    2  |  alpha^2  | 0100    |  4
--    3  |  alpha^3  | 1000    |  8
--    4  |  alpha^4  | 0011    |  3   (x^4 = x+1 mod p)
--    5  |  alpha^5  | 0110    |  6
--    6  |  alpha^6  | 1100    | 12
--    7  |  alpha^7  | 1011    | 11
--    8  |  alpha^8  | 0101    |  5
--    9  |  alpha^9  | 1010    | 10
--   10  | alpha^10  | 0111    |  7
--   11  | alpha^11  | 1110    | 14
--   12  | alpha^12  | 1111    | 15
--   13  | alpha^13  | 1101    | 13
--   14  | alpha^14  | 1001    |  9
--   15  | alpha^15  | 0001    |  1   (period = 15; wraps to alpha^0)
--
-- Multiplication: a × b = alpha^((log[a] + log[b]) mod 15)

-- | GF(16) discrete-logarithm table: 'gf16Log' !! e = k  such that alpha^k = e.
-- Index 0 is undefined (log(0) = –∞); stored as –1 for safety.
gf16Log :: V.Vector Int
gf16Log = V.fromList
  [ -1   -- log(0)  = undefined
  ,  0   -- log(1)  = 0
  ,  1   -- log(2)  = 1
  ,  4   -- log(3)  = 4
  ,  2   -- log(4)  = 2
  ,  8   -- log(5)  = 8
  ,  5   -- log(6)  = 5
  , 10   -- log(7)  = 10
  ,  3   -- log(8)  = 3
  , 14   -- log(9)  = 14
  ,  9   -- log(10) = 9
  ,  7   -- log(11) = 7
  ,  6   -- log(12) = 6
  , 13   -- log(13) = 13
  , 11   -- log(14) = 11
  , 12   -- log(15) = 12
  ]

-- | GF(16) antilogarithm table: 'gf16Alog' !! k = alpha^k.
-- Length 16: index 15 wraps to alpha^0 = 1 (supporting period-15 arithmetic).
gf16Alog :: V.Vector Int
gf16Alog = V.fromList
  [ 1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1 ]

-- | Multiply two elements in GF(16).
--
-- For non-zero @a@ and @b@:
--
-- @
-- gf16Mul a b = alpha ^ ((gf16Log[a] + gf16Log[b]) `mod` 15)
-- @
--
-- If either operand is 0, the product is 0 (zero is absorbing for ×).
gf16Mul :: Int -> Int -> Int
gf16Mul a b
  | a == 0 || b == 0 = 0
  | otherwise        = gf16Alog V.! ((gf16Log V.! a + gf16Log V.! b) `mod` 15)

-- | Build the GF(16) RS generator polynomial with roots alpha^1 .. alpha^n.
--
-- The generator is:
--
-- @
-- g(x) = (x + alpha^1)(x + alpha^2) ··· (x + alpha^n)
-- @
--
-- We build it incrementally: start with @[1]@ and multiply by each factor
-- @(x + alpha^i)@.  The result is a coefficient list in big-endian order
-- (highest-degree term first; last element is the constant term).
buildGf16Generator :: Int -> [Int]
buildGf16Generator n = foldl' step [1] [1 .. n]
  where
    -- Multiply polynomial g by (x + alpha^i) in GF(16):
    --   g(x) * x      = coefficients shifted left (append 0 at end)
    --   g(x) * alpha^i = each coefficient scaled by alpha^i (prepend 0)
    -- Add the two polynomials with XOR (= addition in GF(2)-char field).
    step g i =
      let ai  = gf16Alog V.! (i `mod` 15)
          gx  = g ++ [0]
          gai = 0 : map (gf16Mul ai) g
      in  zipWith xor gx gai

-- | Compute @n@ GF(16) RS check nibbles for the given data nibbles.
--
-- Implements LFSR shift-register polynomial division:
--
-- @
-- remainder[0..n-1] = 0
-- for each data nibble d:
--   feedback = d XOR remainder[0]
--   remainder = remainder[1..] ++ [0]
--   for i in 0..n-1:
--     remainder[i] ^= generator[i+1] * feedback
-- @
gf16RsEncode :: [Int]  -- ^ Data nibbles (0–15 each)
             -> Int    -- ^ Number of ECC nibbles to produce
             -> [Int]  -- ^ ECC nibbles
gf16RsEncode dataWords nEcc =
  let g         = buildGf16Generator nEcc
      genCoeffs = drop 1 g   -- skip monic leading 1
      initRem   = replicate nEcc 0
      step acc d =
        case acc of
          []      -> []
          (r0:rs) ->
            let fb   = d `xor` r0
                acc' = rs ++ [0]
            in  if fb == 0
                then acc'
                else zipWith xor acc' (map (gf16Mul fb) genCoeffs)
  in  foldl' step initRem dataWords

-- ---------------------------------------------------------------------------
-- GF(256)/0x12D arithmetic — for 8-bit data codewords
-- ---------------------------------------------------------------------------
--
-- Aztec Code uses GF(256) with primitive polynomial:
--
--   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301
--
-- This is the SAME polynomial as Data Matrix ECC200.
--
--   QR Code:     0x11D  =  x^8 + x^4 + x^3 + x^2 + 1
--   Data Matrix: 0x12D  =  x^8 + x^5 + x^4 + x^2 + x + 1  ← Aztec uses this
--
-- We build our own tables inline (the repo's gf256 package uses 0x11D for QR).
-- Generator convention: b=1, roots alpha^1..alpha^n — same as Data Matrix.

-- | Primitive polynomial value for GF(256)/Aztec: x^8 + x^5 + x^4 + x^2 + x + 1.
gf256Poly :: Int
gf256Poly = 0x12d

-- | Build the exp and log tables for GF(256)/0x12D.
-- Returns (expTable, logTable).
-- expTable[i] = alpha^i for i in 0..254 (then repeats; length 512 for fast mul).
-- logTable[e] = k such that alpha^k = e.
buildGf256Tables :: (V.Vector Int, V.Vector Int)
buildGf256Tables = (expVec, logVec)
  where
    buildExp :: Int -> Int -> [Int] -> [Int]
    buildExp _   255 acc = reverse acc
    buildExp val i   acc =
      let val' = let v = val `shiftL` 1
                 in  if v .&. 0x100 /= 0 then (v `xor` gf256Poly) .&. 0xFF else v
      in  buildExp val' (i + 1) (val : acc)

    expList = buildExp 1 0 []

    -- Double the table for fast multiply (index la+lb without mod-255)
    expVec = V.fromList (expList ++ expList ++ [0, 0])

    logVec = V.replicate 256 0 V.//
               [ (expList !! i, i) | i <- [0 .. 254] ]

-- | Precomputed exp table for GF(256)/0x12D (length ≥ 510).
gf256Exp :: V.Vector Int
gf256Exp = fst buildGf256Tables

-- | Precomputed log table for GF(256)/0x12D (length 256).
gf256Log :: V.Vector Int
gf256Log = snd buildGf256Tables

-- | Multiply two GF(256)/0x12D elements.
--
-- @
-- gf256Mul a b = alpha ^ ((gf256Log[a] + gf256Log[b]) mod 255)
-- @
--
-- Returns 0 if either operand is 0.
gf256Mul :: Int -> Int -> Int
gf256Mul a b
  | a == 0 || b == 0 = 0
  | otherwise        = gf256Exp V.! (gf256Log V.! a + gf256Log V.! b)

-- | Build the GF(256)/0x12D RS generator polynomial with roots alpha^1..alpha^n.
--
-- Returns big-endian coefficients (highest degree first).
buildGf256Generator :: Int -> [Int]
buildGf256Generator n = foldl' step [1] [1 .. n]
  where
    step g i =
      let ai  = gf256Exp V.! i
          gx  = g ++ [0]
          gai = 0 : map (gf256Mul ai) g
      in  zipWith xor gx gai

-- | Compute @n@ GF(256)/0x12D RS check bytes via LFSR polynomial division.
gf256RsEncode :: [Int]  -- ^ Data bytes (0–255)
              -> Int    -- ^ Number of ECC bytes
              -> [Int]  -- ^ ECC bytes
gf256RsEncode dataBytes nEcc =
  let g         = buildGf256Generator nEcc
      genCoeffs = drop 1 g
      initRem   = replicate nEcc 0
      step acc b =
        case acc of
          []      -> []
          (r0:rs) ->
            let fb   = b `xor` r0
                acc' = rs ++ [0]
            in  if fb == 0
                then acc'
                else zipWith xor acc' (map (gf256Mul fb) genCoeffs)
  in  foldl' step initRem dataBytes

-- ---------------------------------------------------------------------------
-- Capacity tables (from ISO/IEC 24778:2008 Table 1 and TypeScript reference)
-- ---------------------------------------------------------------------------
--
-- Each entry stores the total bit positions available (data+ECC) for that
-- layer count, and the maximum number of 8-bit codewords (byte mode).
--
-- These are the usable data+ECC bit positions after subtracting structural
-- elements (bullseye, mode message ring, orientation marks, reference grid).

-- | A single capacity entry: (totalBits, maxBytes8).
data CapEntry = CapEntry
  { capTotalBits :: !Int  -- ^ Total bit positions for data+ECC
  , capMaxBytes  :: !Int  -- ^ Maximum 8-bit codewords that fit
  } deriving (Show, Eq)

-- | Compact Aztec capacity (index 1..4 = layers 1..4; index 0 is a dummy).
compactCap :: V.Vector CapEntry
compactCap = V.fromList
  [ CapEntry  0   0   -- dummy
  , CapEntry  72   9  -- 1 layer, 15×15
  , CapEntry 200  25  -- 2 layers, 19×19
  , CapEntry 392  49  -- 3 layers, 23×23
  , CapEntry 648  81  -- 4 layers, 27×27
  ]

-- | Full Aztec capacity (index 1..32 = layers 1..32; index 0 is a dummy).
fullCap :: V.Vector CapEntry
fullCap = V.fromList
  [ CapEntry      0     0  -- dummy
  , CapEntry     88    11  -- layer  1
  , CapEntry    216    27  -- layer  2
  , CapEntry    360    45  -- layer  3
  , CapEntry    520    65  -- layer  4
  , CapEntry    696    87  -- layer  5
  , CapEntry    888   111  -- layer  6
  , CapEntry   1096   137  -- layer  7
  , CapEntry   1320   165  -- layer  8
  , CapEntry   1560   195  -- layer  9
  , CapEntry   1816   227  -- layer 10
  , CapEntry   2088   261  -- layer 11
  , CapEntry   2376   297  -- layer 12
  , CapEntry   2680   335  -- layer 13
  , CapEntry   3000   375  -- layer 14
  , CapEntry   3336   417  -- layer 15
  , CapEntry   3688   461  -- layer 16
  , CapEntry   4056   507  -- layer 17
  , CapEntry   4440   555  -- layer 18
  , CapEntry   4840   605  -- layer 19
  , CapEntry   5256   657  -- layer 20
  , CapEntry   5688   711  -- layer 21
  , CapEntry   6136   767  -- layer 22
  , CapEntry   6600   825  -- layer 23
  , CapEntry   7080   885  -- layer 24
  , CapEntry   7576   947  -- layer 25
  , CapEntry   8088  1011  -- layer 26
  , CapEntry   8616  1077  -- layer 27
  , CapEntry   9160  1145  -- layer 28
  , CapEntry   9720  1215  -- layer 29
  , CapEntry  10296  1287  -- layer 30
  , CapEntry  10888  1361  -- layer 31
  , CapEntry  11496  1437  -- layer 32
  ]

-- ---------------------------------------------------------------------------
-- Symbol specification
-- ---------------------------------------------------------------------------

-- | Selected Aztec symbol parameters.
data SymbolSpec = SymbolSpec
  { ssCompact     :: !Bool  -- ^ True = compact, False = full
  , ssLayers      :: !Int   -- ^ Layer count
  , ssDataCwCount :: !Int   -- ^ 8-bit data codeword slots
  , ssEccCwCount  :: !Int   -- ^ 8-bit ECC codeword count
  } deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- Binary-Shift data encoding
-- ---------------------------------------------------------------------------
--
-- All v0.1.0 input is wrapped in a single Binary-Shift block from Upper mode:
--
--   1. Emit 5-bit codeword 31 (binary 11111) = Binary-Shift escape in Upper mode
--   2. If len <= 31: 5 bits for the byte count
--      If len > 31:  5 bits = 00000, then 11 bits for the byte count
--   3. Each byte as 8 bits, MSB first
--
-- Example: encoding 3 bytes [0x41, 0x42, 0x43] ("ABC"):
--
--   Escape:    1 1 1 1 1
--   Length 3:  0 0 0 1 1
--   'A'=0x41:  0 1 0 0 0 0 0 1
--   'B'=0x42:  0 1 0 0 0 0 1 0
--   'C'=0x43:  0 1 0 0 0 0 1 1
--
-- Total bits: 5 + 5 + 3×8 = 34 bits.

-- | Encode a byte sequence as Aztec codeword bits using the Binary-Shift escape.
--
-- Returns a flat list of 0\/1 values, MSB first.
encodeBytesAsBits :: [Int]   -- ^ Input bytes (0–255 each)
                  -> [Int]   -- ^ Flat bit sequence
encodeBytesAsBits input =
  let len      = length input
      writeBits v cnt = [ (v `shiftR` i) .&. 1 | i <- [cnt - 1, cnt - 2 .. 0] ]
      escape   = writeBits 31 5
      lenBits  = if len <= 31
                 then writeBits len 5
                 else writeBits 0 5 ++ writeBits len 11
      dataB    = concatMap (`writeBits` 8) input
  in  escape ++ lenBits ++ dataB

-- ---------------------------------------------------------------------------
-- Symbol size selection
-- ---------------------------------------------------------------------------

-- | Select the smallest Aztec symbol that can hold the encoded data.
--
-- A 20% conservative stuffing overhead is added to the raw bit count.  This
-- ensures the selected symbol is large enough even for worst-case inputs
-- (which can produce up to 25% extra bits from bit stuffing).
--
-- Tries compact layers 1–4 first, then full layers 1–32.
-- Returns 'InputTooLong' if the data exceeds 32-layer full capacity.
selectSymbol :: Int                          -- ^ Raw data bit count (before stuffing)
             -> Int                          -- ^ Min ECC percentage
             -> Either AztecError SymbolSpec
selectSymbol dataBits minEccPct =
  let -- 20% stuffing overhead: bits * 1.2, rounded up
      stuffedBytes = (dataBits * 12 + 79) `div` 80  -- = ceil(dataBits * 1.2 / 8)

      tryLayer isCompact l cap =
        let totalCw  = capMaxBytes cap
            eccCw    = (minEccPct * totalCw + 99) `div` 100  -- ceiling division
            dataCw   = totalCw - eccCw
        in  if dataCw > 0 && stuffedBytes <= dataCw
            then Just SymbolSpec
                   { ssCompact     = isCompact
                   , ssLayers      = l
                   , ssDataCwCount = dataCw
                   , ssEccCwCount  = eccCw
                   }
            else Nothing

      compactTries = [ tryLayer True l (compactCap V.! l) | l <- [1..4]  ]
      fullTries    = [ tryLayer False l (fullCap    V.! l) | l <- [1..32] ]

      firstJust []           = Nothing
      firstJust (Just x:_)   = Just x
      firstJust (Nothing:xs) = firstJust xs

  in  case firstJust (compactTries ++ fullTries) of
        Just spec -> Right spec
        Nothing   -> Left $ InputTooLong
          ("aztec-code: input requires " ++ show dataBits ++
           " bits; exceeds 32-layer full symbol capacity (~93k bits).")

-- ---------------------------------------------------------------------------
-- Padding
-- ---------------------------------------------------------------------------

-- | Pad (or trim) a bit list to exactly @targetBytes * 8@ bits.
--
-- First aligns the length to the next byte boundary by zero-filling, then
-- pads with 0-bits to reach the full target, or trims if already over.
padToBytes :: [Int]  -- ^ Input bits
           -> Int    -- ^ Target byte count
           -> [Int]  -- ^ Output: exactly targetBytes * 8 bits
padToBytes bits targetBytes =
  let extra  = (8 - (length bits `mod` 8)) `mod` 8
      aligned = bits ++ replicate extra 0
      target  = targetBytes * 8
  in  take target (aligned ++ repeat 0)

-- ---------------------------------------------------------------------------
-- Bit stuffing
-- ---------------------------------------------------------------------------
--
-- Rule: after every run of 4 consecutive identical bits, insert one complement.
--
-- The inserted (stuff) bit starts a new run of length 1. A fifth consecutive
-- identical bit does NOT trigger another stuff because the run counter reset.
--
-- Example demonstrating reset:
--   Input:   1 1 1 1 1 0 0 0 0 1
--   After 4× 1: emit stuff 0 → [1 1 1 1 | 0]    run resets to (0, len=1)
--   Next bit (1): new run (1, 1) → emit 1       → [1 1 1 1 0 1]
--   Next bit (0): new run (0, 1) → emit 0       → [1 1 1 1 0 1 0]
--   Continue ... after 4× 0: emit stuff 1
--   Final: 1 1 1 1 0 1 0 0 0 0 1 1
--
-- Stuffing applies ONLY to data+ECC bits. The bullseye, orientation marks,
-- mode message, and reference grid are exempt.

-- | Apply Aztec bit stuffing to the data+ECC bit stream.
--
-- After every run of 4 identical bits, inserts one complement bit.
stuffBits :: [Int]  -- ^ Input bits (0\/1)
          -> [Int]  -- ^ Stuffed output bits
stuffBits = go ((-1) :: Int) (0 :: Int) []
  where
    -- go runVal runLen accum remainingInput
    go _      _      acc []     = reverse acc
    go runVal runLen acc (b:bs) =
      let rl' = if b == runVal then runLen + 1 else 1
          acc' = b : acc
      in  if rl' == 4
          then go (1 - b) 1 ((1 - b) : acc') bs  -- stuff complement, reset run
          else go b       rl'  acc'           bs

-- ---------------------------------------------------------------------------
-- Mode message encoding
-- ---------------------------------------------------------------------------
--
-- The mode message records how many data layers and data codewords the symbol
-- contains, protected by GF(16) Reed-Solomon.
--
-- Compact (28 bits = 7 nibbles):
--   Combined value m = ((layers-1) << 6) | (dataCwCount-1)    [8 bits]
--   dataNibbles = [m & 0xF, (m >> 4) & 0xF]                   [2 nibbles]
--   eccNibbles  = gf16RsEncode dataNibbles 5                   [5 nibbles]
--   allNibbles  = dataNibbles ++ eccNibbles                    [7 nibbles = 28 bits]
--
-- Full (40 bits = 10 nibbles):
--   Combined value m = ((layers-1) << 11) | (dataCwCount-1)   [16 bits]
--   dataNibbles = [m&0xF, (m>>4)&0xF, (m>>8)&0xF, (m>>12)&0xF] [4 nibbles]
--   eccNibbles  = gf16RsEncode dataNibbles 6                   [6 nibbles]
--   allNibbles  = dataNibbles ++ eccNibbles                    [10 nibbles = 40 bits]
--
-- Bits are written MSB-first from each nibble.

-- | Encode the mode message as a flat bit array.
--
-- Returns 28 bits for compact mode and 40 bits for full mode.
encodeModeMessage :: Bool    -- ^ True = compact, False = full
                  -> Int     -- ^ Layer count
                  -> Int     -- ^ Data codeword count
                  -> [Int]   -- ^ Mode message bits
encodeModeMessage isCompact layers dataCwCount =
  let (dataNibbles, nEcc)
        | isCompact =
            let m = ((layers - 1) `shiftL` 6) .|. (dataCwCount - 1)
            in  ([m .&. 0xF, (m `shiftR` 4) .&. 0xF], 5 :: Int)
        | otherwise =
            let m = ((layers - 1) `shiftL` 11) .|. (dataCwCount - 1)
            in  ( [ m .&. 0xF
                  , (m `shiftR` 4) .&. 0xF
                  , (m `shiftR` 8) .&. 0xF
                  , (m `shiftR` 12) .&. 0xF
                  ]
                , 6 :: Int)

      eccNibbles = gf16RsEncode dataNibbles nEcc
      allNibbles = dataNibbles ++ eccNibbles

      -- Each nibble: 4 bits MSB first
      nibToBits n = [ (n `shiftR` i) .&. 1 | i <- [3, 2, 1, 0] ]

  in  concatMap nibToBits allNibbles

-- ---------------------------------------------------------------------------
-- Grid helpers
-- ---------------------------------------------------------------------------

-- | Square symbol side length.
symbolSize :: Bool -> Int -> Int
symbolSize isCompact layers = if isCompact then 11 + 4 * layers else 15 + 4 * layers

-- | Chebyshev distance to the outermost bullseye ring.
bullseyeRadius :: Bool -> Int
bullseyeRadius isCompact = if isCompact then 5 else 7

-- | Set a single module in a row-major 2D list grid (pure update).
setCell :: [[Bool]] -> Int -> Int -> Bool -> [[Bool]]
setCell g r c val =
  let rowOld = g !! r
      rowNew  = take c rowOld ++ [val] ++ drop (c + 1) rowOld
  in  take r g ++ [rowNew] ++ drop (r + 1) g

-- | Read a single module.
getCell :: [[Bool]] -> Int -> Int -> Bool
getCell g r c = (g !! r) !! c

-- ---------------------------------------------------------------------------
-- Bullseye finder pattern
-- ---------------------------------------------------------------------------
--
-- Module colour at Chebyshev distance d from centre (cx, cy):
--
--   d = 0: DARK (centre)
--   d = 1: DARK (solid 3×3 core — both d=0 and d=1 are dark)
--   d = 2: LIGHT
--   d = 3: DARK
--   d = 4: LIGHT
--   d = 5: DARK  ← outermost ring of compact bullseye (radius=5)
--   d = 6: LIGHT ← extra ring for full bullseye
--   d = 7: DARK  ← outermost ring of full bullseye (radius=7)
--
-- The resulting concentric-ring structure has a 1:1:1:1:1 cross-ratio
-- visible from any scan angle, enabling robust self-location.

-- | Draw the bullseye and mark all bullseye modules as reserved.
--
-- Returns updated (modules, reserved) grids.
drawBullseye :: [[Bool]] -> [[Bool]] -> Int -> Int -> Bool
             -> ([[Bool]], [[Bool]])
drawBullseye mods0 res0 cx cy isCompact =
  let br     = bullseyeRadius isCompact
      coords = [ (r, c)
               | r <- [cy - br .. cy + br]
               , c <- [cx - br .. cx + br]
               ]
      place (m, rsv) (r, c) =
        let d    = max (abs (c - cx)) (abs (r - cy))
            dark = d <= 1 || odd d   -- d=0,1 → dark; d=2,4,6 → light; d=3,5,7 → dark
        in  (setCell m r c dark, setCell rsv r c True)
  in  foldl' place (mods0, res0) coords

-- ---------------------------------------------------------------------------
-- Reference grid (full symbols only)
-- ---------------------------------------------------------------------------
--
-- Full symbols include a reference grid of alternating dark/light lines at
-- rows and columns that are multiples of 16 from the centre.
--
-- Module value at (row, col) on a reference grid position:
--
--   Both row and col on reference lines → DARK (intersection)
--   Only row on a reference line        → dark iff (cx – col) is even
--   Only col on a reference line        → dark iff (cy – row) is even
--
-- The reference grid is drawn BEFORE the bullseye: the bullseye pattern
-- overwrites any reference grid modules within the bullseye radius.

-- | Draw reference grid lines for a full Aztec symbol.
drawReferenceGrid :: [[Bool]] -> [[Bool]] -> Int -> Int -> Int
                  -> ([[Bool]], [[Bool]])
drawReferenceGrid mods0 res0 cx cy size =
  let isRefRow r = (cy - r) `mod` 16 == 0
      isRefCol c = (cx - c) `mod` 16 == 0
      coords = [ (r, c)
               | r <- [0 .. size - 1]
               , c <- [0 .. size - 1]
               , isRefRow r || isRefCol c
               ]
      place (m, rsv) (r, c) =
        let onH  = isRefRow r
            onV  = isRefCol c
            dark | onH && onV = True
                 | onH        = (cx - c) `mod` 2 == 0
                 | otherwise  = (cy - r) `mod` 2 == 0
        in  (setCell m r c dark, setCell rsv r c True)
  in  foldl' place (mods0, res0) coords

-- ---------------------------------------------------------------------------
-- Orientation marks and mode message placement
-- ---------------------------------------------------------------------------
--
-- The mode message ring is the perimeter at Chebyshev radius (bullseyeRadius+1).
-- Its four corner modules are DARK orientation marks (always fixed).
-- The remaining non-corner modules carry mode message bits clockwise from TL+1.
--
-- Non-corner perimeter positions, clockwise from top-left+1:
--
--   Top edge:    (col, cy–r) for col ∈ [cx–r+1 .. cx+r–1]    (left→right)
--   Right edge:  (cx+r, row) for row ∈ [cy–r+1 .. cy+r–1]    (top→bottom)
--   Bottom edge: (col, cy+r) for col ∈ [cx+r–1 .. cx–r+1]    (right→left)
--   Left edge:   (cx–r, row) for row ∈ [cy+r–1 .. cy–r+1]    (bottom→top)
--
-- Tuple convention: (col, row) = (x, y) = (column, row).
--
-- The mode message bits fill the first |modeMsg| positions.
-- The REMAINING positions are returned for the data bit stream to continue.

-- | Draw orientation marks and mode message; return remaining ring positions.
drawOrientationAndModeMsg
  :: [[Bool]]     -- ^ modules
  -> [[Bool]]     -- ^ reserved
  -> Int          -- ^ cx
  -> Int          -- ^ cy
  -> Bool         -- ^ isCompact
  -> [Int]        -- ^ mode message bits (28 or 40)
  -> ([[Bool]], [[Bool]], [(Int, Int)])
  -- ^  (modules', reserved', remainingPositions [(col, row)])
drawOrientationAndModeMsg mods0 res0 cx cy isCompact modeMsgBits =
  let r = bullseyeRadius isCompact + 1

      -- Non-corner perimeter, clockwise from top-left+1
      nonCorner :: [(Int, Int)]  -- (col, row)
      nonCorner =
        [ (col, cy - r) | col <- [cx - r + 1 .. cx + r - 1] ]      -- top
        ++ [ (cx + r, row) | row <- [cy - r + 1 .. cy + r - 1] ]   -- right
        ++ [ (col, cy + r) | col <- [cx + r - 1, cx + r - 2 .. cx - r + 1] ]  -- bottom
        ++ [ (cx - r, row) | row <- [cy + r - 1, cy + r - 2 .. cy - r + 1] ]  -- left

      -- Four orientation mark corners (always DARK, always reserved)
      corners :: [(Int, Int)]  -- (col, row)
      corners =
        [ (cx - r, cy - r)
        , (cx + r, cy - r)
        , (cx + r, cy + r)
        , (cx - r, cy + r)
        ]

      -- Place orientation marks
      markCorner (m, rsv) (col, row) =
        (setCell m row col True, setCell rsv row col True)
      (mods1, res1) = foldl' markCorner (mods0, res0) corners

      -- Place mode message bits
      nMsg         = length modeMsgBits
      msgPositions = take nMsg nonCorner
      placeMsg (m, rsv) ((col, row), bit) =
        (setCell m row col (bit == 1), setCell rsv row col True)
      (mods2, res2) = foldl' placeMsg (mods1, res1) (zip msgPositions modeMsgBits)

      -- Remaining positions after the mode message are used by the data stream
      remaining = drop nMsg nonCorner

  in  (mods2, res2, remaining)

-- ---------------------------------------------------------------------------
-- Data layer spiral placement
-- ---------------------------------------------------------------------------
--
-- Bits are placed in a clockwise spiral starting from the innermost data layer.
-- Each layer band is 2 modules wide (inner row/col and outer row/col).
-- Within a layer, the pair order is: outer row/col first, then inner.
--
-- For compact: first data layer inner radius = bullseyeRadius + 2 = 7
-- For full:    first data layer inner radius = bullseyeRadius + 2 = 9
--
-- Layer L (0-indexed from innermost):
--   dI = (bullseyeRadius + 2) + 2*L    inner radius
--   dO = dI + 1                         outer radius
--
-- Placement sequence for layer L:
--   Top edge:    for col ∈ [cx–dI+1 .. cx+dI]:   (col, cy–dO), (col, cy–dI)
--   Right edge:  for row ∈ [cy–dI+1 .. cy+dI]:   (cx+dO, row), (cx+dI, row)
--   Bottom edge: for col ∈ [cx+dI .. cx–dI+1]:   (col, cy+dO), (col, cy+dI)
--   Left edge:   for row ∈ [cy+dI .. cy–dI+1]:   (cx–dO, row), (cx–dI, row)
--
-- Modules already marked as reserved (bullseye, ref grid, mode msg) are
-- skipped WITHOUT consuming a bit — the bit stream is contiguous across gaps.

-- | Generate the ordered list of (col, row) positions for all data layers.
--
-- The mode ring remaining positions are prepended.
layerPositions :: Bool -> Int -> Int -> Int -> [(Int, Int)]
layerPositions isCompact cx cy layers =
  let br     = bullseyeRadius isCompact
      dStart = br + 2
      layerPos l =
        let dI = dStart + 2 * l
            dO = dI + 1
        in  -- Top edge: outer then inner
            concatMap (\col -> [(col, cy - dO), (col, cy - dI)])
              [cx - dI + 1 .. cx + dI]
            ++
            -- Right edge
            concatMap (\row -> [(cx + dO, row), (cx + dI, row)])
              [cy - dI + 1 .. cy + dI]
            ++
            -- Bottom edge
            concatMap (\col -> [(col, cy + dO), (col, cy + dI)])
              [cx + dI, cx + dI - 1 .. cx - dI + 1]
            ++
            -- Left edge
            concatMap (\row -> [(cx - dO, row), (cx - dI, row)])
              [cy + dI, cy + dI - 1 .. cy - dI + 1]
  in  concatMap layerPos [0 .. layers - 1]

-- | Place data+ECC bits into the symbol grid.
--
-- Fills remaining mode ring positions first, then spirals outward.
-- Reserved modules are skipped without consuming a bit.
placeDataBits
  :: [[Bool]]      -- ^ modules grid
  -> [[Bool]]      -- ^ reserved grid (modules that must not receive data bits)
  -> [Int]         -- ^ stuffed data+ECC bits (0\/1)
  -> Int           -- ^ cx — centre column
  -> Int           -- ^ cy — centre row
  -> Bool          -- ^ isCompact
  -> Int           -- ^ layer count
  -> [(Int, Int)]  -- ^ remaining mode ring positions [(col, row)]
  -> [[Bool]]      -- ^ updated modules grid
placeDataBits mods0 res stuffed cx cy isCompact layers modeRem =
  let sz     = symbolSize isCompact layers
      -- All positions to visit: mode ring remainder first, then data spiral
      allPos = modeRem ++ layerPositions isCompact cx cy layers
      -- Step: skip reserved/out-of-bounds modules; consume one bit otherwise
      step (m, bits) (col, row)
        | row < 0 || row >= sz || col < 0 || col >= sz
            = (m, bits)  -- out of bounds: skip without consuming
        | getCell res row col
            = (m, bits)  -- reserved: skip without consuming
        | otherwise = case bits of
            []    -> (m, [])
            (b:bs) -> (setCell m row col (b == 1), bs)
  in  fst $ foldl' step (mods0, stuffed) allPos

-- ---------------------------------------------------------------------------
-- Core encoding pipeline
-- ---------------------------------------------------------------------------

-- | Encode a 'String' into an Aztec Code module grid.
--
-- The input is treated as a sequence of Unicode code points (ISO 8859-1
-- compatible for code points 0–255). For full UTF-8 support, encode to UTF-8
-- bytes first and pass them to 'encodeWithOptions'.
--
-- Returns @Right grid@ where each row is a @[Bool]@ (True = dark module),
-- or @Left AztecError@ if the input is too long.
--
-- == Grid dimensions
--
-- @
-- Compact N layers: (11+4N) × (11+4N)
-- Full N layers:    (15+4N) × (15+4N)
-- @
--
-- == Example
--
-- @
-- case encodeAztecCode "HELLO" of
--   Left err  -> putStrLn ("Error: " ++ show err)
--   Right mat -> putStrLn ("Size: " ++ show (length mat) ++ "×" ++ show (length (head mat)))
-- @
encodeAztecCode :: String -> Either AztecError [[Bool]]
encodeAztecCode input = do
  grid <- encodeWithOptions (map ord input) defaultOptions
  let nRows = B2D.mgRows grid
      nCols = B2D.mgCols grid
      mods  = mgModules grid
  return [ [ mods V.! (r * nCols + c) | c <- [0 .. nCols - 1] ]
         | r <- [0 .. nRows - 1] ]


-- | Encode bytes into an Aztec Code 'ModuleGrid'.
--
-- This is the core pipeline function. The returned 'ModuleGrid' can be
-- passed to barcode-2d's 'layout' function for pixel rendering.
encodeWithOptions :: [Int]          -- ^ Input bytes (0–255)
                  -> AztecOptions
                  -> Either AztecError ModuleGrid
encodeWithOptions inputBytes opts = do
  -- ─── Step 1: Encode data bits ───────────────────────────────────────────
  let dataBits = encodeBytesAsBits inputBytes

  -- ─── Step 2: Select symbol size ─────────────────────────────────────────
  spec <- selectSymbol (length dataBits) (azMinEccPercent opts)
  let isCompact   = ssCompact spec
      layers      = ssLayers spec
      dataCwCount = ssDataCwCount spec
      eccCwCount  = ssEccCwCount spec

  -- ─── Step 3: Pad to dataCwCount bytes ────────────────────────────────────
  let paddedBits = padToBytes dataBits dataCwCount

      byteAt i =
        let b = sum [ (paddedBits !! (i * 8 + bit)) `shiftL` (7 - bit)
                    | bit <- [0..7] ]
        -- All-zero avoidance: the last data codeword must not be 0x00, since
        -- that would make the RS polynomial degenerate. Replace with 0xFF.
        in  if b == 0 && i == dataCwCount - 1 then 0xFF else b

      dataBytes = map byteAt [0 .. dataCwCount - 1]

  -- ─── Step 4: Compute GF(256)/0x12D Reed-Solomon ECC ─────────────────────
  let eccBytes = gf256RsEncode dataBytes eccCwCount

  -- ─── Step 5: Build bit stream and apply bit stuffing ─────────────────────
  let allBytes = dataBytes ++ eccBytes
      rawBits  = concatMap (\b -> [(b `shiftR` i) .&. 1 | i <- [7, 6 .. 0]]) allBytes
      stuffed  = stuffBits rawBits

  -- ─── Step 6: Encode mode message ─────────────────────────────────────────
  let modeMsg = encodeModeMessage isCompact layers dataCwCount

  -- ─── Step 7: Initialise symbol grid ──────────────────────────────────────
  let sz     = symbolSize isCompact layers
      cx     = sz `div` 2
      cy     = sz `div` 2
      emptyM = replicate sz (replicate sz False)
      emptyR = replicate sz (replicate sz False)

      -- Full symbols: reference grid first (bullseye overwrites centre later)
      (mods0, res0)
        | isCompact = (emptyM, emptyR)
        | otherwise = drawReferenceGrid emptyM emptyR cx cy sz

      -- Bullseye finder pattern
      (mods1, res1) = drawBullseye mods0 res0 cx cy isCompact

      -- Orientation marks + mode message
      (mods2, res2, modeRem) =
        drawOrientationAndModeMsg mods1 res1 cx cy isCompact modeMsg

  -- ─── Step 8: Place data+ECC bits ─────────────────────────────────────────
  let modsFinal = placeDataBits mods2 res2 stuffed cx cy isCompact layers modeRem

  -- ─── Step 9: Convert [[Bool]] → ModuleGrid ───────────────────────────────
  let mgrid = foldl'
        (\g (r, c) ->
          if modsFinal !! r !! c then setModule g r c True else g)
        (emptyGrid sz sz B2D.Square)
        [ (r, c) | r <- [0 .. sz - 1], c <- [0 .. sz - 1] ]

  return mgrid

-- | Encode bytes and convert to a 'PaintScene' for pixel rendering.
--
-- Convenience function that runs the full pipeline from raw bytes to a
-- paint-instructions scene ready for the PaintVM (P2D01).
encodeAndLayout :: [Int]                  -- ^ Input bytes
                -> AztecOptions
                -> Barcode2DLayoutConfig
                -> Either AztecError PaintScene
encodeAndLayout input opts cfg = do
  grid <- encodeWithOptions input opts
  return (layout grid cfg)
