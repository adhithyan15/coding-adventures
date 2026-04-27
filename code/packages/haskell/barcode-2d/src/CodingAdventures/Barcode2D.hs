-- | CodingAdventures.Barcode2D — Shared 2D barcode abstraction layer (BC00).
--
-- == Overview
--
-- This module sits between barcode encoders and the paint backend.  Encoders
-- produce a 'ModuleGrid'; this module converts that grid into a 'PaintScene'
-- for rendering.
--
-- @
-- Input data
--   → format encoder (qr-code, data-matrix, aztec…)
--   → 'ModuleGrid'           ← produced by the encoder
--   → 'layout'               ← THIS MODULE converts to pixel instructions
--   → 'PaintScene'           ← consumed by paint-vm (P2D01)
--   → backend (SVG, Canvas, Metal, terminal…)
-- @
--
-- == Module units vs. pixels
--
-- All coordinates /before/ 'layout' are in "module units" — abstract grid
-- steps.  Only 'layout' multiplies by 'moduleSizePx' to produce real pixel
-- coordinates.  Encoders never need to know about screen resolution.
--
-- == Supported module shapes
--
-- * __Square__ (default) — used by QR Code, Data Matrix, Aztec Code, PDF417.
--   Each module becomes a 'PaintRect'.
--
-- * __Hex__ (flat-top hexagons) — used by MaxiCode (ISO\/IEC 16023).
--   Each module becomes a 'PaintPath' tracing six vertices.
--   Odd-numbered rows are offset by half a hexagon width to produce the
--   standard hexagonal tiling.
--
-- == Example
--
-- @
-- import CodingAdventures.Barcode2D
-- import CodingAdventures.PaintInstructions (psWidth, psHeight, psInstructions)
--
-- -- Create a blank 5×5 square grid and set a checkerboard pattern.
-- myGrid :: ModuleGrid
-- myGrid = foldr
--   (\\(r, c, v) g -> setModule g r c v) (emptyGrid 5 5 Square)
--   [ (0,0,True), (0,2,True), (0,4,True)
--   , (1,1,True), (1,3,True)
--   , (2,0,True), (2,2,True), (2,4,True)
--   ]
--
-- scene :: PaintScene
-- scene = layout myGrid defaultConfig
-- -- scene width  = (5 + 2*4) * 10 = 130 px
-- -- scene height = (5 + 2*4) * 10 = 130 px
-- @
module CodingAdventures.Barcode2D
  ( -- * Module shape
    ModuleShape (..)

    -- * ModuleGrid
  , ModuleGrid (..)
  , emptyGrid
  , setModule

    -- * Layout configuration
  , Barcode2DLayoutConfig (..)
  , defaultConfig

    -- * Layout functions
  , layout
  , layoutSquare
  , layoutHex
  ) where

import qualified Data.Vector as V
import CodingAdventures.PaintInstructions
  ( PaintScene (..)
  , PathCommand (..)
  , emptyScene
  , makeRect
  , makePath
  )
import qualified Data.Map.Strict as Map

-- ---------------------------------------------------------------------------
-- ModuleShape
-- ---------------------------------------------------------------------------

-- | The shape used to render each module (grid cell).
--
-- The choice of shape is a property of the barcode format:
--
-- * QR Code, Data Matrix, Aztec Code, PDF417 all use __Square__.
-- * MaxiCode (ISO\/IEC 16023) uses __Hex__ (flat-top hexagons).
--
-- 'ModuleShape' is stored in 'ModuleGrid' so that 'layout' can automatically
-- pick the correct rendering path without the caller having to specify it
-- a second time.
data ModuleShape
  = Square
    -- ^ Each module renders as a filled square.  The overwhelmingly common case.
  | Hex
    -- ^ Each module renders as a flat-top regular hexagon.
    --   Odd-numbered rows are offset right by half a hexagon width.
  deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- ModuleGrid
-- ---------------------------------------------------------------------------

-- | The universal intermediate representation produced by every 2D barcode
-- encoder.
--
-- A module grid is a 2D boolean array:
--
-- @
--   True  — dark module (ink / filled)
--   False — light module (background / empty)
-- @
--
-- Row 0 is the top row. Column 0 is the leftmost column.  This matches the
-- natural reading order in every 2D barcode standard.
--
-- The modules are stored as a flat 'V.Vector' 'Bool' in row-major order:
--
-- @
--   index(row, col) = row * mgCols + col
-- @
--
-- This layout gives O(1) random access.  Use 'setModule' for copy-on-write
-- updates — it builds a new vector with the single changed element.
--
-- === MaxiCode fixed size
--
-- MaxiCode grids are always 33 rows × 30 columns with @'moduleShape' = 'Hex'@.
-- Physical MaxiCode symbols are approximately 1 inch × 1 inch.
--
-- === Immutability
--
-- 'ModuleGrid' is an ordinary Haskell value.  'setModule' produces a new
-- grid without touching the original.  Encoders can branch freely, try
-- different mask patterns, compare scores, and discard inferior versions.
data ModuleGrid = ModuleGrid
  { mgRows    :: Int
    -- ^ Number of rows (height of the grid).
  , mgCols    :: Int
    -- ^ Number of columns (width of the grid).
  , mgModules :: V.Vector Bool
    -- ^ Flat row-major storage: @mgModules V.! (row * mgCols + col)@.
    --   Length = mgRows * mgCols.
  , mgShape   :: ModuleShape
    -- ^ Shape to use when rendering modules.
  } deriving (Show, Eq)

