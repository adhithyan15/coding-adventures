-- | A small, pure terminal backend for 'PaintScene' values.
--
-- Implements the full @P2D02-paint-vm-ascii.md@ contract: filled/stroked
-- rectangles, lines, glyph runs, and plain (untransformed, unfiltered,
-- fully opaque) groups/clips/layers. Scene coordinates are divided by a
-- configurable horizontal and vertical scale to obtain character-cell
-- coordinates.
--
-- The buffer is represented as a sparse 'Map' from @(row, col)@ to a
-- 'Cell', rather than a mutable 2D array — scenes rendered by this backend
-- are small (terminal-sized), so the simplicity of an immutable Map-based
-- model outweighs any performance concern, and it keeps the box-drawing
-- merge logic (two strokes sharing a corner combine into one character)
-- expressible as ordinary pure functions.
module CodingAdventures.PaintVmAscii
  ( AsciiOptions (..)
  , PaintVmAsciiError (..)
  , defaultAsciiOptions
  , render
  , renderDefault
  , version
  ) where

import Control.Monad (foldM)
import Data.Bits ((.&.), (.|.))
import Data.Char (chr, isSpace)
import Data.List (dropWhileEnd)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import CodingAdventures.PaintInstructions
  ( PaintGlyphPlacement (..)
  , PaintInstruction (..)
  , PaintScene (..)
  , Transform2D (..)
  )

-- | Package version, shared with the other language implementations.
version :: String
version = "0.2.0"

-- | How scene coordinates map to terminal character cells.
data AsciiOptions = AsciiOptions
  { scaleX :: Int
  , scaleY :: Int
  } deriving (Eq, Show)

-- | The cross-language default uses cells that are eight scene units wide
-- and sixteen scene units tall.
defaultAsciiOptions :: AsciiOptions
defaultAsciiOptions = AsciiOptions
  { scaleX = 8
  , scaleY = 16
  }

-- | Errors that can be reported without partial functions or exceptions.
data PaintVmAsciiError
  = InvalidScaleX Int
  | InvalidScaleY Int
  | InvalidSceneDimensions Double Double
  | InvalidRectangleGeometry Double Double Double Double
  | InvalidLineGeometry Double Double Double Double
  | InvalidClipGeometry Double Double Double Double
  | SceneTooLarge Double Double
  | UnsupportedInstruction String
  deriving (Eq, Show)

-- | Render with 'defaultAsciiOptions'.
renderDefault :: PaintScene -> Either PaintVmAsciiError String
renderDefault scene = render scene defaultAsciiOptions

-- ---------------------------------------------------------------------------
-- Buffer
-- ---------------------------------------------------------------------------

-- | Directional/fill tag bits, mirroring the csharp\/fsharp\/perl ports'
-- CellFlags. Two intersecting strokes merge their flags so the right
-- box-drawing glyph (corner, tee, cross) is chosen regardless of draw
-- order.
type CellFlags = Int

flagUp, flagRight, flagDown, flagLeft, flagFill :: CellFlags
flagUp = 1
flagRight = 2
flagDown = 4
flagLeft = 8
flagFill = 16

-- | One character cell. 'CellText' (from a glyph run) always wins over
-- 'CellTag' (from rect\/line strokes or fills) — literal text is never
-- overwritten by box-drawing or fill, per P2D02's rendering rules.
data Cell
  = CellTag CellFlags
  | CellText Char
  deriving (Eq, Show)

type Buffer = Map (Int, Int) Cell

data ClipBounds = ClipBounds
  { clMinCol :: Int
  , clMinRow :: Int
  , clMaxCol :: Int
  , clMaxRow :: Int
  } deriving (Eq, Show)

fullClip :: Int -> Int -> ClipBounds
fullClip cols rows = ClipBounds 0 0 cols rows

