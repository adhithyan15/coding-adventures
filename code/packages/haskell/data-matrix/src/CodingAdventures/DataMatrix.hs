-- | CodingAdventures.DataMatrix — ISO\/IEC 16022:2006 Data Matrix ECC200 encoder.
--
-- == What is Data Matrix?
--
-- Data Matrix is a two-dimensional matrix barcode standardised as ISO\/IEC 16022:2006.
-- ECC200 is the modern variant that uses Reed-Solomon over GF(256).
--
-- Where Data Matrix is used:
--
--   * __PCBs__ — every modern board carries a tiny Data Matrix for traceability.
--   * __Pharmaceuticals__ — the US FDA DSCSA mandates Data Matrix on unit-dose packages.
--   * __Aerospace parts__ — etched marks survive decades of heat and abrasion.
--   * __Medical devices__ — GS1 DataMatrix on surgical instruments and implants.
--   * __Postage__ — USPS registered mail and customs forms.
--
-- == Key differences from QR Code
--
-- +------------------+-----------------------+--------------------------+
-- | Property         | QR Code               | Data Matrix ECC200       |
-- +------------------+-----------------------+--------------------------+
-- | GF(256) poly     | 0x11D                 | 0x12D                    |
-- | RS root start    | b=0 (α⁰..)            | b=1 (α¹..)               |
-- | Finder           | three corner squares  | one L-shape (L+bottom)   |
-- | Placement        | column zigzag         | "Utah" diagonal          |
-- | Masking          | 8 patterns, scored    | NONE                     |
-- | Sizes            | 40 versions           | 30 square + 6 rect       |
-- +------------------+-----------------------+--------------------------+
--
-- == Encoding pipeline
--
-- @
-- input string
--   → ASCII encoding      (chars+1; digit pairs packed into one codeword)
--   → symbol selection    (smallest symbol whose capacity ≥ codeword count)
--   → pad to capacity     (scrambled-pad codewords fill unused slots)
--   → RS blocks + ECC     (GF(256)\/0x12D, b=1 convention)
--   → interleave blocks   (data round-robin then ECC round-robin)
--   → grid init           (L-finder + timing border + alignment borders)
--   → Utah placement      (diagonal codeword placement, NO masking)
--   → ModuleGrid          (abstract boolean grid, true = dark)
-- @
module CodingAdventures.DataMatrix
  ( -- * Public encode functions
    encode
  , encodeAt
  , encodeAndLayout

    -- * Symbol shape selector
  , SymbolShape (..)

    -- * Options
  , DataMatrixOptions (..)
  , defaultOptions

    -- * Error types
  , DataMatrixError (..)

    -- * Re-exported grid type
  , ModuleGrid (..)

    -- * Version string
  , version
  ) where