-- | Create a new 'ModuleGrid' with every module set to @False@ (light).
--
-- This is the starting point for every 2D barcode encoder.  The encoder then
-- calls 'setModule' to paint dark modules one at a time as it places finder
-- patterns, timing strips, data bits, and error-correction bits.
--
-- === Example — start a 21×21 QR Code v1 grid
--
-- @
-- let grid = emptyGrid 21 21 Square
-- -- grid is all False; use setModule to paint dark modules
-- @
emptyGrid
  :: Int          -- ^ Number of rows
  -> Int          -- ^ Number of columns
  -> ModuleShape  -- ^ Module shape
  -> ModuleGrid
emptyGrid rows cols shape = ModuleGrid
  { mgRows    = rows
  , mgCols    = cols
  , mgModules = V.replicate (rows * cols) False
  , mgShape   = shape
  }

-- | Return a new 'ModuleGrid' identical to the input except that the module
-- at @(row, col)@ is set to @dark@.
--
-- This function is __pure and immutable__ — the input grid is never modified.
-- Only the single element at the target index is changed; the rest of the
-- vector is shared via copy-on-write.
--
-- === Why immutability matters
--
-- QR encoders try all eight mask patterns and score each one.  With immutable
-- grids, keeping the pre-mask grid is trivial: just hold a reference to it.
-- No undo stack or deep copy needed.
--
-- === Out of bounds
--
-- Calling 'setModule' with a @row@ or @col@ outside the grid dimensions
-- raises an 'error'.  This is a programming error in the encoder, not a
-- user-facing error.
--
-- === Example
--
-- @
-- let g  = emptyGrid 3 3 Square
-- let g2 = setModule g 1 1 True
-- -- g  unchanged: (1,1) is still False
-- -- g2 new value: (1,1) is True
-- @
setModule
  :: ModuleGrid  -- ^ Original grid
  -> Int         -- ^ Row index (0-based)
  -> Int         -- ^ Column index (0-based)
  -> Bool        -- ^ True = dark, False = light
  -> ModuleGrid
