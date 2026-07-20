-- | Pure Code 128 Code Set B encoding.
module CodingAdventures.Code128
  ( EncodedCode128Symbol (..)
  , Code128Error (..)
  , Barcode1DError (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , PaintScene (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , normalizeCode128B
  , code128BValue
  , code128Pattern
  , computeCode128Checksum
  , encodeCode128B
  , expandCode128Runs
  , layoutCode128
  , drawCode128
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
  , defaultBinaryPatternOptions
  , defaultPaintBarcode1DOptions
  , layoutBarcode1D
  , runsFromBinaryPattern
  )
import CodingAdventures.PaintInstructions (PaintScene (..))
import Data.Aeson (toJSON)
import Data.Bifunctor (first)
import Data.Char (ord)
import qualified Data.Map.Strict as Map

-- | Package version shared with the established implementations.
version :: String
version = "0.1.0"

-- | One control, data, checksum, or stop symbol and its module pattern.
data EncodedCode128Symbol = EncodedCode128Symbol
  { encodedCode128Label :: String
  , encodedCode128Value :: Int
  , encodedCode128Pattern :: String
  , encodedCode128SourceIndex :: Int
  , encodedCode128Role :: Barcode1DRunRole
  } deriving (Eq, Show)

-- | Checked failures from input, table lookup, or shared layout validation.
data Code128Error
  = InvalidCode128Character Char
  | InvalidCode128Value Int
  | Code128LayoutError Barcode1DError
  deriving (Eq)

instance Show Code128Error where
  show (InvalidCode128Character character) =
    "Invalid Code 128 Code Set B character " ++ show [character]
      ++ "; expected printable ASCII (32-126)"
  show (InvalidCode128Value value) =
    "Invalid Code 128 symbol value " ++ show value ++ "; expected 0-106"
  show (Code128LayoutError layoutError) = show layoutError

startB :: Int
startB = 104

stop :: Int
stop = 106

-- | The complete Code 128 pattern table, values 0 through 106.
code128Patterns :: [String]
code128Patterns =
  [ "11011001100", "11001101100", "11001100110", "10010011000"
  , "10010001100", "10001001100", "10011001000", "10011000100"
  , "10001100100", "11001001000", "11001000100", "11000100100"
  , "10110011100", "10011011100", "10011001110", "10111001100"
  , "10011101100", "10011100110", "11001110010", "11001011100"
  , "11001001110", "11011100100", "11001110100", "11101101110"
  , "11101001100", "11100101100", "11100100110", "11101100100"
  , "11100110100", "11100110010", "11011011000", "11011000110"
  , "11000110110", "10100011000", "10001011000", "10001000110"
  , "10110001000", "10001101000", "10001100010", "11010001000"
  , "11000101000", "11000100010", "10110111000", "10110001110"
  , "10001101110", "10111011000", "10111000110", "10001110110"
  , "11101110110", "11010001110", "11000101110", "11011101000"
  , "11011100010", "11011101110", "11101011000", "11101000110"
  , "11100010110", "11101101000", "11101100010", "11100011010"
  , "11101111010", "11001000010", "11110001010", "10100110000"
  , "10100001100", "10010110000", "10010000110", "10000101100"
  , "10000100110", "10110010000", "10110000100", "10011010000"
  , "10011000010", "10000110100", "10000110010", "11000010010"
  , "11001010000", "11110111010", "11000010100", "10001111010"
  , "10100111100", "10010111100", "10010011110", "10111100100"
  , "10011110100", "10011110010", "11110100100", "11110010100"
  , "11110010010", "11011011110", "11011110110", "11110110110"
  , "10101111000", "10100011110", "10001011110", "10111101000"
  , "10111100010", "11110101000", "11110100010", "10111011110"
  , "10111101110", "11101011110", "11110101110", "11010000100"
  , "11010010000", "11010011100", "1100011101011"
  ]

-- | Preserve valid Code Set B text and reject everything outside printable
-- ASCII.
normalizeCode128B :: String -> Either Code128Error String
normalizeCode128B input = traverse validate input
  where
    validate character
      | characterCode >= 32 && characterCode <= 126 = Right character
      | otherwise = Left (InvalidCode128Character character)
      where
        characterCode = ord character

-- | Map one printable ASCII character to its Code Set B value.
code128BValue :: Char -> Either Code128Error Int
code128BValue character = do
  _ <- normalizeCode128B [character]
  Right (ord character - 32)

-- | Look up any regular, control, start, or stop pattern by numeric value.
code128Pattern :: Int -> Either Code128Error String
code128Pattern value
  | value < 0 || value > stop = Left (InvalidCode128Value value)
  | otherwise = case drop value code128Patterns of
      patternText : _ -> Right patternText
      [] -> Left (InvalidCode128Value value)

-- | Compute the required weighted modulo-103 checksum for Code Set B values.
computeCode128Checksum :: [Int] -> Int
computeCode128Checksum values =
  (startB + sum (zipWith (*) [1 :: Int ..] values)) `mod` 103

-- | Encode input as Start B, zero or more data symbols, checksum, and stop.
encodeCode128B :: String -> Either Code128Error [EncodedCode128Symbol]
encodeCode128B input = do
  normalized <- normalizeCode128B input
  encodeNormalized normalized

-- | Expand each symbol into alternating shared barcode runs.
expandCode128Runs :: String -> Either Code128Error [Barcode1DRun]
expandCode128Runs input = encodeCode128B input >>= expandEncoded

-- | Render Code 128 through the shared backend-neutral 1D geometry.
layoutCode128
  :: String
  -> PaintBarcode1DOptions
  -> Either Code128Error PaintScene
layoutCode128 input options = do
  normalized <- normalizeCode128B input
  encoded <- encodeNormalized normalized
  runs <- expandEncoded encoded
  let checksum = computeCode128Checksum
        [ encodedCode128Value symbol
        | symbol <- encoded
        , encodedCode128Role symbol == Data
        ]
  first Code128LayoutError
    (layoutBarcode1D runs (code128Options normalized checksum options))

-- | Compatibility alias matching the shared public API name.
drawCode128
  :: String
  -> PaintBarcode1DOptions
  -> Either Code128Error PaintScene
drawCode128 = layoutCode128

encodeNormalized :: String -> Either Code128Error [EncodedCode128Symbol]
encodeNormalized normalized = do
  dataSymbols <- traverse encodeData (zip [0 :: Int ..] normalized)
  startPattern <- code128Pattern startB
  let checksum = computeCode128Checksum (map encodedCode128Value dataSymbols)
  checksumPattern <- code128Pattern checksum
  stopPattern <- code128Pattern stop
  Right
    ( EncodedCode128Symbol "Start B" startB startPattern (-1) Start
    : dataSymbols
      ++ [ EncodedCode128Symbol
            ("Checksum " ++ show checksum)
            checksum
            checksumPattern
            (length normalized)
            Check
         , EncodedCode128Symbol
            "Stop"
            stop
            stopPattern
            (length normalized + 1)
            Stop
         ]
    )
  where
    encodeData (sourceIndex, character) = do
      value <- code128BValue character
      patternText <- code128Pattern value
      Right EncodedCode128Symbol
        { encodedCode128Label = [character]
        , encodedCode128Value = value
        , encodedCode128Pattern = patternText
        , encodedCode128SourceIndex = sourceIndex
        , encodedCode128Role = Data
        }

expandEncoded
  :: [EncodedCode128Symbol]
  -> Either Code128Error [Barcode1DRun]
expandEncoded encoded = concat <$> traverse expandOne encoded
  where
    expandOne symbol = first Code128LayoutError
      (runsFromBinaryPattern
        (encodedCode128Pattern symbol)
        (defaultBinaryPatternOptions
          (encodedCode128Label symbol)
          (encodedCode128SourceIndex symbol)
          (encodedCode128Role symbol)))

code128Options
  :: String
  -> Int
  -> PaintBarcode1DOptions
  -> PaintBarcode1DOptions
code128Options normalized checksum options = options
  { optionsLabel = Just (maybe defaultLabel id (optionsLabel options))
  , optionsMetadata = Map.insert "encodedText" (toJSON normalized)
      (Map.insert "checksum" (toJSON checksum)
        (Map.insert "codeSet" (toJSON ("B" :: String))
          (Map.insert "symbology" (toJSON ("code128" :: String))
            (optionsMetadata options))))
  }
  where
    defaultLabel = "Code 128 barcode for " ++ normalized
