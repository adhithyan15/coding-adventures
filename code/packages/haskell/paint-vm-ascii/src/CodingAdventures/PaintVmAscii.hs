-- | A small, pure terminal backend for 'PaintScene' values.
--
-- Scene coordinates are divided by a configurable horizontal and vertical
-- scale to obtain character-cell coordinates. Filled rectangles are drawn
-- with full-block characters and output is trimmed for terminal use.
module CodingAdventures.PaintVmAscii
  ( AsciiOptions (..)
  , PaintVmAsciiError (..)
  , defaultAsciiOptions
  , render
  , renderDefault
  , version
  ) where

import Control.Monad (foldM)
import Data.Char (isSpace)
import Data.List (dropWhileEnd)
import CodingAdventures.PaintInstructions
  ( PaintInstruction (..)
  , PaintScene (..)
  )

-- | Package version, shared with the other language implementations.
version :: String
version = "0.1.0"

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
  | UnsupportedInstruction String
  deriving (Eq, Show)

-- | Render with 'defaultAsciiOptions'.
renderDefault :: PaintScene -> Either PaintVmAsciiError String
renderDefault scene = render scene defaultAsciiOptions

-- | Render a scene as terminal-friendly text.
--
-- The renderer supports the rectangle instruction available in every paint
-- IR implementation. Haskell's other current instruction, 'PaintPath', is
-- rejected explicitly so a caller never mistakes a partial rendering for a
-- complete one.
render :: PaintScene -> AsciiOptions -> Either PaintVmAsciiError String
render scene options
  | scaleX options <= 0 = Left (InvalidScaleX (scaleX options))
  | scaleY options <= 0 = Left (InvalidScaleY (scaleY options))
  | not (validDimension (psWidth scene) && validDimension (psHeight scene)) =
      Left (InvalidSceneDimensions (psWidth scene) (psHeight scene))
  | otherwise = do
      let columns = ceiling (psWidth scene / fromIntegral (scaleX options))
          rows = ceiling (psHeight scene / fromIntegral (scaleY options))
          buffer = replicate rows (replicate columns ' ')
      rendered <- foldM (renderInstruction options) buffer (psInstructions scene)
      pure (bufferText rendered)

validDimension :: Double -> Bool
validDimension value = value >= 0 && not (isNaN value || isInfinite value)

renderInstruction
  :: AsciiOptions
  -> [String]
  -> PaintInstruction
  -> Either PaintVmAsciiError [String]
renderInstruction options buffer instruction = case instruction of
  PaintRect {}
    | validRectangle instruction -> Right (renderRectangle options instruction buffer)
    | otherwise -> Left (InvalidRectangleGeometry
        (prX instruction) (prY instruction) (prW instruction) (prH instruction))
  PaintPath {} -> Left (UnsupportedInstruction "path")

validRectangle :: PaintInstruction -> Bool
validRectangle rectangle =
  validCoordinate (prX rectangle)
    && validCoordinate (prY rectangle)
    && validDimension (prW rectangle)
    && validDimension (prH rectangle)

validCoordinate :: Double -> Bool
validCoordinate value = not (isNaN value || isInfinite value)

renderRectangle :: AsciiOptions -> PaintInstruction -> [String] -> [String]
renderRectangle options rectangle buffer
  | not (visiblePaint (prFill rectangle)) = buffer
  | otherwise =
      [ if rowIndex >= firstRow && rowIndex <= lastRow
          then paintColumns firstColumn lastColumn row
          else row
      | (rowIndex, row) <- zip [0 ..] buffer
      ]
  where
    firstColumn = toCell (prX rectangle) (scaleX options)
    firstRow = toCell (prY rectangle) (scaleY options)
    lastColumn = toCell (prX rectangle + prW rectangle) (scaleX options)
    lastRow = toCell (prY rectangle + prH rectangle) (scaleY options)

toCell :: Double -> Int -> Int
toCell coordinate scale = round (coordinate / fromIntegral scale)

paintColumns :: Int -> Int -> String -> String
paintColumns firstColumn lastColumn row =
  [ if column >= firstColumn && column <= lastColumn then '\x2588' else cell
  | (column, cell) <- zip [0 ..] row
  ]

visiblePaint :: String -> Bool
visiblePaint paint = trimmed /= "" && trimmed /= "transparent" && trimmed /= "none"
  where
    trimmed = trim paint

trim :: String -> String
trim = dropWhileEnd isSpace . dropWhile isSpace

bufferText :: [String] -> String
bufferText = joinLines . dropWhileEnd null . map (dropWhileEnd (== ' '))

joinLines :: [String] -> String
joinLines [] = ""
joinLines rows = foldr1 (\row rest -> row ++ "\n" ++ rest) rows
