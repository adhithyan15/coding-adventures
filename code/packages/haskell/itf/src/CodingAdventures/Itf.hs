-- | Pure Interleaved 2 of 5 barcode encoding.
module CodingAdventures.Itf
  ( EncodedPair (..)
  , ItfError (..)
  , Barcode1DError (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , PaintScene (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , normalizeItf
  , encodeItf
  , expandItfRuns
  , layoutItf
  , drawItf
  , version
  ) where

import CodingAdventures.BarcodeLayout1D
  ( Barcode1DError (..)
  , Barcode1DRenderConfig (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DSymbolDescriptor (..)
  , Barcode1DSymbolRole (..)
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
import qualified Data.Map.Strict as Map

-- | Package version shared with the established implementations.
version :: String
version = "0.1.0"

-- | The two digits and their fully expanded interleaved pattern.
data EncodedPair = EncodedPair
  { encodedPairDigits :: String
  , encodedPairBarPattern :: String
  , encodedPairSpacePattern :: String
  , encodedPairBinaryPattern :: String
  , encodedPairSourceIndex :: Int
  } deriving (Eq, Show)

-- | Checked failures from validation or the shared layout layer.
data ItfError
  = InvalidItfInput String
  | ItfLayoutError Barcode1DError
  deriving (Eq, Show)

startPattern :: String
startPattern = "1010"

stopPattern :: String
stopPattern = "11101"

digitPatterns :: [String]
digitPatterns =
  [ "00110"
  , "10001"
  , "01001"
  , "11000"
  , "00101"
  , "10100"
  , "01100"
  , "00011"
  , "10010"
  , "01010"
  ]

-- | Require a non-empty, even-length string of ASCII digits.
normalizeItf :: String -> Either ItfError String
normalizeItf input
  | null input || any (not . isAsciiDigit) input =
      Left (InvalidItfInput "ITF input must contain digits only")
  | odd (length input) =
      Left (InvalidItfInput "ITF input must contain an even number of digits")
  | otherwise = Right input

-- | Encode adjacent digits as interleaved bar and space widths.
encodeItf :: String -> Either ItfError [EncodedPair]
encodeItf input = encodeNormalized <$> normalizeItf input

-- | Expand the start marker, digit pairs, and stop marker into shared runs.
expandItfRuns :: String -> Either ItfError [Barcode1DRun]
expandItfRuns input = do
  pairs <- encodeItf input
  runsForPairs pairs

-- | Render ITF through the shared backend-neutral 1D barcode geometry.
layoutItf
  :: String
  -> PaintBarcode1DOptions
  -> Either ItfError PaintScene
layoutItf input options = do
  normalized <- normalizeItf input
  let pairs = encodeNormalized normalized
  runs <- runsForPairs pairs
  first ItfLayoutError (layoutBarcode1D runs (itfOptions normalized pairs options))

-- | Compatibility alias matching the shared public API name.
drawItf
  :: String
  -> PaintBarcode1DOptions
  -> Either ItfError PaintScene
drawItf = layoutItf

isAsciiDigit :: Char -> Bool
isAsciiDigit character = character >= '0' && character <= '9'

encodeNormalized :: String -> [EncodedPair]
encodeNormalized = zipWith encodePair [0 :: Int ..] . pairsOfDigits

pairsOfDigits :: String -> [String]
pairsOfDigits [] = []
pairsOfDigits (firstDigit : secondDigit : rest) =
  [firstDigit, secondDigit] : pairsOfDigits rest
pairsOfDigits _ = []

encodePair :: Int -> String -> EncodedPair
encodePair sourceIndex pair = EncodedPair
  { encodedPairDigits = pair
  , encodedPairBarPattern = barPattern
  , encodedPairSpacePattern = spacePattern
  , encodedPairBinaryPattern = interleavePatterns barPattern spacePattern
  , encodedPairSourceIndex = sourceIndex
  }
  where
    barPattern = digitPattern (head pair)
    spacePattern = digitPattern (pair !! 1)

digitPattern :: Char -> String
digitPattern character = digitPatterns !! (fromEnum character - fromEnum '0')

interleavePatterns :: String -> String -> String
interleavePatterns bars spaces = concatMap expand (zip bars spaces)
  where
    expand (barMarker, spaceMarker) =
      (if barMarker == '1' then "111" else "1")
        ++ (if spaceMarker == '1' then "000" else "0")

runsForPairs :: [EncodedPair] -> Either ItfError [Barcode1DRun]
runsForPairs pairs = do
  startRuns <- patternRuns startPattern "start" (-1) Start
  pairRuns <- traverse runsForPair pairs
  stopRuns <- patternRuns stopPattern "stop" (-2) Stop
  Right (startRuns ++ concat pairRuns ++ stopRuns)

runsForPair :: EncodedPair -> Either ItfError [Barcode1DRun]
runsForPair pair = patternRuns
  (encodedPairBinaryPattern pair)
  (encodedPairDigits pair)
  (encodedPairSourceIndex pair)
  Data

patternRuns
  :: String
  -> String
  -> Int
  -> Barcode1DRunRole
  -> Either ItfError [Barcode1DRun]
patternRuns patternText label sourceIndex role = first ItfLayoutError
  (runsFromBinaryPattern patternText
    (defaultBinaryPatternOptions label sourceIndex role))

itfOptions
  :: String
  -> [EncodedPair]
  -> PaintBarcode1DOptions
  -> PaintBarcode1DOptions
itfOptions normalized pairs options = options
  { optionsLabel = Just (maybe defaultLabel id (optionsLabel options))
  , optionsMetadata = Map.insert "pairCount" (toJSON (length pairs))
      (Map.insert "symbology" (toJSON ("itf" :: String)) (optionsMetadata options))
  , optionsSymbols = Just (symbolDescriptors pairs)
  }
  where
    defaultLabel = "ITF barcode for " ++ normalized

symbolDescriptors :: [EncodedPair] -> [Barcode1DSymbolDescriptor]
symbolDescriptors pairs =
  Barcode1DSymbolDescriptor "start" (length startPattern) (-1) SymbolStart
    : map pairDescriptor pairs
    ++ [Barcode1DSymbolDescriptor "stop" (length stopPattern) (-2) SymbolStop]

pairDescriptor :: EncodedPair -> Barcode1DSymbolDescriptor
pairDescriptor pair = Barcode1DSymbolDescriptor
  { descriptorLabel = encodedPairDigits pair
  , descriptorModules = length (encodedPairBinaryPattern pair)
  , descriptorSourceIndex = encodedPairSourceIndex pair
  , descriptorRole = SymbolData
  }