import Data.Bits (xor, shiftR, shiftL, (.&.))
import Data.Char (ord)
import Data.List (sortBy, foldl')
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
-- SymbolShape — controls which sizes are considered
-- ---------------------------------------------------------------------------

-- | Controls which symbol shapes the encoder will consider.
--
-- 'Square' selects only the 24 square symbols (10×10 … 144×144).
-- 'Rectangular' selects only the 6 rectangular symbols (8×18 … 16×48).
-- 'AnyShape' considers both and picks the smallest by module count.
data SymbolShape
  = Square
    -- ^ Select only from the 24 square symbols. This is the default.
  | Rectangular
    -- ^ Select only from the 6 rectangular symbols.
  | AnyShape
    -- ^ Consider all 30 symbols; pick the smallest.
  deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- DataMatrixOptions
-- ---------------------------------------------------------------------------

-- | Options for the Data Matrix encoder.
--
-- Use 'defaultOptions' as the starting point and override with record syntax:
--
-- @
-- let opts = defaultOptions { dmShape = Rectangular }
-- @
data DataMatrixOptions = DataMatrixOptions
  { dmShape :: SymbolShape
    -- ^ Which symbol shapes to consider. Default: 'Square'.
  } deriving (Show, Eq)

-- | Default options: square symbols only.
defaultOptions :: DataMatrixOptions
defaultOptions = DataMatrixOptions { dmShape = Square }

-- ---------------------------------------------------------------------------
-- Error types
-- ---------------------------------------------------------------------------

-- | All errors the Data Matrix encoder can raise.
data DataMatrixError
  = InputTooLong String
    -- ^ Input encodes to more codewords than the largest symbol can hold.
    --   The largest ECC200 symbol is 144×144 with 1558 data codewords.
  | InvalidSymbolSize String
    -- ^ The requested explicit size does not match any ECC200 symbol.
  deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- Symbol size table — ISO\/IEC 16022:2006 Table 7
-- ---------------------------------------------------------------------------
--
-- Every Data Matrix symbol decomposes as:
--
--   symbol = outer_border + (region_rows × region_cols) data regions
--
-- Each data region is (data_region_height × data_region_width) modules of
-- pure data.  Regions are separated by 2-module alignment borders, and the
-- whole symbol is wrapped in a 1-module finder/timing border.
--
-- The Utah placement algorithm scans the *logical* grid — the concatenation
-- of all data region interiors — then we map back to physical coordinates.

-- | One symbol entry from ISO/IEC 16022:2006 Table 7.
data SymbolEntry = SymbolEntry
  { seSymbolRows        :: !Int  -- ^ Total symbol rows including border
  , seSymbolCols        :: !Int  -- ^ Total symbol cols including border
  , seRegionRows        :: !Int  -- ^ Number of data region rows
  , seRegionCols        :: !Int  -- ^ Number of data region cols
  , seDataRegionHeight  :: !Int  -- ^ Data region interior height (excl. borders)
  , seDataRegionWidth   :: !Int  -- ^ Data region interior width (excl. borders)
  , seDataCw            :: !Int  -- ^ Total data codeword capacity
  , seEccCw             :: !Int  -- ^ Total ECC codewords (all blocks combined)
  , seNumBlocks         :: !Int  -- ^ Number of RS blocks
  , seEccPerBlock       :: !Int  -- ^ ECC codewords per block
  } deriving (Show, Eq)

-- | The 24 square symbol sizes from ISO\/IEC 16022:2006, Table 7.
--
-- Fields: symbolRows, symbolCols, regionRows, regionCols,
--         dataRegionHeight, dataRegionWidth, dataCw, eccCw,
--         numBlocks, eccPerBlock
squareSizes :: [SymbolEntry]
squareSizes =
  [ SymbolEntry  10  10 1 1  8  8    3   5  1  5
  , SymbolEntry  12  12 1 1 10 10    5   7  1  7
  , SymbolEntry  14  14 1 1 12 12    8  10  1 10
  , SymbolEntry  16  16 1 1 14 14   12  12  1 12
  , SymbolEntry  18  18 1 1 16 16   18  14  1 14
  , SymbolEntry  20  20 1 1 18 18   22  18  1 18
  , SymbolEntry  22  22 1 1 20 20   30  20  1 20
  , SymbolEntry  24  24 1 1 22 22   36  24  1 24
  , SymbolEntry  26  26 1 1 24 24   44  28  1 28
  , SymbolEntry  32  32 2 2 14 14   62  36  2 18
  , SymbolEntry  36  36 2 2 16 16   86  42  2 21
  , SymbolEntry  40  40 2 2 18 18  114  48  2 24
  , SymbolEntry  44  44 2 2 20 20  144  56  4 14
  , SymbolEntry  48  48 2 2 22 22  174  68  4 17
  , SymbolEntry  52  52 2 2 24 24  204  84  4 21
  , SymbolEntry  64  64 4 4 14 14  280 112  4 28
  , SymbolEntry  72  72 4 4 16 16  368 144  4 36
  , SymbolEntry  80  80 4 4 18 18  456 192  4 48
  , SymbolEntry  88  88 4 4 20 20  576 224  4 56
  , SymbolEntry  96  96 4 4 22 22  696 272  4 68
  , SymbolEntry 104 104 4 4 24 24  816 336  6 56
  , SymbolEntry 120 120 6 6 18 18 1050 408  6 68
  , SymbolEntry 132 132 6 6 20 20 1304 496  8 62
  , SymbolEntry 144 144 6 6 22 22 1558 620 10 62
  ]

-- | The 6 rectangular symbol sizes from ISO\/IEC 16022:2006, Table 7.
rectSizes :: [SymbolEntry]
rectSizes =
  [ SymbolEntry  8 18 1 1  6 16   5  7 1  7
  , SymbolEntry  8 32 1 2  6 14  10 11 1 11
  , SymbolEntry 12 26 1 1 10 24  16 14 1 14
  , SymbolEntry 12 36 1 2 10 16  22 18 1 18
  , SymbolEntry 16 36 1 2 14 16  32 24 1 24
  , SymbolEntry 16 48 1 2 14 22  49 28 1 28
  ]

-- | Maximum data codewords across all symbols (for error messages).
maxDataCw :: Int
maxDataCw = 1558

-- ---------------------------------------------------------------------------
-- GF(256) over 0x12D — Data Matrix field
-- ---------------------------------------------------------------------------
--
-- Data Matrix uses GF(256) with primitive polynomial 0x12D:
--
--     p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301
--
-- IMPORTANT: this is DIFFERENT from QR Code's 0x11D polynomial.  Both are
-- degree-8 irreducible polynomials over GF(2), but do not mix tables between
-- Data Matrix and QR Code.
--
-- We build exp/log tables at module load time for O(1) multiplication.

-- | Build (exp, log) tables for GF(256) over 0x12D.
--
-- Algorithm: start with val=1 (= α⁰). Each step left-shifts val by one bit
-- (multiply by α = x in polynomial form). If bit 8 is set (val ≥ 256),
-- XOR with 0x12D to reduce modulo the primitive polynomial.
buildDmTables :: (V.Vector Int, V.Vector Int)
buildDmTables = (expVec, logVec)
  where
    go :: [Int] -> Int -> Int -> [Int]
    go acc val 255 = acc ++ [val]   -- α^255 = α^0 = 1
    go acc val i =
      let val' = if val `shiftL` 1 >= 0x100
                 then (val `shiftL` 1) `xor` 0x12D
                 else val `shiftL` 1
      in  go (acc ++ [val]) val' (i + 1)

    expList = go [] 1 0
    expVec  = V.fromList (take 256 (expList ++ [0]))

    logVec  = V.replicate 256 0 V.//
                [ (expList !! i, i) | i <- [0 .. 254] ]

-- | Precomputed exp table: dmExp[i] = α^i in GF(256)/0x12D.
dmExp :: V.Vector Int
dmExp = fst buildDmTables

-- | Precomputed log table: dmLog[v] = k such that α^k = v.
dmLog :: V.Vector Int
dmLog = snd buildDmTables

-- | Multiply two GF(256)/0x12D field elements via log/antilog tables.
--
-- For a, b ≠ 0: a × b = α^{(log[a] + log[b]) mod 255}.
-- If either operand is 0, the product is 0 (zero absorbs multiplication).
dmMul :: Int -> Int -> Int
dmMul a b
  | a == 0 || b == 0 = 0
  | otherwise        = dmExp V.! ((dmLog V.! a + dmLog V.! b) `mod` 255)

-- ---------------------------------------------------------------------------
-- RS generator polynomials (GF(256)/0x12D, b=1 convention)
-- ---------------------------------------------------------------------------
--
-- Data Matrix uses the b=1 convention: the RS generator's roots are α¹, α²,
-- …, α^n (not α⁰, α¹, …, α^{n-1} like QR).  This shifts the generator.
--
-- g(x) = (x + α¹)(x + α²) ··· (x + α^{n_ecc})
--
-- We build on demand and store in a simple association list.

-- | Build the RS generator polynomial for n ECC bytes (b=1 convention).
--
-- Starts with g=[1] (degree 0), then multiplies by each linear factor
-- (x + α^i) for i in 1..n.
--
-- Output: highest-degree first, length = n+1.
buildGenerator :: Int -> [Int]
buildGenerator n = foldl' multiplyByLinear [1] [1 .. n]
  where
    -- Multiply polynomial g by (x + α^i).
    -- g*x:   append trailing 0 → [g_k, ..., g_0, 0]
    -- g*α^i: prepend leading 0 → [0, α^i*g_k, ..., α^i*g_0]
    -- XOR (add in GF) the two.
    multiplyByLinear g i =
      let ai   = dmExp V.! i  -- α^i
          gx   = g ++ [0]                      -- g × x
          gai  = 0 : map (dmMul ai) g          -- g × α^i
      in  zipWith xor gx gai

-- ---------------------------------------------------------------------------
-- Reed-Solomon encoding
-- ---------------------------------------------------------------------------

-- | Compute n ECC bytes using LFSR polynomial division.
--
-- Computes the remainder of D(x)·x^n mod G(x) over GF(256)/0x12D.
--
-- Algorithm (LFSR shift register):
--
-- @
-- remainder = [0] * n
-- for each byte b in data:
--     feedback = b XOR remainder[0]
--     remainder = remainder[1:] ++ [0]
--     for i in 0..n-1:
--         remainder[i] ^= generator[i+1] × feedback
-- @
--
-- The generator has n+1 coefficients; we use indices 1..n (skip leading 1).
rsEncode :: [Int]  -- ^ Data bytes
         -> [Int]  -- ^ Generator polynomial (from 'buildGenerator')
         -> [Int]  -- ^ ECC bytes
rsEncode dataBytes gen =
  let n         = length gen - 1
      genCoeffs = drop 1 gen  -- skip leading 1 (monic coefficient)
      initRem   = replicate n 0
      step rem b =
        case rem of
          []     -> []
          (r0:rs) ->
            let feedback = b `xor` r0
                rem'     = rs ++ [0]
            in  if feedback == 0
                then rem'
                else zipWith xor rem' (map (dmMul feedback) genCoeffs)
  in  foldl' step initRem dataBytes

-- ---------------------------------------------------------------------------
-- ASCII data encoding
-- ---------------------------------------------------------------------------

-- | Encode input bytes in Data Matrix ASCII mode.
--
-- ASCII mode rules (ISO\/IEC 16022:2006 §5.2.4):
--
-- * Two consecutive ASCII digits (0x30–0x39) → one codeword =
--   @130 + (d1 × 10 + d2)@.  This digit-pair optimization halves the
--   codeword budget for numeric strings.
--
-- * Single ASCII char (0–127) → one codeword = @ASCII_value + 1@.
--   So 'A' (65) → 66, space (32) → 33.
--
-- * Extended ASCII (128–255) → two codewords: @235@ (UPPER_SHIFT), then
--   @ASCII_value - 127@.
--
-- Examples:
--
-- +-------------+--------------------+------------------------------+
-- | Input       | Codewords          | Why                          |
-- +=============+====================+==============================+
-- | "A"         | [66]               | 65 + 1                       |
-- | " "         | [33]               | 32 + 1                       |
-- | "12"        | [142]              | 130 + 12 (digit pair)        |
-- | "1234"      | [142, 174]         | two digit pairs              |
-- | "00"        | [130]              | 130 + 0                      |
-- | "99"        | [229]              | 130 + 99                     |
-- +-------------+--------------------+------------------------------+
encodeAscii :: [Int]   -- ^ Input bytes (as Int, 0–255)
            -> [Int]   -- ^ Resulting codewords
encodeAscii = go []
  where
    go acc []     = reverse acc
    go acc (b:bs)
      -- Digit pair: both current and next are ASCII digits.
      | b >= 0x30, b <= 0x39, not (null bs), head bs >= 0x30, head bs <= 0x39
          = let d1 = b - 0x30
                d2 = head bs - 0x30
            in  go (130 + d1 * 10 + d2 : acc) (tail bs)
      -- Standard single ASCII character: value + 1.
      | b <= 127  = go (b + 1 : acc) bs
      -- Extended ASCII: UPPER_SHIFT (235) then (value - 127).
      | otherwise = go (b - 127 : 235 : acc) bs

-- ---------------------------------------------------------------------------
-- Pad codewords (ISO\/IEC 16022:2006 §5.2.3)
-- ---------------------------------------------------------------------------

-- | Pad codewords to exactly dataCw bytes using the ECC200 scrambled rule.
--
-- Padding rules:
--
-- 1. The first pad codeword is always the literal value 129 (End of Message).
--
-- 2. Subsequent pads use a /scrambled/ value that depends on 1-indexed
--    position k within the full codeword stream:
--
-- @
-- scrambled = 129 + (149 × k) mod 253 + 1
-- if scrambled > 254: scrambled -= 254
-- @
--
-- The scrambling prevents long runs of 129 from creating degenerate
-- placement patterns in the Utah algorithm.
--
-- Example: "A" (codewords [66]) in 10×10 (dataCw=3):
--
--   k=2: 129                    (first pad — always literal)
--   k=3: 129 + (149*3 mod 253) + 1 = 129 + 194 + 1 = 324 → 324-254 = 70
--   Result: [66, 129, 70]
padCodewords :: [Int]  -- ^ Raw codewords
             -> Int    -- ^ Target data capacity
             -> [Int]
padCodewords cws dataCw = go cws True (length cws + 1)
  where
    go acc _ _  | length acc >= dataCw = take dataCw acc
    go acc True  k = go (acc ++ [129]) False (k + 1)
    go acc False k =
      let s = 129 + (149 * k) `mod` 253 + 1
          s' = if s > 254 then s - 254 else s
      in  go (acc ++ [s']) False (k + 1)

-- ---------------------------------------------------------------------------
-- Symbol selection
-- ---------------------------------------------------------------------------

-- | Find the smallest symbol that can hold n codewords.
selectSymbol :: Int -> SymbolShape -> Either DataMatrixError SymbolEntry
selectSymbol n shape =
  let candidates = case shape of
        Square      -> squareSizes
        Rectangular -> rectSizes
        AnyShape    -> squareSizes ++ rectSizes
      -- Sort by capacity ascending, ties broken by area.
      sorted = sortBy (comparing (\e -> (seDataCw e, seSymbolRows e * seSymbolCols e)))
                 candidates
  in  case filter (\e -> seDataCw e >= n) sorted of
        (e:_) -> Right e
        []    -> Left $ InputTooLong
                  ("data-matrix: input encodes to " ++ show n ++
                   " codewords, maximum is " ++ show maxDataCw ++
                   " (144×144 symbol).")

-- | Find a symbol entry by exact (rows, cols) size.
findEntryBySize :: Int -> Int -> Either DataMatrixError SymbolEntry
findEntryBySize rows cols =
  let all' = squareSizes ++ rectSizes
  in  case filter (\e -> seSymbolRows e == rows && seSymbolCols e == cols) all' of
        (e:_) -> Right e
        []    -> Left $ InvalidSymbolSize
                  ("data-matrix: " ++ show rows ++ "×" ++ show cols ++
                   " is not a valid ECC200 symbol size.")

-- ---------------------------------------------------------------------------
-- Block splitting, ECC computation, and interleaving
-- ---------------------------------------------------------------------------

-- | Split data across RS blocks, compute ECC per block, and interleave.
--
-- Block splitting:
--
-- @
-- base_len     = data_cw / num_blocks   (integer division)
-- extra_blocks = data_cw mod num_blocks
-- Blocks 0..extra_blocks-1   get base_len + 1 data codewords.
-- Blocks extra_blocks..end-1 get base_len     data codewords.
-- @
--
-- Interleaving distributes burst errors across multiple blocks:
--
-- @
-- for pos in 0..max_data_per_block:
--     for blk in 0..num_blocks:
--         emit data[blk][pos] if pos < len(data[blk])
-- for pos in 0..ecc_per_block:
--     for blk in 0..num_blocks:
--         emit ecc[blk][pos]
-- @
computeInterleaved :: [Int] -> SymbolEntry -> [Int]
computeInterleaved dat entry =
  let numBlk      = seNumBlocks entry
      eccPerBlk   = seEccPerBlock entry
      dataCw      = seDataCw entry
      gen         = buildGenerator eccPerBlk
      baseLen     = dataCw `div` numBlk
      extraBlks   = dataCw `mod` numBlk

      -- Split into blocks. Earlier blocks get one extra codeword if needed.
      splitBlocks :: [[Int]] -> [Int] -> Int -> Int -> [[Int]]
      splitBlocks acc _   _ 0 = reverse acc
      splitBlocks acc rem b remaining =
        let l = if b < extraBlks then baseLen + 1 else baseLen
            (blk, rest) = splitAt l rem
        in  splitBlocks (blk : acc) rest (b + 1) (remaining - 1)

      dataBlocks = splitBlocks [] dat 0 numBlk
      eccBlocks  = map (\d -> rsEncode d gen) dataBlocks

      maxDataLen = maximum (map length dataBlocks)

      -- Round-robin data interleaving.
      dataPart = concatMap (\i ->
                   concatMap (\blk ->
                     if i < length blk then [blk !! i] else [])
                   dataBlocks)
                 [0 .. maxDataLen - 1]

      -- Round-robin ECC interleaving.
      eccPart = concatMap (\i ->
                  map (\ecc -> ecc !! i) eccBlocks)
                [0 .. eccPerBlk - 1]

  in  dataPart ++ eccPart

-- ---------------------------------------------------------------------------
-- Grid initialization (outer border + alignment borders)
-- ---------------------------------------------------------------------------

-- | Allocate the physical grid and fill in all fixed structural elements.
--
-- Outer "finder + clock" border:
--
-- * Top row (row 0): alternating dark/light starting dark at col 0 (timing clock).
-- * Right col (col C-1): alternating dark/light starting dark at row 0.
-- * Left col (col 0): all dark (vertical leg of the L-finder).
-- * Bottom row (row R-1): all dark (horizontal leg of the L-finder).
--
-- The L-shaped solid bar tells a scanner where the symbol starts and which
-- orientation it has.  The alternating timing on the opposite two edges
-- distinguishes all four 90° rotations.
--
-- Alignment borders (multi-region symbols):
-- For symbols with region_rows × region_cols > 1, alignment borders separate
-- adjacent data regions.  Each is two modules wide:
--
-- * The first row/col is all dark.
-- * The second row/col is alternating (starts dark at col/row 0).
--
-- Writing order: alignment borders FIRST, then timing, then L-finder last.
-- The L-finder bottom row WINS at intersections.
initGrid :: SymbolEntry -> [[Bool]]
initGrid entry =
  let r = seSymbolRows entry
      c = seSymbolCols entry
      rh = seDataRegionHeight entry
      rw = seDataRegionWidth entry
      rr = seRegionRows entry
      rc = seRegionCols entry

      -- Start with all-False (light) grid.
      base = replicate r (replicate c False)

      -- Apply a list of (row, col, dark) updates to a grid.
      applyUpdates :: [[Bool]] -> [(Int, Int, Bool)] -> [[Bool]]
      applyUpdates g [] = g
      applyUpdates g ((row, col, dark):rest) =
        let row' = take col (g !! row) ++ [dark] ++ drop (col + 1) (g !! row)
            g'   = take row g ++ [row'] ++ drop (row + 1) g
        in  applyUpdates g' rest

      -- Alignment border rows (horizontal separators between region rows).
      abRowUpdates = concatMap (\rri ->
                       let ab0 = 1 + (rri + 1) * rh + rri * 2
                           ab1 = ab0 + 1
                       in  [(ab0, col', True)         | col' <- [0..c-1]] ++
                           [(ab1, col', col' `mod` 2 == 0) | col' <- [0..c-1]]
                     ) [0 .. rr - 2]

      -- Alignment border cols (vertical separators between region cols).
      abColUpdates = concatMap (\rci ->
                       let ab0 = 1 + (rci + 1) * rw + rci * 2
                           ab1 = ab0 + 1
                       in  [(row', ab0, True)         | row' <- [0..r-1]] ++
                           [(row', ab1, row' `mod` 2 == 0) | row' <- [0..r-1]]
                     ) [0 .. rc - 2]

      -- Top row: timing clock, alternating dark/light.
      topRow = [(0, col', col' `mod` 2 == 0) | col' <- [0..c-1]]

      -- Right col: timing clock, alternating dark/light.
      rightCol = [(row', c - 1, row' `mod` 2 == 0) | row' <- [0..r-1]]

      -- Left col: L-finder left leg, all dark.
      leftCol = [(row', 0, True) | row' <- [0..r-1]]

      -- Bottom row: L-finder bottom leg, all dark (written LAST to win).
      botRow = [(r - 1, col', True) | col' <- [0..c-1]]

  in  applyUpdates base
        (abRowUpdates ++ abColUpdates ++ topRow ++ rightCol ++ leftCol ++ botRow)

-- ---------------------------------------------------------------------------
-- Utah placement algorithm
-- ---------------------------------------------------------------------------
--
-- The Utah placement algorithm is the most distinctive part of Data Matrix
-- encoding.  Its name comes from the 8-module codeword shape, which resembles
-- the outline of the US state of Utah — a rectangle with a notch cut from
-- the top-left corner.
--
-- The algorithm scans the *logical* grid (all data region interiors
-- concatenated) in a diagonal zigzag.  For each codeword, 8 bits are placed
-- at 8 fixed offsets relative to the current reference position (row, col).
-- After each codeword, the reference moves diagonally.
--
-- There is NO masking step after placement.  The diagonal traversal
-- naturally distributes bits across the symbol without degenerate clustering.

-- | Apply the boundary wrap rules from ISO\/IEC 16022:2006 Annex F.
--
-- When the standard Utah shape extends beyond the logical grid edge,
-- these rules fold the coordinates back into the valid range.
--
-- The four wrap rules (applied in order):
--
-- 1. row < 0 AND col == 0       → (1, 3)            top-left singularity
-- 2. row < 0 AND col == n_cols  → (0, col-2)         wrapped past right
-- 3. row < 0                    → (row+n_rows, col-4) wrap top→bottom
-- 4. col < 0                    → (row-4, col+n_cols) wrap left→right
applyWrap :: Int -> Int -> Int -> Int -> (Int, Int)
applyWrap row col nRows nCols
  | row < 0 && col == 0     = (1, 3)
  | row < 0 && col == nCols = (0, col - 2)
  | row < 0                 = (row + nRows, col - 4)
  | col < 0                 = (row - 4, col + nCols)
  | otherwise               = (row, col)

-- | Place one codeword using the standard "Utah" 8-module pattern.
--
-- The Utah shape at reference position (row, col):
--
-- @
--              col-2  col-1   col
--
--   row-2 :    .   [bit1]  [bit2]
--   row-1 :  [bit3] [bit4] [bit5]
--   row   :  [bit6] [bit7] [bit8]
-- @
--
-- Bits 1–8 are extracted with bit 8 = MSB (at (row, col)) and bit 1 = LSB
-- (at (row-2, col-1)).
placeUtah :: Int                   -- ^ Codeword (0..255)
          -> Int -> Int            -- ^ Reference position (row, col)
          -> Int -> Int            -- ^ Logical grid size (nRows, nCols)
          -> [[Bool]]              -- ^ Logical grid (mutable-style via rebuild)
          -> [[Bool]]              -- ^ Used map
          -> ([[Bool]], [[Bool]])
placeUtah cw row col nRows nCols grid used =
  let -- (raw_row, raw_col, bit_shift)  — bit_shift 7=MSB, 0=LSB
      rawPlacements =
        [ (row,     col,     7)  -- bit 8 (MSB)
        , (row,     col - 1, 6)  -- bit 7
        , (row,     col - 2, 5)  -- bit 6
        , (row - 1, col,     4)  -- bit 5
        , (row - 1, col - 1, 3)  -- bit 4
        , (row - 1, col - 2, 2)  -- bit 3
        , (row - 2, col,     1)  -- bit 2
        , (row - 2, col - 1, 0)  -- bit 1 (LSB)
        ]
      setAt g r c val =
        let row' = take c (g !! r) ++ [val] ++ drop (c + 1) (g !! r)
        in  take r g ++ [row'] ++ drop (r + 1) g
      applyPlacement (g, u) (rawR, rawC, bit) =
        let (r, c) = applyWrap rawR rawC nRows nCols
        in  if r >= 0 && r < nRows && c >= 0 && c < nCols && not (u !! r !! c)
            then
              let dark = (cw `shiftR` bit) .&. 1 == 1
                  g'   = setAt g r c dark
                  u'   = setAt u r c True
              in  (g', u')
            else (g, u)
  in  foldl' applyPlacement (grid, used) rawPlacements

-- | Place a codeword at explicit (row, col, bit) positions.
placeWithPositions :: Int                       -- ^ Codeword
                   -> [(Int, Int, Int)]         -- ^ (row, col, bit) triples
                   -> Int -> Int                -- ^ Grid size
                   -> [[Bool]] -> [[Bool]]      -- ^ Grid and used map
                   -> ([[Bool]], [[Bool]])
placeWithPositions cw positions nRows nCols grid used =
  let setAt g r c val =
        let row' = take c (g !! r) ++ [val] ++ drop (c + 1) (g !! r)
        in  take r g ++ [row'] ++ drop (r + 1) g
      applyPlacement (g, u) (r, c, bit) =
        if r >= 0 && r < nRows && c >= 0 && c < nCols && not (u !! r !! c)
        then
          let dark = (cw `shiftR` bit) .&. 1 == 1
              g'   = setAt g r c dark
              u'   = setAt u r c True
          in  (g', u')
        else (g, u)
  in  foldl' applyPlacement (grid, used) positions

-- | Corner pattern 1 — triggered at the top-left boundary.
placeCorner1 :: Int -> Int -> Int -> [[Bool]] -> [[Bool]] -> ([[Bool]], [[Bool]])
placeCorner1 cw nRows nCols grid used =
  placeWithPositions cw
    [ (0,          nCols - 2, 7)
    , (0,          nCols - 1, 6)
    , (1,          0,         5)
    , (2,          0,         4)
    , (nRows - 2,  0,         3)
    , (nRows - 1,  0,         2)
    , (nRows - 1,  1,         1)
    , (nRows - 1,  2,         0)
    ] nRows nCols grid used

-- | Corner pattern 2 — triggered at the top-right boundary.
placeCorner2 :: Int -> Int -> Int -> [[Bool]] -> [[Bool]] -> ([[Bool]], [[Bool]])
placeCorner2 cw nRows nCols grid used =
  placeWithPositions cw
    [ (0,          nCols - 2, 7)
    , (0,          nCols - 1, 6)
    , (1,          nCols - 1, 5)
    , (2,          nCols - 1, 4)
    , (nRows - 1,  0,         3)
    , (nRows - 1,  1,         2)
    , (nRows - 1,  2,         1)
    , (nRows - 1,  3,         0)
    ] nRows nCols grid used

-- | Corner pattern 3 — triggered at the bottom-left boundary.
placeCorner3 :: Int -> Int -> Int -> [[Bool]] -> [[Bool]] -> ([[Bool]], [[Bool]])
placeCorner3 cw nRows nCols grid used =
  placeWithPositions cw
    [ (0,          nCols - 1, 7)
    , (1,          0,         6)
    , (2,          0,         5)
    , (nRows - 2,  0,         4)
    , (nRows - 1,  0,         3)
    , (nRows - 1,  1,         2)
    , (nRows - 1,  2,         1)
    , (nRows - 1,  3,         0)
    ] nRows nCols grid used

-- | Corner pattern 4 — triggered for nCols mod 8 == 0.
placeCorner4 :: Int -> Int -> Int -> [[Bool]] -> [[Bool]] -> ([[Bool]], [[Bool]])
placeCorner4 cw nRows nCols grid used =
  placeWithPositions cw
    [ (nRows - 3, nCols - 1, 7)
    , (nRows - 2, nCols - 1, 6)
    , (nRows - 1, nCols - 3, 5)
    , (nRows - 1, nCols - 2, 4)
    , (nRows - 1, nCols - 1, 3)
    , (0,         0,         2)
    , (1,         0,         1)
    , (2,         0,         0)
    ] nRows nCols grid used

-- | Run the Utah diagonal placement algorithm on the logical data matrix.
--
-- The reference position (row, col) starts at (4, 0) and zigzags diagonally.
-- Each outer-loop iteration has two legs:
--
-- 1. __Upward-right leg__: place codewords, then move row -= 2, col += 2
--    until out of bounds.  Then step to next diagonal start: row += 1, col += 3.
--
-- 2. __Downward-left leg__: place codewords, then move row += 2, col -= 2
--    until out of bounds.  Then step to next diagonal start: row += 3, col += 1.
--
-- Between legs, four corner patterns fire when the reference matches
-- specific trigger conditions.
--
-- Termination: when both row >= nRows and col >= nCols, all codewords have been
-- visited.  Any unvisited modules get the fill pattern (r+c) mod 2 == 1 (dark).
utahPlacement :: [Int] -> Int -> Int -> [[Bool]]
utahPlacement codewords nRows nCols =
  let initGrid' = replicate nRows (replicate nCols False)
      initUsed  = replicate nRows (replicate nCols False)

      setAt :: [[Bool]] -> Int -> Int -> Bool -> [[Bool]]
      setAt g r c val =
        let row' = take c (g !! r) ++ [val] ++ drop (c + 1) (g !! r)
        in  take r g ++ [row'] ++ drop (r + 1) g

      -- Main loop state: (row, col, cw_idx, grid, used)
      outerLoop :: Int -> Int -> Int -> [[Bool]] -> [[Bool]] -> [[Bool]]
      outerLoop row col cwIdx g u
        | row >= nRows && col >= nCols = finish g u
        | cwIdx >= length codewords    = finish g u
        | otherwise =
            -- Corner special cases.
            let (g1, u1, cwIdx1) = maybeCorner1 row col cwIdx g u
                (g2, u2, cwIdx2) = maybeCorner2 row col cwIdx1 g1 u1
                (g3, u3, cwIdx3) = maybeCorner3 row col cwIdx2 g2 u2
                (g4, u4, cwIdx4) = maybeCorner4 row col cwIdx3 g3 u3

                -- Upward-right leg: row -= 2, col += 2 until out of bounds.
                (g5, u5, cwIdx5, row5, col5) = upLeg row col cwIdx4 g4 u4
                row6 = row5 + 1
                col6 = col5 + 3

                -- Downward-left leg: row += 2, col -= 2 until out of bounds.
                (g7, u7, cwIdx7, row7, col7) = downLeg row6 col6 cwIdx5 g5 u5
                row8 = row7 + 3
                col8 = col7 + 1

            in  outerLoop row8 col8 cwIdx7 g7 u7

      -- Corner 1: (nRows, 0) when nRows or nCols ≡ 0 (mod 4).
      maybeCorner1 row col cwIdx g u
        | row == nRows && col == 0 && (nRows `mod` 4 == 0 || nCols `mod` 4 == 0)
        , cwIdx < length codewords
            = let (g', u') = placeCorner1 (codewords !! cwIdx) nRows nCols g u
              in  (g', u', cwIdx + 1)
        | otherwise = (g, u, cwIdx)

      -- Corner 2: (nRows-2, 0) when nCols mod 4 ≠ 0.
      maybeCorner2 row col cwIdx g u
        | row == nRows - 2 && col == 0 && nCols `mod` 4 /= 0
        , cwIdx < length codewords
            = let (g', u') = placeCorner2 (codewords !! cwIdx) nRows nCols g u
              in  (g', u', cwIdx + 1)
        | otherwise = (g, u, cwIdx)

      -- Corner 3: (nRows-2, 0) when nCols mod 8 == 4.
      maybeCorner3 row col cwIdx g u
        | row == nRows - 2 && col == 0 && nCols `mod` 8 == 4
        , cwIdx < length codewords
            = let (g', u') = placeCorner3 (codewords !! cwIdx) nRows nCols g u
              in  (g', u', cwIdx + 1)
        | otherwise = (g, u, cwIdx)

      -- Corner 4: (nRows+4, 2) when nCols mod 8 == 0.
      maybeCorner4 row col cwIdx g u
        | row == nRows + 4 && col == 2 && nCols `mod` 8 == 0
        , cwIdx < length codewords
            = let (g', u') = placeCorner4 (codewords !! cwIdx) nRows nCols g u
              in  (g', u', cwIdx + 1)
        | otherwise = (g, u, cwIdx)

      -- Upward-right diagonal: row decreases, col increases.
      -- Returns (grid, used, cwIdx, row_oob, col_oob) where row_oob/col_oob is
      -- the FIRST out-of-bounds position (matching the Python reference convention).
      -- The caller then computes the next diagonal start as row_oob+1, col_oob+3.
      upLeg :: Int -> Int -> Int -> [[Bool]] -> [[Bool]] -> ([[Bool]], [[Bool]], Int, Int, Int)
      upLeg row col cwIdx g u
        | row < 0 || col >= nCols = (g, u, cwIdx, row, col)  -- return oob position
        | otherwise =
            let (g', u', cwIdx') =
                  if row >= 0 && row < nRows && col >= 0 && col < nCols
                     && not (u !! row !! col) && cwIdx < length codewords
                  then let (g'', u'') = placeUtah (codewords !! cwIdx) row col nRows nCols g u
                       in  (g'', u'', cwIdx + 1)
                  else (g, u, cwIdx)
            in  upLeg (row - 2) (col + 2) cwIdx' g' u'

      -- Downward-left diagonal: row increases, col decreases.
      -- Returns the FIRST out-of-bounds position.
      -- The caller then computes the next diagonal start as row_oob+3, col_oob+1.
      downLeg :: Int -> Int -> Int -> [[Bool]] -> [[Bool]] -> ([[Bool]], [[Bool]], Int, Int, Int)
      downLeg row col cwIdx g u
        | row >= nRows || col < 0 = (g, u, cwIdx, row, col)  -- return oob position
        | otherwise =
            let (g', u', cwIdx') =
                  if row >= 0 && row < nRows && col >= 0 && col < nCols
                     && not (u !! row !! col) && cwIdx < length codewords
                  then let (g'', u'') = placeUtah (codewords !! cwIdx) row col nRows nCols g u
                       in  (g'', u'', cwIdx + 1)
                  else (g, u, cwIdx)
            in  downLeg (row + 2) (col - 2) cwIdx' g' u'

      -- Fill remaining unset modules with (r+c) mod 2 == 1 (dark).
      finish :: [[Bool]] -> [[Bool]] -> [[Bool]]
      finish g u =
        foldl' (\g' (r, c) -> setAt g' r c ((r + c) `mod` 2 == 1))
          g
          [ (r, c) | r <- [0..nRows-1], c <- [0..nCols-1], not (u !! r !! c) ]

  in  outerLoop 4 0 0 initGrid' initUsed

-- ---------------------------------------------------------------------------
-- Logical → physical coordinate mapping
-- ---------------------------------------------------------------------------

-- | Map a logical data-matrix coordinate to its physical symbol coordinate.
--
-- The logical data matrix is the concatenation of all data region interiors.
-- Utah placement works in this logical space.  After placement we map back
-- to the physical grid, which adds:
--
-- * 1-module outer border (finder + timing) on all four sides.
-- * 2-module alignment border between adjacent data regions.
--
-- For a symbol with region_rows × region_cols data regions, each of size rh × rw:
--
-- @
-- phys_row = (r / rh) * (rh + 2) + (r mod rh) + 1
-- phys_col = (c / rw) * (rw + 2) + (c mod rw) + 1
-- @
--
-- For single-region symbols (1 × 1) this simplifies to phys_row = r+1, phys_col = c+1.
logicalToPhysical :: Int -> Int -> SymbolEntry -> (Int, Int)
logicalToPhysical r c entry =
  let rh = seDataRegionHeight entry
      rw = seDataRegionWidth entry
      physRow = (r `div` rh) * (rh + 2) + (r `mod` rh) + 1
      physCol = (c `div` rw) * (rw + 2) + (c `mod` rw) + 1
  in  (physRow, physCol)

-- ---------------------------------------------------------------------------
-- Core encode function
-- ---------------------------------------------------------------------------

-- | Encode a string into a Data Matrix ECC200 'ModuleGrid'.
--
-- Automatically selects the smallest symbol that fits the input.
-- Returns @Left DataMatrixError@ if the input is too long.
--
-- == Example
--
-- @
-- case encode "A" defaultOptions of
--   Left err   -> putStrLn ("Error: " ++ show err)
--   Right grid -> print (mgRows grid)  -- 10 (10×10 symbol)
-- @
encode :: String                         -- ^ Input string (ASCII/UTF-8)
       -> DataMatrixOptions              -- ^ Encoder options
       -> Either DataMatrixError ModuleGrid
encode input opts = do
  let inputBytes = map ord input
  let cws        = encodeAscii inputBytes
  entry <- selectSymbol (length cws) (dmShape opts)
  encodeWithEntry input entry

-- | Encode a string to a specific symbol size.
--
-- Raises 'InvalidSymbolSize' if the size is not one of the 30 ECC200 sizes.
-- Raises 'InputTooLong' if the input does not fit.
encodeAt :: String       -- ^ Input string
         -> Int          -- ^ Symbol rows
         -> Int          -- ^ Symbol cols
         -> Either DataMatrixError ModuleGrid
encodeAt input rows cols = do
  entry <- findEntryBySize rows cols
  let inputBytes = map ord input
      cws        = encodeAscii inputBytes
  if length cws > seDataCw entry
    then Left $ InputTooLong
           ("data-matrix: input encodes to " ++ show (length cws) ++
            " codewords but " ++ show rows ++ "×" ++ show cols ++
            " symbol holds only " ++ show (seDataCw entry) ++ ".")
    else encodeWithEntry input entry

-- | Internal: encode using a pre-selected SymbolEntry.
encodeWithEntry :: String -> SymbolEntry -> Either DataMatrixError ModuleGrid
encodeWithEntry input entry =
  let inputBytes  = map ord input
      cws         = encodeAscii inputBytes

      -- Step 1: Pad to data capacity with the ECC200 scrambled-pad sequence.
      padded      = padCodewords cws (seDataCw entry)

      -- Step 2: Compute ECC and interleave blocks.
      interleaved = computeInterleaved padded entry

      -- Step 3: Initialize physical grid with finder + timing + alignment.
      physGrid    = initGrid entry

      -- Step 4: Run Utah placement on the logical grid.
      nRows       = seRegionRows entry * seDataRegionHeight entry
      nCols       = seRegionCols entry * seDataRegionWidth entry
      logGrid     = utahPlacement interleaved nRows nCols

      -- Step 5: Map logical → physical coordinates.
      physUpdates = [ (pr, pc, logGrid !! r !! c)
                    | r <- [0..nRows-1]
                    , c <- [0..nCols-1]
                    , let (pr, pc) = logicalToPhysical r c entry ]

      applyUp g (r, c, dark) =
        let row = g !! r
            row' = take c row ++ [dark] ++ drop (c + 1) row
        in  take r g ++ [row'] ++ drop (r + 1) g

      finalPhys = foldl' applyUp physGrid physUpdates

      -- Step 6: Build immutable ModuleGrid.
      sRows = seSymbolRows entry
      sCols = seSymbolCols entry
      mgrid = foldl' (\g (r, c) ->
                  if finalPhys !! r !! c then setModule g r c True else g)
                (emptyGrid sRows sCols CodingAdventures.Barcode2D.Square)
                [(r, c) | r <- [0..sRows-1], c <- [0..sCols-1]]

  in  Right mgrid

-- | Encode a string and convert to a @PaintScene@ in one call.
--
-- Convenience function combining 'encode' and barcode-2d's 'layout'.
-- The concrete return type is @Either DataMatrixError PaintScene@ where
-- @PaintScene@ is from the @paint-instructions@ package.  No explicit type
-- signature is given here so that callers do not need to import
-- @paint-instructions@ directly; GHC infers the type from 'layout'.
encodeAndLayout input opts cfg = do
  grid <- encode input opts
  return (layout grid cfg)
