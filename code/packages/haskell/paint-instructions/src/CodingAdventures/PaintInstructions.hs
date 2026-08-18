-- | CodingAdventures.PaintInstructions — Universal 2D paint IR (P2D00).
--
-- == Overview
--
-- This module is the shared vocabulary between producers and backends in a
-- composable 2D painting pipeline.
--
-- @
-- Producer (chart, barcode, diagram)
--   → PaintScene / [PaintInstruction]   ← this module
--   → PaintVM (P2D01)
--   → Backend (SVG, Canvas, Metal, terminal)
-- @
--
-- Everything in this module is pure data — no IO, no side effects.
-- A 'PaintScene' is a simple Haskell value you can pass around, inspect,
-- transform, and eventually hand to a VM.
--
-- == Core Concepts
--
-- * 'PathCommand' — one step for an imaginary pen plotter (move, line, close).
-- * 'PaintInstruction' — a single drawing command: a rect, a path, etc.
-- * 'PaintScene' — the top-level container with dimensions, background,
--   and an ordered list of instructions (painted back-to-front).
--
-- == Example
--
-- @
-- import CodingAdventures.PaintInstructions
-- import qualified Data.Map.Strict as Map
--
-- -- A 200×100 white scene with one blue rectangle.
-- example :: PaintScene
-- example = PaintScene
--   { psWidth        = 200
--   , psHeight       = 100
--   , psBg           = "#ffffff"
--   , psInstructions =
--       [ PaintRect { prX = 10, prY = 10, prW = 80, prH = 40
--                   , prFill = "#2563eb", prMeta = Map.empty }
--       ]
--   , psMeta = Map.empty
--   }
-- @
module CodingAdventures.PaintInstructions
  ( -- * PathCommand
    PathCommand (..)

    -- * Transform2D
  , Transform2D (..)
  , identityTransform

    -- * PaintGlyphPlacement
  , PaintGlyphPlacement (..)

    -- * PaintInstruction
  , PaintInstruction (..)

    -- * PaintScene
  , PaintScene (..)

    -- * Builder helpers
  , emptyScene
  , makeRect
  , makePath
  , makeGlyphRun
  , makeLine
  , makeGroup
  , makeClip
  , makeLayer
  , addInstruction
  ) where

import Data.Aeson  (Value)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

-- ---------------------------------------------------------------------------
-- PathCommand
-- ---------------------------------------------------------------------------

-- | A single drawing command inside a 'PaintInstruction' path.
--
-- Think of it as an instruction to a pen plotter:
--
-- @
--   MoveTo x y  — lift the pen and move to (x, y) without drawing
--   LineTo x y  — press the pen down and draw a straight line to (x, y)
--   ClosePath   — draw a straight line back to the last MoveTo, closing the shape
-- @
--
-- Example — an equilateral triangle with vertices at (50,10), (90,80), (10,80):
--
-- @
--   [ MoveTo 50 10
--   , LineTo 90 80
--   , LineTo 10 80
--   , ClosePath
--   ]
-- @
--
-- This minimal command set is sufficient for all 2D barcode shapes.
-- For curved paths (Bézier, arc), extend this type.
data PathCommand
  = MoveTo Double Double
    -- ^ Lift the pen and position it at (x, y).
    --   Starts a new sub-path.  Does not draw a line.
  | LineTo Double Double
    -- ^ Draw a straight line from the current pen position to (x, y).
    --   The pen moves to (x, y) after the command.
  | ClosePath
    -- ^ Draw a straight line from the current position back to the
    --   last 'MoveTo' point, closing the current sub-path.
  deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- PaintInstruction
-- ---------------------------------------------------------------------------