setModule grid row col dark
  | row < 0 || row >= mgRows grid =
      error $ "Barcode2D.setModule: row " ++ show row ++
              " out of range [0, " ++ show (mgRows grid - 1) ++ "]"
  | col < 0 || col >= mgCols grid =
      error $ "Barcode2D.setModule: col " ++ show col ++
              " out of range [0, " ++ show (mgCols grid - 1) ++ "]"
  | otherwise =
      let idx = row * mgCols grid + col
      in  grid { mgModules = mgModules grid V.// [(idx, dark)] }

-- ---------------------------------------------------------------------------
-- Barcode2DLayoutConfig
-- ---------------------------------------------------------------------------

-- | Configuration for 'layout'.
--
-- All fields are required. Use 'defaultConfig' as a starting point and
-- override only what you need using record update syntax:
--
-- @
-- let cfg = defaultConfig { moduleSizePx = 20, quietZoneModules = 5 }
-- @
--
-- === moduleSizePx
--
-- The size of one module in pixels.
--
-- * Square: both width and height of the square.
-- * Hex: the flat-to-flat width of the hexagon (also equal to the side length
--   for a regular hexagon when using the formula below).
--
-- Must be > 0.
--
-- === quietZoneModules
--
-- The number of module-width quiet-zone units added to each side of the grid.
--
-- * QR Code requires at minimum 4 modules of quiet zone per ISO\/IEC 18004.
-- * Data Matrix requires 1 module. MaxiCode requires 1 module.
--
-- Must be ≥ 0.
--
-- === foreground / background
--
-- CSS color strings for dark modules and the background (quiet zone +
-- light modules).  Standard barcodes use black on white.
data Barcode2DLayoutConfig = Barcode2DLayoutConfig
  { moduleSizePx     :: Int
    -- ^ Pixels per module. Must be > 0. Default: 10.
  , quietZoneModules :: Int
    -- ^ Quiet zone width in modules on each side. Must be ≥ 0. Default: 4.
  , foreground       :: String
    -- ^ CSS color for dark modules. Default: @\"#000000\"@.
  , background       :: String
    -- ^ CSS color for background and quiet zone. Default: @\"#ffffff\"@.
  } deriving (Show, Eq)

-- | Sensible defaults for 'layout'.
--
-- | Field              | Default     | Rationale                                |
-- |--------------------|-------------|------------------------------------------|
-- | moduleSizePx       | 10          | 21×21 QR v1 renders at 290×290 px        |
-- | quietZoneModules   | 4           | QR Code minimum per ISO\/IEC 18004       |
-- | foreground         | @\"#000000\"@  | Black ink on white paper               |
-- | background         | @\"#ffffff\"@  | White paper                            |
defaultConfig :: Barcode2DLayoutConfig
defaultConfig = Barcode2DLayoutConfig
  { moduleSizePx     = 10
  , quietZoneModules = 4
  , foreground       = "#000000"
  , background       = "#ffffff"
  }

-- ---------------------------------------------------------------------------
-- layout — public entry point
-- ---------------------------------------------------------------------------

-- | Convert a 'ModuleGrid' into a 'PaintScene' ready for the PaintVM.
--
-- This is the __only__ function in the entire 2D barcode stack that knows
-- about pixels.  Everything above this step works in abstract module units.
-- Everything below is handled by the paint backend.
--
-- The function dispatches to 'layoutSquare' or 'layoutHex' depending on the
-- grid's 'ModuleShape'.
--
-- === Validation
--
-- Calls 'error' if:
--
-- * @moduleSizePx cfg <= 0@
-- * @quietZoneModules cfg < 0@
--
-- === Square layout formula
--
-- @
-- quietZonePx = quietZoneModules * moduleSizePx
-- totalWidth  = (cols + 2 * quietZoneModules) * moduleSizePx
-- totalHeight = (rows + 2 * quietZoneModules) * moduleSizePx
-- x(col)      = quietZonePx + col * moduleSizePx
-- y(row)      = quietZonePx + row * moduleSizePx
-- @
--
-- === Hex layout formula (flat-top hexagons)
--
-- @
-- hexWidth   = moduleSizePx
-- hexHeight  = moduleSizePx * (sqrt 3 / 2)   -- row step
-- circumR    = moduleSizePx / sqrt 3          -- center to vertex
-- cx(row,col)= quietZonePx + col*hexWidth + (row `mod` 2) * (hexWidth/2)
-- cy(row)    = quietZonePx + row * hexHeight
-- @
--
-- Odd rows are shifted right by @hexWidth\/2@ to create the standard
-- hexagonal tiling used by MaxiCode.
layout :: ModuleGrid -> Barcode2DLayoutConfig -> PaintScene
layout grid cfg
  | moduleSizePx cfg <= 0     =
      error $ "Barcode2D.layout: moduleSizePx must be > 0, got "
              ++ show (moduleSizePx cfg)
  | quietZoneModules cfg < 0 =
      error $ "Barcode2D.layout: quietZoneModules must be >= 0, got "
              ++ show (quietZoneModules cfg)
  | otherwise = case mgShape grid of
      Square -> layoutSquare grid cfg
      Hex    -> layoutHex    grid cfg

-- ---------------------------------------------------------------------------
-- layoutSquare
-- ---------------------------------------------------------------------------

-- | Render a square-module 'ModuleGrid' into a 'PaintScene'.
--
-- Called by 'layout' after validation.  The algorithm is:
--
-- 1. Compute total pixel dimensions (grid + quiet zone on all four sides).
-- 2. Emit one background 'PaintRect' covering the entire symbol.
-- 3. For each dark module, emit one filled 'PaintRect'.
--
-- Light modules are implicitly covered by the background rect.  Only dark
-- modules produce explicit instructions, so the instruction count is
-- proportional to the number of dark modules rather than the total grid area.
--
-- @
-- ┌──────────────────────────────────┐
-- │  quiet zone (all white)          │
-- │   ┌──────────────────┐           │
-- │   │  ■ □ ■ □ ■       │           │
-- │   │  □ ■ □ ■ □       │           │
-- │   │  ■ □ ■ □ ■       │           │
-- │   └──────────────────┘           │
-- │  quiet zone (all white)          │
-- └──────────────────────────────────┘
-- @
layoutSquare :: ModuleGrid -> Barcode2DLayoutConfig -> PaintScene
layoutSquare grid cfg =
  let sz          = fromIntegral (moduleSizePx cfg)     :: Double
      qz          = fromIntegral (quietZoneModules cfg)  :: Double
      quietZonePx = qz * sz
      totalW      = (fromIntegral (mgCols grid) + 2 * qz) * sz
      totalH      = (fromIntegral (mgRows grid) + 2 * qz) * sz

      -- Background fills the whole symbol including quiet zone.
      bgRect = makeRect 0 0 totalW totalH (background cfg)

      -- One rect per dark module.
      darkRects = [ makeRect (quietZonePx + fromIntegral col * sz)
                             (quietZonePx + fromIntegral row * sz)
                             sz sz
                             (foreground cfg)
                  | row <- [0 .. mgRows grid - 1]
                  , col <- [0 .. mgCols grid - 1]
                  , mgModules grid V.! (row * mgCols grid + col)
                  ]

      scene = emptyScene totalW totalH (background cfg)
  in  scene { psInstructions = bgRect : darkRects
            , psMeta         = Map.empty }

-- ---------------------------------------------------------------------------
-- layoutHex
-- ---------------------------------------------------------------------------

-- | Render a hex-module 'ModuleGrid' into a 'PaintScene'.
--
-- Used for MaxiCode (ISO\/IEC 16023), which uses flat-top hexagons in an
-- offset-row grid.  Odd rows are shifted right by half a hexagon width to
-- create the standard hexagonal tiling.
--
-- === Flat-top hexagon geometry
--
-- A "flat-top" hexagon has horizontal flat edges at top and bottom:
--
-- @
--    ___
--   /   \   ← flat top
--  |     |
--   \___/   ← flat bottom
-- @
--
-- For a flat-top hexagon centered at @(cx, cy)@ with circumradius @R@
-- (center to vertex distance):
--
-- @
-- vertex i = ( cx + R * cos(i * 60°),
--              cy + R * sin(i * 60°) )   for i = 0..5
--
-- i=0:  right midpoint     (angle   0°)
-- i=1:  bottom-right       (angle  60°)
-- i=2:  bottom-left        (angle 120°)
-- i=3:  left midpoint      (angle 180°)
-- i=4:  top-left           (angle 240°)
-- i=5:  top-right          (angle 300°)
-- @
--
-- === Tiling formula
--
-- @
-- hexWidth  = moduleSizePx
-- hexHeight = moduleSizePx * (sqrt 3 / 2)   -- vertical row step
-- circumR   = moduleSizePx / sqrt 3
-- cx        = quietZonePx + col*hexWidth + (row `mod` 2) * (hexWidth/2)
-- cy        = quietZonePx + row*hexHeight
-- @
layoutHex :: ModuleGrid -> Barcode2DLayoutConfig -> PaintScene
layoutHex grid cfg =
  let sz          = fromIntegral (moduleSizePx cfg) :: Double
      qz          = fromIntegral (quietZoneModules cfg) :: Double
      quietZonePx = qz * sz

      hexWidth  = sz
      hexHeight = sz * (sqrt 3 / 2)
      circumR   = sz / sqrt 3

      -- Total canvas. The extra hexWidth/2 prevents odd-row hexagons from
      -- clipping outside the right edge.
      totalW = (fromIntegral (mgCols grid) + 2 * qz) * hexWidth + hexWidth / 2
      totalH = (fromIntegral (mgRows grid) + 2 * qz) * hexHeight

      bgRect = makeRect 0 0 totalW totalH (background cfg)

      hexPaths =
        [ let cx = quietZonePx + fromIntegral col * hexWidth
                   + (if odd row then hexWidth / 2 else 0)
              cy = quietZonePx + fromIntegral row * hexHeight
          in  makePath (buildHexPath cx cy circumR) (foreground cfg)
        | row <- [0 .. mgRows grid - 1]
        , col <- [0 .. mgCols grid - 1]
        , mgModules grid V.! (row * mgCols grid + col)
        ]

      scene = emptyScene totalW totalH (background cfg)
  in  scene { psInstructions = bgRect : hexPaths
            , psMeta         = Map.empty }

-- ---------------------------------------------------------------------------
-- buildHexPath — geometry helper
-- ---------------------------------------------------------------------------

-- | Build the six 'PathCommand' steps for a flat-top regular hexagon.
--
-- The six vertices are at angles 0°, 60°, 120°, 180°, 240°, 300° from the
-- center @(cx, cy)@ at circumradius @circumR@:
--
-- @
-- vertex i = ( cx + circumR * cos(i * 60° in radians)
--            , cy + circumR * sin(i * 60° in radians) )
-- @
--
-- The resulting path is: MoveTo v0, LineTo v1..v5, ClosePath.
buildHexPath :: Double -> Double -> Double -> [PathCommand]
buildHexPath cx cy circumR =
  let degToRad d = d * pi / 180.0
      vertex i   = let angle = degToRad (fromIntegral i * 60.0)
                   in  ( cx + circumR * cos angle
                       , cy + circumR * sin angle )
      (x0, y0)   = vertex (0 :: Int)
      rest       = [ let (x, y) = vertex i in LineTo x y | i <- [1..5 :: Int] ]
  in  MoveTo x0 y0 : rest ++ [ClosePath]