insideClip :: ClipBounds -> (Int, Int) -> Bool
insideClip clip (row, col) =
  row >= clMinRow clip && row < clMaxRow clip
    && col >= clMinCol clip && col < clMaxCol clip

-- | Clamp a cell coordinate into a clip's bounds. Used before building any
-- range that iterates between two cell coordinates (rect fill/stroke, line
-- endpoints), so a caller-supplied geometry with a huge (but finite, valid)
-- extent can't force iteration/recursion far beyond the actual clipped
-- surface — bounded by the clip's own size instead of by caller input.
clampCol :: ClipBounds -> Int -> Int
clampCol clip value = max (clMinCol clip) (min value (clMaxCol clip - 1))

clampRow :: ClipBounds -> Int -> Int
clampRow clip value = max (clMinRow clip) (min value (clMaxRow clip - 1))

writeTag :: ClipBounds -> (Int, Int) -> CellFlags -> Buffer -> Buffer
writeTag clip pos flags buf
  | not (insideClip clip pos) = buf
  | otherwise = Map.alter merge pos buf
  where
    merge Nothing = Just (CellTag flags)
    merge (Just (CellTag existing)) = Just (CellTag (existing .|. flags))
    merge existing@(Just (CellText _)) = existing

writeChar :: ClipBounds -> (Int, Int) -> Char -> Buffer -> Buffer
writeChar clip pos ch buf
  | not (insideClip clip pos) = buf
  | otherwise = Map.insert pos (CellText ch) buf

resolveCell :: Cell -> Char
resolveCell (CellText ch) = ch
resolveCell (CellTag flags)
  | directions /= 0, Just boxChar <- lookup directions boxCharacters = boxChar
  | flags .&. flagFill /= 0 = '\x2588'
  | otherwise = '+'
  where
    directions = flags .&. (flagUp .|. flagRight .|. flagDown .|. flagLeft)

boxCharacters :: [(CellFlags, Char)]
boxCharacters =
  [ (flagLeft .|. flagRight, '\x2500')
  , (flagUp .|. flagDown, '\x2502')
  , (flagDown .|. flagRight, '\x250C')
  , (flagDown .|. flagLeft, '\x2510')
  , (flagUp .|. flagRight, '\x2514')
  , (flagUp .|. flagLeft, '\x2518')
  , (flagLeft .|. flagRight .|. flagDown, '\x252C')
  , (flagLeft .|. flagRight .|. flagUp, '\x2534')
  , (flagUp .|. flagDown .|. flagRight, '\x251C')
  , (flagUp .|. flagDown .|. flagLeft, '\x2524')
  , (flagUp .|. flagDown .|. flagLeft .|. flagRight, '\x253C')
  , (flagRight, '\x2500')
  , (flagLeft, '\x2500')
  , (flagUp, '\x2502')
  , (flagDown, '\x2502')
  ]

bufferToText :: Int -> Int -> Buffer -> String
bufferToText rows cols buf =
  joinLines (dropWhileEnd null (map rowText [0 .. rows - 1]))
  where
    rowText row = dropWhileEnd (== ' ') [cellAt row col | col <- [0 .. cols - 1]]
    cellAt row col = maybe ' ' resolveCell (Map.lookup (row, col) buf)

joinLines :: [String] -> String
joinLines [] = ""
joinLines rows = foldr1 (\row rest -> row ++ "\n" ++ rest) rows

-- ---------------------------------------------------------------------------
-- Coordinate conversion
-- ---------------------------------------------------------------------------

