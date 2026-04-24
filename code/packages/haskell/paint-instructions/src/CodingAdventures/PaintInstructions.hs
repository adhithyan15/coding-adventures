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

    -- * PaintInstruction
  , PaintInstruction (..)

    -- * PaintScene
  , PaintScene (..)

    -- * Builder helpers
  , emptyScene
  , makeRect
  , makePath
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
  deriving (Show, Eq)

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
