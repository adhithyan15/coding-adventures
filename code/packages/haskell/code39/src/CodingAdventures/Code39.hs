-- | Pure Code 39 barcode encoding.
module CodingAdventures.Code39
  ( EncodedCharacter (..)
  , Code39Error (..)
  , Barcode1DError (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , PaintScene (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , normalizeCode39
  , encodeCode39Char
  , encodeCode39
  , expandCode39Runs
  , layoutCode39
  , drawCode39
  , version
  ) where

import CodingAdventures.BarcodeLayout1D
  ( Barcode1DError (..)
  , Barcode1DRenderConfig (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , PaintBarcode1DOptions (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , defaultWidthPatternOptions
  , layoutBarcode1D
  , runsFromWidthPattern
  )
import CodingAdventures.PaintInstructions (PaintScene (..))
import Data.Aeson (toJSON)
import Data.Bifunctor (first)
import Data.Char (toUpper)
import qualified Data.Map.Strict as Map

-- | Package version shared with the established implementations.
version :: String
version = "0.1.0"

-- | One normalized character and its nine-element width pattern.
data EncodedCharacter = EncodedCharacter
  { encodedCharacterValue :: Char
  , encodedCharacterIsStartStop :: Bool
  , encodedCharacterPattern :: String
  } deriving (Eq, Show)

-- | Checked failures from input validation or shared layout validation.
data Code39Error
  = InvalidCode39Character Char
  | ReservedCode39StartStop
  | Code39LayoutError Barcode1DError
  deriving (Eq)

instance Show Code39Error where
  show (InvalidCode39Character character) =
    "Invalid character: " ++ show [character] ++ " is not supported by Code 39"
  show ReservedCode39StartStop =
    "Input must not contain \"*\" because it is reserved for start/stop"
  show (Code39LayoutError layoutError) = show layoutError

code39Patterns :: Map.Map Char String
code39Patterns = Map.fromList
  [ ('0', "bwbWBwBwb")
  , ('1', "BwbWbwbwB")
  , ('2', "bwBWbwbwB")
  , ('3', "BwBWbwbwb")
  , ('4', "bwbWBwbwB")
  , ('5', "BwbWBwbwb")
  , ('6', "bwBWBwbwb")
  , ('7', "bwbWbwBwB")
  , ('8', "BwbWbwBwb")
  , ('9', "bwBWbwBwb")
  , ('A', "BwbwbWbwB")
  , ('B', "bwBwbWbwB")
  , ('C', "BwBwbWbwb")
  , ('D', "bwbwBWbwB")
  , ('E', "BwbwBWbwb")
  , ('F', "bwBwBWbwb")
  , ('G', "bwbwbWBwB")
  , ('H', "BwbwbWBwb")
  , ('I', "bwBwbWBwb")
  , ('J', "bwbwBWBwb")
  , ('K', "BwbwbwbWB")
  , ('L', "bwBwbwbWB")
  , ('M', "BwBwbwbWb")
  , ('N', "bwbwBwbWB")
  , ('O', "BwbwBwbWb")
  , ('P', "bwBwBwbWb")
  , ('Q', "bwbwbwBWB")
  , ('R', "BwbwbwBWb")
  , ('S', "bwBwbwBWb")
  , ('T', "bwbwBwBWb")
  , ('U', "BWbwbwbwB")
  , ('V', "bWBwbwbwB")
  , ('W', "BWBwbwbwb")
  , ('X', "bWbwBwbwB")
  , ('Y', "BWbwBwbwb")
  , ('Z', "bWBwBwbwb")
  , ('-', "bWbwbwBwB")
  , ('.', "BWbwbwBwb")
  , (' ', "bWBwbwBwb")
  , ('$', "bWbWbWbwb")
  , ('/', "bWbWbwbWb")
  , ('+', "bWbwbWbWb")
  , ('%', "bwbWbWbWb")
  , ('*', "bWbwBwBwb")
  ]

-- | Uppercase letters, preserve spaces, and reject unsupported input.
normalizeCode39 :: String -> Either Code39Error String
normalizeCode39 input = validate normalized >> Right normalized
  where
    normalized = map toUpper input
    validate [] = Right ()
    validate (character : rest)
      | character == '*' = Left ReservedCode39StartStop
      | Map.notMember character code39Patterns = Left (InvalidCode39Character character)
      | otherwise = validate rest

-- | Look up one exact standard Code 39 symbol.
encodeCode39Char :: Char -> Either Code39Error EncodedCharacter
encodeCode39Char character = case Map.lookup character code39Patterns of
  Nothing -> Left (InvalidCode39Character character)
  Just patternText -> Right EncodedCharacter
    { encodedCharacterValue = character
    , encodedCharacterIsStartStop = character == '*'
    , encodedCharacterPattern = widthPattern patternText
    }

-- | Normalize data and insert the standard start and stop markers.
encodeCode39 :: String -> Either Code39Error [EncodedCharacter]
encodeCode39 input = normalizeCode39 input >>= encodeNormalized

-- | Expand every symbol into alternating bar and space runs with narrow gaps.
expandCode39Runs :: String -> Either Code39Error [Barcode1DRun]
expandCode39Runs input = do
  encoded <- encodeCode39 input
  expandEncoded encoded

-- | Render Code 39 through the shared backend-neutral 1D geometry.
layoutCode39
  :: String
  -> PaintBarcode1DOptions
  -> Either Code39Error PaintScene
layoutCode39 input options = do
  normalized <- normalizeCode39 input
  encoded <- encodeNormalized normalized
  runs <- expandEncoded encoded
  first Code39LayoutError
    (layoutBarcode1D runs (code39Options normalized options))

-- | Compatibility alias matching the shared public API name.
drawCode39
  :: String
  -> PaintBarcode1DOptions
  -> Either Code39Error PaintScene
drawCode39 = layoutCode39

widthPattern :: String -> String
widthPattern = map widthMarker
  where
    widthMarker element
      | element == 'B' || element == 'W' = 'W'
      | otherwise = 'N'

encodeNormalized :: String -> Either Code39Error [EncodedCharacter]
encodeNormalized normalized =
  traverse encodeCode39Char ('*' : normalized ++ "*")

expandEncoded :: [EncodedCharacter] -> Either Code39Error [Barcode1DRun]
expandEncoded encoded = concat <$> traverse expandOne indexed
  where
    encodedLength = length encoded
    indexed = zip [0 :: Int ..] encoded
    expandOne (sourceIndex, encodedCharacter) = do
      symbolRuns <- first Code39LayoutError (runsFromWidthPattern
        (encodedCharacterPattern encodedCharacter)
        (defaultWidthPatternOptions
          [encodedCharacterValue encodedCharacter]
          sourceIndex
          (roleFor encodedLength sourceIndex encodedCharacter)))
      let gap
            | sourceIndex < encodedLength - 1 =
                [ Barcode1DRun
                    { runColor = Space
                    , runModules = 1
                    , runSourceLabel = [encodedCharacterValue encodedCharacter]
                    , runSourceIndex = sourceIndex
                    , runRole = InterCharacterGap
                    }
                ]
            | otherwise = []
      Right (symbolRuns ++ gap)

roleFor :: Int -> Int -> EncodedCharacter -> Barcode1DRunRole
roleFor encodedLength sourceIndex encodedCharacter
  | not (encodedCharacterIsStartStop encodedCharacter) = Data
  | sourceIndex == 0 = Start
  | sourceIndex == encodedLength - 1 = Stop
  | otherwise = Guard

code39Options :: String -> PaintBarcode1DOptions -> PaintBarcode1DOptions
code39Options normalized options = options
  { optionsLabel = Just (maybe defaultLabel id (optionsLabel options))
  , optionsMetadata = Map.insert "encodedText" (toJSON normalized)
      (Map.insert "symbology" (toJSON ("code39" :: String)) (optionsMetadata options))
  }
  where
    defaultLabel
      | null normalized = "Code 39 barcode"
      | otherwise = "Code 39 barcode for " ++ normalized