-- | A single drawing operation.
--
-- The 'PaintScene' holds an ordered list of these.  Instructions are
-- rendered back-to-front (painter's algorithm): the first instruction is
-- drawn first, and later instructions can cover earlier ones.
--
-- == Instruction types
--
-- === PaintRect
--
-- A filled rectangle.
--
-- @
--   ┌──────────────────────┐
--   │  x,y = top-left      │
--   │  w = width           │ ← height h
--   │  h = height          │
--   └──────────────────────┘
-- @
--
-- The 'prFill' field is a CSS-style color string: @\"#ff0000\"@, @\"red\"@,
-- @\"rgba(0,0,0,0.5)\"@, etc.  An empty string means transparent (no fill).
--
-- === PaintPath
--
-- An arbitrary vector path built from 'PathCommand' steps.
-- The 'ppFill' field colors the enclosed area; leave it empty for
-- an unfilled (outline-only) shape.  This path implementation is
-- stroke-less — only fill is rendered.
--
-- Typical use: hexagonal modules in MaxiCode barcodes.
--
-- == Metadata
--
-- Every instruction carries a @meta@ field: a 'Map' from 'String' keys to
-- aeson 'Value's.  The PaintVM ignores it — it is for producers and
-- debuggers.  Example:
--
-- @
--   Map.fromList [("source", String "qr-finder"), ("layer", String "structural")]
-- @
data PaintInstruction
  = PaintRect
      { prX    :: Double
        -- ^ Top-left x coordinate in scene units.
      , prY    :: Double
        -- ^ Top-left y coordinate in scene units.
      , prW    :: Double
        -- ^ Width in scene units. Must be ≥ 0.
      , prH    :: Double
        -- ^ Height in scene units. Must be ≥ 0.
      , prFill :: String
        -- ^ CSS fill color.  @\"#000000\"@ = black, @\"\"@ = no fill.
      , prStroke :: String
        -- ^ CSS stroke color.  @\"\"@ = no stroke.
      , prStrokeWidth :: Double
        -- ^ Stroke width in scene units. Ignored when 'prStroke' is empty.
      , prMeta :: Map String Value
        -- ^ Optional metadata; ignored by the renderer.
      }
  | PaintPath
      { ppCommands :: [PathCommand]
        -- ^ Ordered list of drawing commands tracing the path.
      , ppFill     :: String
        -- ^ CSS fill color.  @\"\"@ = no fill.
      , ppMeta     :: Map String Value
        -- ^ Optional metadata; ignored by the renderer.
      }
  | PaintGlyphRun
      { pgGlyphs   :: [PaintGlyphPlacement]
        -- ^ Pre-positioned glyphs, each already placed in scene coordinates.
      , pgFontRef  :: String
        -- ^ Opaque font identifier. Text-mode backends ignore this.
      , pgFontSize :: Double
        -- ^ Font size in scene units. Text-mode backends ignore this.
      , pgFill     :: String
        -- ^ CSS color of the glyphs. Text-mode backends ignore this.
      , pgMeta     :: Map String Value
        -- ^ Optional metadata; ignored by the renderer.
      }
  | PaintLine
      { plX1 :: Double
        -- ^ Start point x.
      , plY1 :: Double
        -- ^ Start point y.
      , plX2 :: Double
        -- ^ End point x.
      , plY2 :: Double
        -- ^ End point y.
      , plStroke :: String
        -- ^ CSS stroke color. Required — a line with no stroke is invisible.
      , plStrokeWidth :: Double
        -- ^ Stroke width in scene units.
      , plMeta :: Map String Value
        -- ^ Optional metadata; ignored by the renderer.
      }
  | PaintGroup
      { pgrChildren  :: [PaintInstruction]
        -- ^ Instructions inside this group, rendered back-to-front.
      , pgrTransform :: Maybe Transform2D
        -- ^ Optional affine transform applied to all children.
        --   'Nothing' or 'identityTransform' means no transform.
      , pgrOpacity   :: Maybe Double
        -- ^ Optional group-level compositing opacity (0.0–1.0).
        --   'Nothing' or @Just 1.0@ means fully opaque.
      , pgrMeta      :: Map String Value
        -- ^ Optional metadata; ignored by the renderer.
      }
  | PaintClip
      { pclX        :: Double
        -- ^ Clip rectangle top-left x.
      , pclY        :: Double
        -- ^ Clip rectangle top-left y.
      , pclW        :: Double
        -- ^ Clip rectangle width.
      , pclH        :: Double
        -- ^ Clip rectangle height.
      , pclChildren :: [PaintInstruction]
        -- ^ Instructions rendered inside the clip region.
      , pclMeta     :: Map String Value
        -- ^ Optional metadata; ignored by the renderer.
      }
  | PaintLayer
      { plyChildren   :: [PaintInstruction]
        -- ^ Instructions rendered into the (conceptual) offscreen buffer.
      , plyHasFilters :: Bool
        -- ^ Whether any pixel-level filter (blur, drop shadow, etc.) is
        --   attached. Kept as a simple flag rather than the full filter
        --   union — no backend in this repo's Haskell port implements
        --   filters, so all that matters is whether to reject the layer.
      , plyBlendMode  :: Maybe String
        -- ^ Optional blend mode name. 'Nothing' or @Just \"normal\"@ means
        --   standard alpha compositing (no special blending).
      , plyOpacity    :: Maybe Double
        -- ^ Optional layer-level opacity (0.0–1.0).
      , plyTransform  :: Maybe Transform2D
        -- ^ Optional affine transform applied to the layer as a whole.
      , plyMeta       :: Map String Value
        -- ^ Optional metadata; ignored by the renderer.
      }
  deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- Transform2D
-- ---------------------------------------------------------------------------

-- | A six-value affine transform, matching the Canvas/SVG convention:
--
-- @
--   x' = a*x + c*y + e
--   y' = b*x + d*y + f
-- @
data Transform2D = Transform2D
  { t2dA :: Double
  , t2dB :: Double
  , t2dC :: Double
  , t2dD :: Double
  , t2dE :: Double
  , t2dF :: Double
  } deriving (Show, Eq)

-- | The identity transform — no rotation, scale, or translation.
identityTransform :: Transform2D
identityTransform = Transform2D 1 0 0 1 0 0

-- ---------------------------------------------------------------------------
-- PaintGlyphPlacement
-- ---------------------------------------------------------------------------

-- | One glyph's position within a 'PaintGlyphRun'.
--
-- 'pgpGlyphId' is a font-internal glyph index in the general contract, but
-- text-mode (\"ascii\") backends relax this to a literal Unicode code point
-- — see @P2D02-paint-vm-ascii.md@ §\"Glyph runs\" for the rationale.
data PaintGlyphPlacement = PaintGlyphPlacement
  { pgpGlyphId :: Int
  , pgpX       :: Double
  , pgpY       :: Double
  } deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- PaintScene
-- ---------------------------------------------------------------------------

-- | Top-level container passed to the PaintVM.
--
-- A 'PaintScene' is everything a backend needs to produce a complete image:
-- the canvas dimensions, the background color, and the ordered list of
-- drawing instructions.
--
-- Instructions are rendered in list order (back-to-front / painter's
-- algorithm): the first element is drawn first and may be covered by later
-- elements.
--
-- @
-- ┌──────────────────────────────────────┐  ← psHeight pixels tall
-- │  background color (psBg)             │
-- │  instruction[0]  (drawn first)       │
-- │  instruction[1]  (may cover [0])     │
-- │  ...                                 │
-- └──────────────────────────────────────┘
--   ← psWidth pixels wide →
-- @
--
-- The @psMeta@ field carries arbitrary metadata for producers and dev-tools;
-- the PaintVM forwards it unchanged to backends that support it.
data PaintScene = PaintScene
  { psWidth        :: Double
    -- ^ Canvas width in user-space units (typically pixels).
  , psHeight       :: Double
    -- ^ Canvas height in user-space units.
  , psInstructions :: [PaintInstruction]
    -- ^ Ordered drawing instructions. Painted back-to-front.
  , psBg           :: String
    -- ^ Background CSS color painted before all instructions.
    --   Use @\"transparent\"@ for no background fill.
  , psMeta         :: Map String Value
    -- ^ Optional scene-level metadata; forwarded unchanged by the VM.
  } deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- Builder helpers
-- ---------------------------------------------------------------------------

-- | Create an empty 'PaintScene' with no instructions.
--
-- Useful as a starting point:
--
-- @
-- let scene = (emptyScene 400 300 "#ffffff")
--               { psInstructions = [makeRect 10 10 50 50 "#cc0000"] }
-- @
emptyScene
  :: Double  -- ^ Width
  -> Double  -- ^ Height
  -> String  -- ^ Background color (CSS)
  -> PaintScene
emptyScene w h bg = PaintScene
  { psWidth        = w
  , psHeight       = h
  , psInstructions = []
  , psBg           = bg
  , psMeta         = Map.empty
  }

-- | Build a 'PaintRect' instruction with no metadata.
--
-- This is a convenience wrapper so you don't have to spell out every field:
--
-- @
-- makeRect 10 10 80 40 \"#2563eb\"
-- -- is equivalent to:
-- PaintRect { prX = 10, prY = 10, prW = 80, prH = 40
--           , prFill = \"#2563eb\", prMeta = Map.empty }
-- @
makeRect
  :: Double  -- ^ x (top-left)
  -> Double  -- ^ y (top-left)
  -> Double  -- ^ width
  -> Double  -- ^ height
  -> String  -- ^ fill color (CSS)
  -> PaintInstruction
makeRect x y w h fill = PaintRect
  { prX    = x
  , prY    = y
  , prW    = w
  , prH    = h
  , prFill = fill
  , prStroke = ""
  , prStrokeWidth = 0
  , prMeta = Map.empty
  }

-- | Build a 'PaintPath' instruction with no metadata.
--
-- @
-- makePath
--   [ MoveTo 50 10, LineTo 90 80, LineTo 10 80, ClosePath ]
--   \"#ef4444\"
-- @
makePath
  :: [PathCommand]  -- ^ Path commands
  -> String         -- ^ Fill color (CSS)
  -> PaintInstruction
makePath cmds fill = PaintPath
  { ppCommands = cmds
  , ppFill     = fill
  , ppMeta     = Map.empty
  }

-- | Build a 'PaintGlyphRun' instruction with no metadata.
makeGlyphRun
  :: [PaintGlyphPlacement]  -- ^ Pre-positioned glyphs
  -> String                 -- ^ Font ref (opaque; text-mode backends ignore it)
  -> Double                 -- ^ Font size (text-mode backends ignore it)
  -> String                 -- ^ Fill color (CSS; text-mode backends ignore it)
  -> PaintInstruction
makeGlyphRun glyphs fontRef fontSize fill = PaintGlyphRun
  { pgGlyphs   = glyphs
  , pgFontRef  = fontRef
  , pgFontSize = fontSize
  , pgFill     = fill
  , pgMeta     = Map.empty
  }

-- | Build a 'PaintLine' instruction with no metadata.
makeLine
  :: Double  -- ^ x1
  -> Double  -- ^ y1
  -> Double  -- ^ x2
  -> Double  -- ^ y2
  -> String  -- ^ Stroke color (CSS)
  -> Double  -- ^ Stroke width
  -> PaintInstruction
makeLine x1 y1 x2 y2 stroke strokeWidth = PaintLine
  { plX1 = x1, plY1 = y1, plX2 = x2, plY2 = y2
  , plStroke = stroke
  , plStrokeWidth = strokeWidth
  , plMeta = Map.empty
  }

-- | Build a plain (untransformed, fully opaque) 'PaintGroup' with no metadata.
makeGroup :: [PaintInstruction] -> PaintInstruction
makeGroup children = PaintGroup
  { pgrChildren = children
  , pgrTransform = Nothing
  , pgrOpacity = Nothing
  , pgrMeta = Map.empty
  }

-- | Build a 'PaintClip' instruction with no metadata.
makeClip
  :: Double            -- ^ x
  -> Double            -- ^ y
  -> Double            -- ^ width
  -> Double            -- ^ height
  -> [PaintInstruction] -- ^ Children rendered inside the clip region
  -> PaintInstruction
makeClip x y w h children = PaintClip
  { pclX = x, pclY = y, pclW = w, pclH = h
  , pclChildren = children
  , pclMeta = Map.empty
  }

-- | Build a plain (untransformed, unfiltered, fully opaque, normal-blend)
-- 'PaintLayer' with no metadata.
makeLayer :: [PaintInstruction] -> PaintInstruction
makeLayer children = PaintLayer
  { plyChildren = children
  , plyHasFilters = False
  , plyBlendMode = Nothing
  , plyOpacity = Nothing
  , plyTransform = Nothing
  , plyMeta = Map.empty
  }

-- | Append an instruction to an existing 'PaintScene'.
--
-- Because 'PaintScene' is immutable data, this returns a new scene with
-- the instruction appended.  The original scene is unchanged.
--
-- Example:
--
-- @
-- let scene1 = emptyScene 200 100 \"#fff\"
-- let scene2 = addInstruction scene1 (makeRect 0 0 200 100 \"#000\")
-- length (psInstructions scene2) == 1
-- @
addInstruction :: PaintScene -> PaintInstruction -> PaintScene
addInstruction scene instr =
  scene { psInstructions = psInstructions scene ++ [instr] }