-- | Cell-coordinate values are saturated to this bound (rather than left as
-- raw @round@ output) so a large-but-ordinary finite 'Double' (e.g.
-- @~6.6e35@, nowhere near 'Double''s own range limit) can never land on
-- exactly 'minBound'\/'maxBound' :: 'Int'. Without this, a 'PaintClip' extent
-- rounding to 'minBound' would defeat 'clampCol'\/'clampRow' downstream: the
-- @clMaxCol - 1@ they compute silently wraps 'Int' from 'minBound' to
-- 'maxBound', un-clamping every rect\/line nested in that clip and
-- reopening the same unbounded-iteration DoS the clip clamping exists to
-- prevent. A billion cells in either direction is far beyond any real
-- rendered scene (scenes are additionally capped at 'maxAxisCells' per
-- axis) while leaving enormous headroom below 'Int''s actual bounds for
-- 'clampCol'\/'clampRow's arithmetic to stay overflow-free.
cellBound :: Int
cellBound = 1000000000

toCell :: Double -> Int -> Int
toCell coordinate scale
  | isNaN scaled = 0
  | scaled >= fromIntegral cellBound = cellBound
  | scaled <= fromIntegral (negate cellBound) = negate cellBound
  | otherwise = round scaled
  where
    scaled = coordinate / fromIntegral scale

validDimension :: Double -> Bool
validDimension value = value >= 0 && not (isNaN value || isInfinite value)

validCoordinate :: Double -> Bool
validCoordinate value = not (isNaN value || isInfinite value)

-- | Upper bound on the number of character cells a rendered scene may
-- occupy, both in total and per axis. Scene dimensions are otherwise only
-- checked for being finite and non-negative, so without this a
-- caller-supplied @psWidth@\/@psHeight@ of e.g. @1e12@ would force
-- 'bufferToText' to iterate on the order of @10^22@ cells even with zero
-- drawing instructions — a denial-of-service unrelated to (and not fixed
-- by) the per-instruction clip clamping below. The per-axis bound is
-- required in addition to the product bound: a zero-width, huge-height
-- scene has a product of zero (passing a product-only check) while still
-- forcing an unbounded 'bufferToText' traversal along the surviving axis.
-- 2000x2000 (a generous terminal-sized canvas) is cheap to fully
-- materialize either way.
maxAxisCells :: Integer
maxAxisCells = 2000

maxBufferCells :: Integer
maxBufferCells = maxAxisCells * maxAxisCells

-- ---------------------------------------------------------------------------
-- Top-level render
-- ---------------------------------------------------------------------------

-- | Render a scene as terminal-friendly text.
render :: PaintScene -> AsciiOptions -> Either PaintVmAsciiError String
render scene options
  | scaleX options <= 0 = Left (InvalidScaleX (scaleX options))
  | scaleY options <= 0 = Left (InvalidScaleY (scaleY options))
  | not (validDimension (psWidth scene) && validDimension (psHeight scene)) =
      Left (InvalidSceneDimensions (psWidth scene) (psHeight scene))
  | otherwise =
      -- Computed as 'Integer' (not 'Int') so an astronomically large but
      -- finite scene can't overflow the size check it's meant to trigger.
      let columnsI = ceiling (psWidth scene / fromIntegral (scaleX options)) :: Integer
          rowsI = ceiling (psHeight scene / fromIntegral (scaleY options)) :: Integer
       in if columnsI > maxAxisCells || rowsI > maxAxisCells || columnsI * rowsI > maxBufferCells
            then Left (SceneTooLarge (psWidth scene) (psHeight scene))
            else do
              let columns = fromInteger columnsI
                  rows = fromInteger rowsI
                  clip = fullClip columns rows
              buffer <- foldM (dispatch options clip) Map.empty (psInstructions scene)
              pure (bufferToText rows columns buffer)

-- | Render one instruction (recursing into group\/clip\/layer children),
-- threading the buffer and failing loudly on anything not in the P2D02
-- contract.
dispatch :: AsciiOptions -> ClipBounds -> Buffer -> PaintInstruction -> Either PaintVmAsciiError Buffer
dispatch options clip buffer instruction = case instruction of
  PaintRect {}
    | validRectangle instruction -> Right (renderRectangle options clip instruction buffer)
    | otherwise -> Left (InvalidRectangleGeometry
        (prX instruction) (prY instruction) (prW instruction) (prH instruction))
  PaintLine {}
    | validLine instruction -> Right (renderLine options clip instruction buffer)
    | otherwise -> Left (InvalidLineGeometry
        (plX1 instruction) (plY1 instruction) (plX2 instruction) (plY2 instruction))
  PaintGlyphRun {} -> Right (renderGlyphRun options clip instruction buffer)
  PaintGroup {} -> do
    assertPlainGroup instruction
    foldM (dispatch options clip) buffer (pgrChildren instruction)
  PaintClip {}
    | validClip instruction -> do
        let nextClip = intersectClip clip (clipBoundsOf options instruction)
        foldM (dispatch options nextClip) buffer (pclChildren instruction)
    | otherwise -> Left (InvalidClipGeometry
        (pclX instruction) (pclY instruction) (pclW instruction) (pclH instruction))
  PaintLayer {} -> do
    assertPlainLayer instruction
    foldM (dispatch options clip) buffer (plyChildren instruction)
  PaintPath {} -> Left (UnsupportedInstruction "path")

clipBoundsOf :: AsciiOptions -> PaintInstruction -> ClipBounds
clipBoundsOf options instruction =
  ClipBounds
    { clMinCol = toCell (pclX instruction) (scaleX options)
    , clMinRow = toCell (pclY instruction) (scaleY options)
    , clMaxCol = toCell (pclX instruction + pclW instruction) (scaleX options)
    , clMaxRow = toCell (pclY instruction + pclH instruction) (scaleY options)
    }

-- | Validates the individual fields *and* the @x+w@\/@y+h@ extents used by
-- 'clipBoundsOf' — two individually-finite values near @DBL_MAX@ can still
-- sum to +Infinity under IEEE-754 arithmetic, so checking the fields alone
-- isn't sufficient to guarantee 'toCell' never sees a non-finite input.
validClip :: PaintInstruction -> Bool
validClip clipInstruction =
  validCoordinate (pclX clipInstruction)
    && validCoordinate (pclY clipInstruction)
    && validDimension (pclW clipInstruction)
    && validDimension (pclH clipInstruction)
    && validCoordinate (pclX clipInstruction + pclW clipInstruction)
    && validCoordinate (pclY clipInstruction + pclH clipInstruction)

intersectClip :: ClipBounds -> ClipBounds -> ClipBounds
intersectClip parent child =
  ClipBounds
    { clMinCol = max (clMinCol parent) (clMinCol child)
    , clMinRow = max (clMinRow parent) (clMinRow child)
    , clMaxCol = min (clMaxCol parent) (clMaxCol child)
    , clMaxRow = min (clMaxRow parent) (clMaxRow child)
    }

-- ---------------------------------------------------------------------------
-- Rect
-- ---------------------------------------------------------------------------

-- | Validates the individual fields *and* the @x+w@\/@y+h@ extents used by
-- 'renderRectangle' — see 'validClip's comment for why the sum needs its
-- own check.
validRectangle :: PaintInstruction -> Bool
validRectangle rectangle =
  validCoordinate (prX rectangle)
    && validCoordinate (prY rectangle)
    && validDimension (prW rectangle)
    && validDimension (prH rectangle)
    && validCoordinate (prX rectangle + prW rectangle)
    && validCoordinate (prY rectangle + prH rectangle)

renderRectangle :: AsciiOptions -> ClipBounds -> PaintInstruction -> Buffer -> Buffer
renderRectangle options clip rectangle buffer =
  strokePass (fillPass buffer)
  where
    c1 = clampCol clip (toCell (prX rectangle) (scaleX options))
    r1 = clampRow clip (toCell (prY rectangle) (scaleY options))
    c2 = clampCol clip (toCell (prX rectangle + prW rectangle) (scaleX options))
    r2 = clampRow clip (toCell (prY rectangle + prH rectangle) (scaleY options))

    hasFill = visiblePaint (prFill rectangle)
    hasStroke = not (null (trim (prStroke rectangle)))

    fillPass buf
      | not hasFill = buf
      | otherwise =
          foldl (\acc (row, col) -> writeTag clip (row, col) flagFill acc) buf
            [(row, col) | row <- [r1 .. r2], col <- [c1 .. c2]]

    strokePass buf
      | not hasStroke = buf
      | otherwise =
          let corners =
                [ ((r1, c1), flagDown .|. flagRight)
                , ((r1, c2), flagDown .|. flagLeft)
                , ((r2, c1), flagUp .|. flagRight)
                , ((r2, c2), flagUp .|. flagLeft)
                ]
              topBottom =
                [((r, col), flagLeft .|. flagRight) | r <- [r1, r2], col <- [c1 + 1 .. c2 - 1]]
              leftRight =
                [((row, c), flagUp .|. flagDown) | c <- [c1, c2], row <- [r1 + 1 .. r2 - 1]]
           in foldl (\acc (pos, flags) -> writeTag clip pos flags acc) buf
                (corners ++ topBottom ++ leftRight)

visiblePaint :: String -> Bool
visiblePaint paint = trimmed /= "" && trimmed /= "transparent" && trimmed /= "none"
  where
    trimmed = trim paint

trim :: String -> String
trim = dropWhileEnd isSpace . dropWhile isSpace

-- ---------------------------------------------------------------------------
-- Line (horizontal/vertical fast paths + Bresenham for the diagonal case)
-- ---------------------------------------------------------------------------

validLine :: PaintInstruction -> Bool
validLine line =
  validCoordinate (plX1 line)
    && validCoordinate (plY1 line)
    && validCoordinate (plX2 line)
    && validCoordinate (plY2 line)

renderLine :: AsciiOptions -> ClipBounds -> PaintInstruction -> Buffer -> Buffer
renderLine options clip line buffer
  | r1 == r2 =
      foldl (\acc col -> writeTag clip (r1, col) (horizontalFlags col) acc) buffer [minCol .. maxCol]
  | c1 == c2 =
      foldl (\acc row -> writeTag clip (row, c1) (verticalFlags row) acc) buffer [minRow .. maxRow]
  | otherwise = bresenham buffer r1 c1 0
  where
    -- Clamped into the clip's own bounds before use — an out-of-range but
    -- otherwise valid (finite) endpoint can't force iteration/recursion far
    -- beyond the actual clipped surface. See 'clampCol'/'clampRow'.
    c1 = clampCol clip (toCell (plX1 line) (scaleX options))
    r1 = clampRow clip (toCell (plY1 line) (scaleY options))
    c2 = clampCol clip (toCell (plX2 line) (scaleX options))
    r2 = clampRow clip (toCell (plY2 line) (scaleY options))

    minCol = min c1 c2
    maxCol = max c1 c2
    horizontalFlags col
      | col == minCol && col == maxCol = flagLeft .|. flagRight
      | col == minCol = flagRight
      | col == maxCol = flagLeft
      | otherwise = flagLeft .|. flagRight

    minRow = min r1 r2
    maxRow = max r1 r2
    verticalFlags row
      | row == minRow && row == maxRow = flagUp .|. flagDown
      | row == minRow = flagDown
      | row == maxRow = flagUp
      | otherwise = flagUp .|. flagDown

    deltaRow = abs (r2 - r1)
    deltaCol = abs (c2 - c1)
    stepRow = if r1 < r2 then 1 else -1
    stepCol = if c1 < c2 then 1 else -1
    diagonalFlags = if deltaCol > deltaRow then flagLeft .|. flagRight else flagUp .|. flagDown

    bresenham buf row col errorValue =
      let buf' = writeTag clip (row, col) diagonalFlags buf
       in if row == r2 && col == c2
            then buf'
            else
              let doubled = 2 * errorValue
                  (errorValue1, col1) =
                    if doubled > negate deltaRow then (errorValue - deltaRow, col + stepCol) else (errorValue, col)
                  (errorValue2, row1) =
                    if doubled < deltaCol then (errorValue1 + deltaCol, row + stepRow) else (errorValue1, row)
               in bresenham buf' row1 col1 errorValue2

-- ---------------------------------------------------------------------------
-- Glyph run
-- ---------------------------------------------------------------------------

-- | A glyph with a non-finite position is skipped rather than passed to
-- 'toCell' (whose 'round' has no defined behavior for NaN\/Infinity) —
-- unlike a malformed 'PaintRect'\/'PaintLine'\/'PaintClip', a single bad
-- glyph placement doesn't need to fail the whole render.
renderGlyphRun :: AsciiOptions -> ClipBounds -> PaintInstruction -> Buffer -> Buffer
renderGlyphRun options clip glyphRun buffer =
  foldl place buffer (pgGlyphs glyphRun)
  where
    place buf glyph
      | not (validCoordinate (pgpX glyph) && validCoordinate (pgpY glyph)) = buf
      | otherwise =
          writeChar
            clip
            (toCell (pgpY glyph) (scaleY options), toCell (pgpX glyph) (scaleX options))
            (toSafeTerminalGlyph (pgpGlyphId glyph))
            buf

-- | ASCII-backend-specific relaxation of the general 'PaintGlyphPlacement'
-- contract: 'pgpGlyphId' is treated as a literal Unicode code point (no
-- font resolution happens in a terminal), per @P2D02-paint-vm-ascii.md@.
-- Control characters and bidi-control code points are replaced with @?@ so
-- a crafted message can't inject terminal escape sequences.
toSafeTerminalGlyph :: Int -> Char
toSafeTerminalGlyph codePoint
  | isSafeTerminalCodePoint codePoint, codePoint >= 0, codePoint <= 0x10FFFF = chr codePoint
  | otherwise = '?'

isSafeTerminalCodePoint :: Int -> Bool
isSafeTerminalCodePoint codePoint
  | codePoint < 0x20 = False
  | codePoint >= 0x7f && codePoint <= 0x9f = False
  | codePoint >= 0xD800 && codePoint <= 0xDFFF = False
  | codePoint `elem` [0x200e, 0x200f, 0x061c] = False
  | codePoint >= 0x202a && codePoint <= 0x202e = False
  | codePoint >= 0x2066 && codePoint <= 0x2069 = False
  | otherwise = True

-- ---------------------------------------------------------------------------
-- Group / Layer plainness checks
-- ---------------------------------------------------------------------------

isIdentityTransform :: Maybe Transform2D -> Bool
isIdentityTransform Nothing = True
isIdentityTransform (Just t) =
  t2dA t == 1 && t2dB t == 0 && t2dC t == 0 && t2dD t == 1 && t2dE t == 0 && t2dF t == 0

assertPlainGroup :: PaintInstruction -> Either PaintVmAsciiError ()
assertPlainGroup group
  | not (isIdentityTransform (pgrTransform group)) =
      Left (UnsupportedInstruction "group with a non-identity transform")
  | maybe False (/= 1.0) (pgrOpacity group) =
      Left (UnsupportedInstruction "group with non-default opacity")
  | otherwise = Right ()

assertPlainLayer :: PaintInstruction -> Either PaintVmAsciiError ()
assertPlainLayer layer
  | not (isIdentityTransform (plyTransform layer)) =
      Left (UnsupportedInstruction "layer with a non-identity transform")
  | maybe False (/= 1.0) (plyOpacity layer) =
      Left (UnsupportedInstruction "layer with non-default opacity")
  | plyHasFilters layer =
      Left (UnsupportedInstruction "layer with filters")
  | maybe False (/= "normal") (plyBlendMode layer) =
      Left (UnsupportedInstruction "layer with a non-normal blend mode")
  | otherwise = Right ()
