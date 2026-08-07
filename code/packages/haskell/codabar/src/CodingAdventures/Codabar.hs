-- | Pure Codabar barcode encoding.
module CodingAdventures.Codabar
  ( EncodedCodabarSymbol (..)
  , CodabarGuards (..)
  , CodabarError (..)
  , Barcode1DError (..)
  , Barcode1DRun (..)
  , Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , PaintScene (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , defaultCodabarGuards
  , normalizeCodabar
  , encodeCodabar
  , expandCodabarRuns
  , layoutCodabar
  , drawCodabar
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
import Data.Char (toUpper)
import qualified Data.Map.Strict as Map

-- | Package version shared with the established implementations.
version :: String
version = "0.1.0"

-- | Start and stop symbols used when the payload does not provide its own.
data CodabarGuards = CodabarGuards
  { codabarStartGuard :: Char
  , codabarStopGuard :: Char
  } deriving (Eq, Show)

-- | The standard default wraps body data in @A ... A@.
defaultCodabarGuards :: CodabarGuards
defaultCodabarGuards = CodabarGuards 'A' 'A'

-- | One normalized symbol and its fully expanded binary module pattern.
data EncodedCodabarSymbol = EncodedCodabarSymbol
  { encodedCodabarCharacter :: Char
  , encodedCodabarPattern :: String
  , encodedCodabarSourceIndex :: Int
  , encodedCodabarRole :: Barcode1DRunRole
  } deriving (Eq, Show)

-- | Checked failures from guard, body, or shared layout validation.
data CodabarError
  = InvalidCodabarBodyCharacter Char
  | InvalidCodabarGuard Char
  | CodabarLayoutError Barcode1DError
  deriving (Eq)

instance Show CodabarError where
  show (InvalidCodabarBodyCharacter character) =
    "Invalid Codabar body character " ++ show [character]
  show (InvalidCodabarGuard character) =
    "Invalid Codabar guard " ++ show [character]
      ++ "; expected A, B, C, or D"
  show (CodabarLayoutError layoutError) = show layoutError

codabarPatterns :: Map.Map Char String
codabarPatterns = Map.fromList
  [ ('0', "101010011")
  , ('1', "101011001")
  , ('2', "101001011")
  , ('3', "110010101")
  , ('4', "101101001")
  , ('5', "110101001")
  , ('6', "100101011")
  , ('7', "100101101")
  , ('8', "100110101")
  , ('9', "110100101")
  , ('-', "101001101")
  , ('$', "101100101")
  , (':', "1101011011")
  , ('/', "1101101011")
  , ('.', "1101101101")
  , ('+', "1011011011")
  , ('A', "1011001001")
  , ('B', "1001001011")
  , ('C', "1010010011")
  , ('D', "1010011001")
  ]

-- | Uppercase input, preserve valid explicit guards, or insert configured
-- guards around a body-only payload.
normalizeCodabar :: String -> CodabarGuards -> Either CodabarError String
normalizeCodabar input guards = case explicitBody normalized of
  Just body -> validateBody body >> Right normalized
  Nothing -> do
      validateGuard (codabarStartGuard guards)
      validateGuard (codabarStopGuard guards)
      validateBody normalized
      Right (codabarStartGuard guards : normalized ++ [codabarStopGuard guards])
  where
    normalized = map toUpper input

-- | Encode normalized body data and its two guards.
encodeCodabar
  :: String
  -> CodabarGuards
  -> Either CodabarError [EncodedCodabarSymbol]
encodeCodabar input guards = normalizeCodabar input guards >>= encodeNormalized

-- | Expand every symbol into shared runs with one-module inter-character gaps.
expandCodabarRuns
  :: String
  -> CodabarGuards
  -> Either CodabarError [Barcode1DRun]
expandCodabarRuns input guards = do
  encoded <- encodeCodabar input guards
  expandEncoded encoded

-- | Render Codabar through the shared backend-neutral 1D geometry.
layoutCodabar
  :: String
  -> CodabarGuards
  -> PaintBarcode1DOptions
  -> Either CodabarError PaintScene
layoutCodabar input guards options = do
  normalized <- normalizeCodabar input guards
  encoded <- encodeNormalized normalized
  runs <- expandEncoded encoded
  first CodabarLayoutError
    (layoutBarcode1D runs (codabarOptions normalized options))

-- | Compatibility alias matching the shared public API name.
drawCodabar
  :: String
  -> CodabarGuards
  -> PaintBarcode1DOptions
  -> Either CodabarError PaintScene
drawCodabar = layoutCodabar

isGuard :: Char -> Bool
isGuard character = character `elem` "ABCD"

explicitBody :: String -> Maybe String
explicitBody normalized = case normalized of
  firstCharacter : rest -> case reverse rest of
    stopCharacter : reversedBody
      | isGuard firstCharacter && isGuard stopCharacter ->
          Just (reverse reversedBody)
    _ -> Nothing
  [] -> Nothing

validateGuard :: Char -> Either CodabarError ()
validateGuard character
  | isGuard character = Right ()
  | otherwise = Left (InvalidCodabarGuard character)

validateBody :: String -> Either CodabarError ()
validateBody [] = Right ()
validateBody (character : rest)
  | Map.member character codabarPatterns && not (isGuard character) =
      validateBody rest
  | otherwise = Left (InvalidCodabarBodyCharacter character)

encodeNormalized :: String -> Either CodabarError [EncodedCodabarSymbol]
encodeNormalized normalized = traverse encodeOne indexed
  where
    encodedLength = length normalized
    indexed = zip [0 :: Int ..] normalized
    encodeOne (sourceIndex, character) = case Map.lookup character codabarPatterns of
      Nothing -> Left (InvalidCodabarBodyCharacter character)
      Just patternText -> Right EncodedCodabarSymbol
        { encodedCodabarCharacter = character
        , encodedCodabarPattern = patternText
        , encodedCodabarSourceIndex = sourceIndex
        , encodedCodabarRole = roleFor encodedLength sourceIndex
        }

roleFor :: Int -> Int -> Barcode1DRunRole
roleFor encodedLength sourceIndex
  | sourceIndex == 0 = Start
  | sourceIndex == encodedLength - 1 = Stop
  | otherwise = Data

expandEncoded
  :: [EncodedCodabarSymbol]
  -> Either CodabarError [Barcode1DRun]
expandEncoded encoded = concat <$> traverse expandOne indexed
  where
    encodedLength = length encoded
    indexed = zip [0 :: Int ..] encoded
    expandOne (encodedIndex, symbol) = do
      symbolRuns <- first CodabarLayoutError
        (runsFromBinaryPattern
          (encodedCodabarPattern symbol)
          (defaultBinaryPatternOptions
            [encodedCodabarCharacter symbol]
            (encodedCodabarSourceIndex symbol)
            (encodedCodabarRole symbol)))
      let gap
            | encodedIndex < encodedLength - 1 =
                [ Barcode1DRun
                    { runColor = Space
                    , runModules = 1
                    , runSourceLabel = [encodedCodabarCharacter symbol]
                    , runSourceIndex = encodedCodabarSourceIndex symbol
                    , runRole = InterCharacterGap
                    }
                ]
            | otherwise = []
      Right (symbolRuns ++ gap)

codabarOptions
  :: String
  -> PaintBarcode1DOptions
  -> PaintBarcode1DOptions
codabarOptions normalized options = options
  { optionsLabel = Just (maybe defaultLabel id (optionsLabel options))
  , optionsMetadata = Map.insert "encodedText" (toJSON normalized)
      (Map.insert "stop" (toJSON (take 1 (reverse normalized)))
        (Map.insert "start" (toJSON (take 1 normalized))
          (Map.insert "symbology" (toJSON ("codabar" :: String))
            (optionsMetadata options))))
  }
  where
    defaultLabel = "Codabar barcode for " ++ normalized
