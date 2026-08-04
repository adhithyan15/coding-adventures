-- | Pure EAN-13 barcode encoding.
module CodingAdventures.Ean13
  ( Ean13Encoding (..)
  , EncodedEan13Digit (..)
  , Ean13Error (..)
  , Barcode1DError (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , PaintScene (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , ean13DigitPattern
  , computeEan13CheckDigit
  , normalizeEan13
  , leftParityPattern
  , encodeEan13
  , expandEan13Runs
  , layoutEan13
  , drawEan13
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

-- | The pattern family used for one visible EAN-13 digit.
data Ean13Encoding = Ean13L | Ean13G | Ean13R
  deriving (Eq, Show)

-- | One visible digit and its selected seven-module pattern.
data EncodedEan13Digit = EncodedEan13Digit
  { encodedEan13Digit :: Char
  , encodedEan13Encoding :: Ean13Encoding
  , encodedEan13Pattern :: String
  , encodedEan13SourceIndex :: Int
  , encodedEan13Role :: Barcode1DRunRole
  } deriving (Eq, Show)

-- | Checked failures from input, checksum, pattern, or shared layout rules.
data Ean13Error
  = InvalidEan13Character Int Char
  | InvalidEan13Length Int
  | InvalidEan13CheckDigit Char Char
  | Ean13LayoutError Barcode1DError
  deriving (Eq)

instance Show Ean13Error where
  show (InvalidEan13Character index character) =
    "Invalid EAN-13 character " ++ show [character]
      ++ " at index " ++ show index ++ "; expected an ASCII digit"
  show (InvalidEan13Length actualLength) =
    "Invalid EAN-13 length " ++ show actualLength
      ++ "; expected 12 payload digits or 13 complete digits"
  show (InvalidEan13CheckDigit expected actual) =
    "Invalid EAN-13 check digit: expected " ++ [expected]
      ++ " but received " ++ [actual]
  show (Ean13LayoutError layoutError) = show layoutError

sideGuard :: String
sideGuard = "101"

centerGuard :: String
centerGuard = "01010"

leftPatterns :: [String]
leftPatterns =
  [ "0001101", "0011001", "0010011", "0111101", "0100011"
  , "0110001", "0101111", "0111011", "0110111", "0001011"
  ]

evenPatterns :: [String]
evenPatterns =
  [ "0100111", "0110011", "0011011", "0100001", "0011101"
  , "0111001", "0000101", "0010001", "0001001", "0010111"
  ]

rightPatterns :: [String]
rightPatterns =
  [ "1110010", "1100110", "1101100", "1000010", "1011100"
  , "1001110", "1010000", "1000100", "1001000", "1110100"
  ]

leftParityEncodings :: [[Ean13Encoding]]
leftParityEncodings =
  [ replicate 6 Ean13L
  , [Ean13L, Ean13L, Ean13G, Ean13L, Ean13G, Ean13G]
  , [Ean13L, Ean13L, Ean13G, Ean13G, Ean13L, Ean13G]
  , [Ean13L, Ean13L, Ean13G, Ean13G, Ean13G, Ean13L]
  , [Ean13L, Ean13G, Ean13L, Ean13L, Ean13G, Ean13G]
  , [Ean13L, Ean13G, Ean13G, Ean13L, Ean13L, Ean13G]
  , [Ean13L, Ean13G, Ean13G, Ean13G, Ean13L, Ean13L]
  , [Ean13L, Ean13G, Ean13L, Ean13G, Ean13L, Ean13G]
  , [Ean13L, Ean13G, Ean13L, Ean13G, Ean13G, Ean13L]
  , [Ean13L, Ean13G, Ean13G, Ean13L, Ean13G, Ean13L]
  ]

-- | Look up one of the thirty standard seven-module digit patterns.
ean13DigitPattern :: Ean13Encoding -> Char -> Either Ean13Error String
ean13DigitPattern encoding digit = do
  value <- digitValue 0 digit
  case drop value patterns of
    patternText : _ -> Right patternText
    [] -> Left (InvalidEan13Character 0 digit)
  where
    patterns = case encoding of
      Ean13L -> leftPatterns
      Ean13G -> evenPatterns
      Ean13R -> rightPatterns

-- | Compute the modulo-10 check digit for exactly twelve payload digits.
computeEan13CheckDigit :: String -> Either Ean13Error Char
computeEan13CheckDigit payload = do
  validateDigits [12] payload
  values <- traverse (uncurry digitValue) (zip [0 :: Int ..] payload)
  let total = sum
        [ value * if even index then 1 else 3
        | (index, value) <- zip [0 :: Int ..] values
        ]
  Right (toDigit ((10 - total `mod` 10) `mod` 10))

-- | Append a missing check digit or verify the supplied thirteenth digit.
normalizeEan13 :: String -> Either Ean13Error String
normalizeEan13 input = do
  validateDigits [12, 13] input
  expected <- computeEan13CheckDigit (take 12 input)
  case drop 12 input of
    [] -> Right (input ++ [expected])
    actual : _
      | actual == expected -> Right input
      | otherwise -> Left (InvalidEan13CheckDigit expected actual)

-- | Return the six L/G choices selected indirectly by the leading digit.
leftParityPattern :: String -> Either Ean13Error String
leftParityPattern input = do
  normalized <- normalizeEan13 input
  encodings <- parityForLeadingDigit (head normalized)
  Right (map encodingMarker encodings)

-- | Encode the twelve visible digits. The leading digit selects parity and
-- occupies no modules of its own.
encodeEan13 :: String -> Either Ean13Error [EncodedEan13Digit]
encodeEan13 input = normalizeEan13 input >>= encodeNormalized

-- | Expand the start, visible digits, center, and end into attributed runs.
expandEan13Runs :: String -> Either Ean13Error [Barcode1DRun]
expandEan13Runs input = encodeEan13 input >>= expandEncoded

-- | Render EAN-13 through the shared backend-neutral 1D geometry.
layoutEan13
  :: String
  -> PaintBarcode1DOptions
  -> Either Ean13Error PaintScene
layoutEan13 input options = do
  normalized <- normalizeEan13 input
  parity <- leftParityPattern normalized
  encoded <- encodeNormalized normalized
  runs <- expandEncoded encoded
  let checkDigit = encodedEan13Digit (last encoded)
  first Ean13LayoutError
    (layoutBarcode1D runs
      (ean13Options normalized parity checkDigit encoded options))

-- | Compatibility alias matching the shared public API name.
drawEan13
  :: String
  -> PaintBarcode1DOptions
  -> Either Ean13Error PaintScene
drawEan13 = layoutEan13

validateDigits :: [Int] -> String -> Either Ean13Error ()
validateDigits expectedLengths input = case findIndex (not . isAsciiDigit) input of
  Just index -> Left (InvalidEan13Character index (input !! index))
  Nothing
    | length input `elem` expectedLengths -> Right ()
    | otherwise -> Left (InvalidEan13Length (length input))

isAsciiDigit :: Char -> Bool
isAsciiDigit character = character >= '0' && character <= '9'

digitValue :: Int -> Char -> Either Ean13Error Int
digitValue index character
  | isAsciiDigit character = Right (ord character - ord '0')
  | otherwise = Left (InvalidEan13Character index character)

toDigit :: Int -> Char
toDigit value = toEnum (ord '0' + value)

parityForLeadingDigit :: Char -> Either Ean13Error [Ean13Encoding]
parityForLeadingDigit leadingDigit = do
  value <- digitValue 0 leadingDigit
  case drop value leftParityEncodings of
    encodings : _ -> Right encodings
    [] -> Left (InvalidEan13Character 0 leadingDigit)

encodingMarker :: Ean13Encoding -> Char
encodingMarker encoding = case encoding of
  Ean13L -> 'L'
  Ean13G -> 'G'
  Ean13R -> 'R'

encodeNormalized :: String -> Either Ean13Error [EncodedEan13Digit]
encodeNormalized normalized = do
  parity <- parityForLeadingDigit (head normalized)
  leftEncoded <- traverse encodeLeft
    (zip3 [1 :: Int ..] (take 6 (drop 1 normalized)) parity)
  rightEncoded <- traverse encodeRight
    (zip [7 :: Int ..] (drop 7 normalized))
  Right (leftEncoded ++ rightEncoded)
  where
    encodeLeft (sourceIndex, digit, encoding) =
      encodeOne sourceIndex digit encoding Data
    encodeRight (sourceIndex, digit) =
      encodeOne sourceIndex digit Ean13R
        (if sourceIndex == 12 then Check else Data)
    encodeOne sourceIndex digit encoding role = do
      patternText <- ean13DigitPattern encoding digit
      Right EncodedEan13Digit
        { encodedEan13Digit = digit
        , encodedEan13Encoding = encoding
        , encodedEan13Pattern = patternText
        , encodedEan13SourceIndex = sourceIndex
        , encodedEan13Role = role
        }

expandEncoded :: [EncodedEan13Digit] -> Either Ean13Error [Barcode1DRun]
expandEncoded encoded = do
  startRuns <- expandPattern "start" (-1) Guard sideGuard
  leftRuns <- concat <$> traverse expandDigit (take 6 encoded)
  middleRuns <- expandPattern "center" (-2) Guard centerGuard
  rightRuns <- concat <$> traverse expandDigit (drop 6 encoded)
  endRuns <- expandPattern "end" (-3) Guard sideGuard
  Right (startRuns ++ leftRuns ++ middleRuns ++ rightRuns ++ endRuns)
  where
    expandDigit entry = expandPattern
      [encodedEan13Digit entry]
      (encodedEan13SourceIndex entry)
      (encodedEan13Role entry)
      (encodedEan13Pattern entry)

expandPattern
  :: String
  -> Int
  -> Barcode1DRunRole
  -> String
  -> Either Ean13Error [Barcode1DRun]
expandPattern label sourceIndex role patternText = first Ean13LayoutError
  (runsFromBinaryPattern patternText
    (defaultBinaryPatternOptions label sourceIndex role))

buildSymbols :: [EncodedEan13Digit] -> [Barcode1DSymbolDescriptor]
buildSymbols encoded =
  [Barcode1DSymbolDescriptor "start" 3 (-1) SymbolGuard]
    ++ map digitDescriptor (take 6 encoded)
    ++ [Barcode1DSymbolDescriptor "center" 5 (-2) SymbolGuard]
    ++ map digitDescriptor (drop 6 encoded)
    ++ [Barcode1DSymbolDescriptor "end" 3 (-3) SymbolGuard]
  where
    digitDescriptor entry = Barcode1DSymbolDescriptor
      { descriptorLabel = [encodedEan13Digit entry]
      , descriptorModules = 7
      , descriptorSourceIndex = encodedEan13SourceIndex entry
      , descriptorRole = if encodedEan13Role entry == Check
          then SymbolCheck
          else SymbolData
      }

ean13Options
  :: String
  -> String
  -> Char
  -> [EncodedEan13Digit]
  -> PaintBarcode1DOptions
  -> PaintBarcode1DOptions
ean13Options normalized parity checkDigit encoded options = options
  { optionsLabel = Just (maybe defaultLabel id (optionsLabel options))
  , optionsMetadata = Map.insert "checkDigit" (toJSON [checkDigit])
      (Map.insert "leftParity" (toJSON parity)
        (Map.insert "leadingDigit" (toJSON (take 1 normalized))
          (Map.insert "encodedText" (toJSON normalized)
            (Map.insert "symbology" (toJSON ("ean-13" :: String))
              (optionsMetadata options)))))
  , optionsSymbols = Just (buildSymbols encoded)
  }
  where
    defaultLabel = "EAN-13 barcode for " ++ normalized
