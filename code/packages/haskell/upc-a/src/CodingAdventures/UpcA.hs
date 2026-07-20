-- | Pure UPC-A barcode encoding.
module CodingAdventures.UpcA
  ( UpcAEncoding (..)
  , EncodedUpcADigit (..)
  , UpcAError (..)
  , Barcode1DError (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , PaintScene (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , upcADigitPattern
  , computeUpcACheckDigit
  , normalizeUpcA
  , encodeUpcA
  , expandUpcARuns
  , layoutUpcA
  , drawUpcA
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
import Data.Char (ord)
import Data.List (findIndex)
import qualified Data.Map.Strict as Map

-- | Package version shared with the established implementations.
version :: String
version = "0.1.0"

-- | The side of the UPC-A symbol that supplies a digit's pattern.
data UpcAEncoding = LeftEncoding | RightEncoding
  deriving (Eq, Show)

-- | One normalized digit and its seven-module pattern.
data EncodedUpcADigit = EncodedUpcADigit
  { encodedUpcADigit :: Char
  , encodedUpcAEncoding :: UpcAEncoding
  , encodedUpcAPattern :: String
  , encodedUpcASourceIndex :: Int
  , encodedUpcARole :: Barcode1DRunRole
  } deriving (Eq, Show)

-- | Checked failures from input, checksum, pattern, or shared layout rules.
data UpcAError
  = InvalidUpcACharacter Int Char
  | InvalidUpcALength Int
  | InvalidUpcACheckDigit Char Char
  | UpcALayoutError Barcode1DError
  deriving (Eq)

instance Show UpcAError where
  show (InvalidUpcACharacter index character) =
    "Invalid UPC-A character " ++ show [character]
      ++ " at index " ++ show index ++ "; expected an ASCII digit"
  show (InvalidUpcALength actualLength) =
    "Invalid UPC-A length " ++ show actualLength
      ++ "; expected 11 payload digits or 12 complete digits"
  show (InvalidUpcACheckDigit expected actual) =
    "Invalid UPC-A check digit: expected " ++ [expected]
      ++ " but received " ++ [actual]
  show (UpcALayoutError layoutError) = show layoutError

sideGuard :: String
sideGuard = "101"

centerGuard :: String
centerGuard = "01010"

leftPatterns :: [String]
leftPatterns =
  [ "0001101", "0011001", "0010011", "0111101", "0100011"
  , "0110001", "0101111", "0111011", "0110111", "0001011"
  ]

rightPatterns :: [String]
rightPatterns =
  [ "1110010", "1100110", "1101100", "1000010", "1011100"
  , "1001110", "1010000", "1000100", "1001000", "1110100"
  ]

-- | Look up one of the twenty standard seven-module digit patterns.
upcADigitPattern :: UpcAEncoding -> Char -> Either UpcAError String
upcADigitPattern encoding digit = do
  value <- digitValue 0 digit
  case drop value patterns of
    patternText : _ -> Right patternText
    [] -> Left (InvalidUpcACharacter 0 digit)
  where
    patterns = case encoding of
      LeftEncoding -> leftPatterns
      RightEncoding -> rightPatterns

-- | Compute the modulo-10 check digit for exactly eleven payload digits.
computeUpcACheckDigit :: String -> Either UpcAError Char
computeUpcACheckDigit payload = do
  validateDigits [11] payload
  values <- traverse (uncurry digitValue) (zip [0 :: Int ..] payload)
  let (oddValues, evenValues) = splitAlternating values
      total = 3 * sum oddValues + sum evenValues
  Right (toDigit ((10 - total `mod` 10) `mod` 10))

-- | Append a missing check digit or verify the supplied twelfth digit.
normalizeUpcA :: String -> Either UpcAError String
normalizeUpcA input = do
  validateDigits [11, 12] input
  expected <- computeUpcACheckDigit (take 11 input)
  case drop 11 input of
    [] -> Right (input ++ [expected])
    actual : _
      | actual == expected -> Right input
      | otherwise -> Left (InvalidUpcACheckDigit expected actual)

-- | Encode the twelve normalized digits using six left and six right patterns.
encodeUpcA :: String -> Either UpcAError [EncodedUpcADigit]
encodeUpcA input = normalizeUpcA input >>= encodeNormalized

-- | Expand the start, digits, center, and end into an attributed run stream.
expandUpcARuns :: String -> Either UpcAError [Barcode1DRun]
expandUpcARuns input = encodeUpcA input >>= expandEncoded

-- | Render UPC-A through the shared backend-neutral 1D geometry.
layoutUpcA
  :: String
  -> PaintBarcode1DOptions
  -> Either UpcAError PaintScene
layoutUpcA input options = do
  normalized <- normalizeUpcA input
  encoded <- encodeNormalized normalized
  runs <- expandEncoded encoded
  let checkDigit = encodedUpcADigit (last encoded)
  first UpcALayoutError
    (layoutBarcode1D runs (upcAOptions normalized checkDigit encoded options))

-- | Compatibility alias matching the shared public API name.
drawUpcA
  :: String
  -> PaintBarcode1DOptions
  -> Either UpcAError PaintScene
drawUpcA = layoutUpcA

validateDigits :: [Int] -> String -> Either UpcAError ()
validateDigits expectedLengths input = case findIndex (not . isAsciiDigit) input of
  Just index -> Left (InvalidUpcACharacter index (input !! index))
  Nothing
    | length input `elem` expectedLengths -> Right ()
    | otherwise -> Left (InvalidUpcALength (length input))

isAsciiDigit :: Char -> Bool
isAsciiDigit character = character >= '0' && character <= '9'

digitValue :: Int -> Char -> Either UpcAError Int
digitValue index character
  | isAsciiDigit character = Right (ord character - ord '0')
  | otherwise = Left (InvalidUpcACharacter index character)

toDigit :: Int -> Char
toDigit value = toEnum (ord '0' + value)

splitAlternating :: [value] -> ([value], [value])
splitAlternating values =
  ( [value | (index, value) <- indexed, even index]
  , [value | (index, value) <- indexed, odd index]
  )
  where
    indexed = zip [0 :: Int ..] values

encodeNormalized :: String -> Either UpcAError [EncodedUpcADigit]
encodeNormalized normalized = traverse encodeOne (zip [0 :: Int ..] normalized)
  where
    encodeOne (sourceIndex, digit) = do
      let encoding
            | sourceIndex < 6 = LeftEncoding
            | otherwise = RightEncoding
          role
            | sourceIndex == 11 = Check
            | otherwise = Data
      patternText <- upcADigitPattern encoding digit
      Right EncodedUpcADigit
        { encodedUpcADigit = digit
        , encodedUpcAEncoding = encoding
        , encodedUpcAPattern = patternText
        , encodedUpcASourceIndex = sourceIndex
        , encodedUpcARole = role
        }

expandEncoded :: [EncodedUpcADigit] -> Either UpcAError [Barcode1DRun]
expandEncoded encoded = do
  startRuns <- expandPattern "start" (-1) Guard sideGuard
  leftRuns <- concat <$> traverse expandDigit (take 6 encoded)
  middleRuns <- expandPattern "center" (-2) Guard centerGuard
  rightRuns <- concat <$> traverse expandDigit (drop 6 encoded)
  endRuns <- expandPattern "end" (-3) Guard sideGuard
  Right (startRuns ++ leftRuns ++ middleRuns ++ rightRuns ++ endRuns)
  where
    expandDigit entry = expandPattern
      [encodedUpcADigit entry]
      (encodedUpcASourceIndex entry)
      (encodedUpcARole entry)
      (encodedUpcAPattern entry)

expandPattern
  :: String
  -> Int
  -> Barcode1DRunRole
  -> String
  -> Either UpcAError [Barcode1DRun]
expandPattern label sourceIndex role patternText = first UpcALayoutError
  (runsFromBinaryPattern patternText
    (defaultBinaryPatternOptions label sourceIndex role))

buildSymbols :: [EncodedUpcADigit] -> [Barcode1DSymbolDescriptor]
buildSymbols encoded =
  [Barcode1DSymbolDescriptor "start" 3 (-1) SymbolGuard]
    ++ map digitDescriptor (take 6 encoded)
    ++ [Barcode1DSymbolDescriptor "center" 5 (-2) SymbolGuard]
    ++ map digitDescriptor (drop 6 encoded)
    ++ [Barcode1DSymbolDescriptor "end" 3 (-3) SymbolGuard]
  where
    digitDescriptor entry = Barcode1DSymbolDescriptor
      { descriptorLabel = [encodedUpcADigit entry]
      , descriptorModules = 7
      , descriptorSourceIndex = encodedUpcASourceIndex entry
      , descriptorRole = if encodedUpcARole entry == Check
          then SymbolCheck
          else SymbolData
      }

upcAOptions
  :: String
  -> Char
  -> [EncodedUpcADigit]
  -> PaintBarcode1DOptions
  -> PaintBarcode1DOptions
upcAOptions normalized checkDigit encoded options = options
  { optionsLabel = Just (maybe defaultLabel id (optionsLabel options))
  , optionsMetadata = Map.insert "checkDigit" (toJSON [checkDigit])
      (Map.insert "encodedText" (toJSON normalized)
        (Map.insert "symbology" (toJSON ("upc-a" :: String))
          (optionsMetadata options)))
  , optionsSymbols = Just (buildSymbols encoded)
  }
  where
    defaultLabel = "UPC-A barcode for " ++ normalized
