-- | Pure coordinator for the repository's one-dimensional barcode packages.
module CodingAdventures.Barcode1D
  ( Symbology (..)
  , Barcode1DOptions (..)
  , Barcode1DPipelineError (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , CodabarGuards (..)
  , AsciiOptions (..)
  , PaintScene (..)
  , PaintVmAsciiError (..)
  , Code39Error (..)
  , CodabarError (..)
  , Code128Error (..)
  , Ean13Error (..)
  , ItfError (..)
  , UpcAError (..)
  , defaultBarcode1DOptions
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , defaultCodabarGuards
  , defaultAsciiOptions
  , defaultRenderConfig
  , currentBackend
  , normalizeSymbology
  , buildScene
  , buildSceneForSymbology
  , renderAscii
  , renderAsciiForSymbology
  , renderAsciiWithOptions
  , version
  ) where

import CodingAdventures.BarcodeLayout1D
  ( Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  )
import CodingAdventures.Codabar
  ( CodabarError (..)
  , CodabarGuards (..)
  , defaultCodabarGuards
  )
import qualified CodingAdventures.Codabar as Codabar
import CodingAdventures.Code128 (Code128Error (..))
import qualified CodingAdventures.Code128 as Code128
import CodingAdventures.Code39 (Code39Error (..))
import qualified CodingAdventures.Code39 as Code39
import CodingAdventures.Ean13 (Ean13Error (..))
import qualified CodingAdventures.Ean13 as Ean13
import CodingAdventures.Itf (ItfError (..))
import qualified CodingAdventures.Itf as Itf
import CodingAdventures.PaintInstructions (PaintScene (..))
import CodingAdventures.PaintVmAscii
  ( AsciiOptions (..)
  , PaintVmAsciiError (..)
  , defaultAsciiOptions
  )
import qualified CodingAdventures.PaintVmAscii as PaintVmAscii
import CodingAdventures.UpcA (UpcAError (..))
import qualified CodingAdventures.UpcA as UpcA
import Data.Bifunctor (first)
import Data.Char (isSpace, toLower)
import Data.List (dropWhileEnd)

-- | Package version shared with the other language implementations.
version :: String
version = "0.1.0"

-- | The native encoders available through the coordinator.
data Symbology
  = Codabar
  | Code128
  | Code39
  | Ean13
  | Itf
  | UpcA
  deriving (Eq, Show)

-- | Encoder selection, shared paint options, and Codabar-specific guards.
data Barcode1DOptions = Barcode1DOptions
  { barcodeSymbology :: Symbology
  , barcodePaintOptions :: PaintBarcode1DOptions
  , barcodeCodabarGuards :: CodabarGuards
  } deriving (Eq, Show)

-- | Checked failures retain the originating package and error value.
data Barcode1DPipelineError
  = UnsupportedSymbology String
  | Code39Failure Code39Error
  | CodabarFailure CodabarError
  | Code128Failure Code128Error
  | Ean13Failure Ean13Error
  | ItfFailure ItfError
  | UpcAFailure UpcAError
  | AsciiBackendFailure PaintVmAsciiError
  deriving (Eq, Show)

-- | Code 39 with the shared paint defaults and standard Codabar guards.
defaultBarcode1DOptions :: Barcode1DOptions
defaultBarcode1DOptions = Barcode1DOptions
  { barcodeSymbology = Code39
  , barcodePaintOptions = defaultPaintBarcode1DOptions
  , barcodeCodabarGuards = defaultCodabarGuards
  }

-- | The cross-language geometry and color defaults.
defaultRenderConfig :: Barcode1DRenderConfig
defaultRenderConfig = defaultBarcode1DRenderConfig

-- | The only pure Haskell Paint VM currently wired into this package.
currentBackend :: String
currentBackend = "ascii"

-- | Normalize a case-insensitive name while ignoring hyphens and underscores.
-- An empty name selects Code 39, matching the other coordinator packages.
normalizeSymbology :: String -> Either Barcode1DPipelineError Symbology
normalizeSymbology input = case normalized of
  "codabar" -> Right Codabar
  "code128" -> Right Code128
  "code39" -> Right Code39
  "ean13" -> Right Ean13
  "itf" -> Right Itf
  "upca" -> Right UpcA
  _ -> Left (UnsupportedSymbology input)
  where
    compact = filter (`notElem` "-_") (map toLower (trim input))
    normalized = if null compact then "code39" else compact

-- | Build a backend-neutral scene with the selected native encoder.
buildScene
  :: String
  -> Barcode1DOptions
  -> Either Barcode1DPipelineError PaintScene
buildScene input options = case barcodeSymbology options of
  Codabar -> first CodabarFailure
    (Codabar.layoutCodabar input
      (barcodeCodabarGuards options)
      (barcodePaintOptions options))
  Code128 -> first Code128Failure
    (Code128.layoutCode128 input (barcodePaintOptions options))
  Code39 -> first Code39Failure
    (Code39.layoutCode39 input (barcodePaintOptions options))
  Ean13 -> first Ean13Failure
    (Ean13.layoutEan13 input (barcodePaintOptions options))
  Itf -> first ItfFailure
    (Itf.layoutItf input (barcodePaintOptions options))
  UpcA -> first UpcAFailure
    (UpcA.layoutUpcA input (barcodePaintOptions options))

-- | Normalize a user-facing name, then build a scene with that encoder.
buildSceneForSymbology
  :: String
  -> String
  -> Barcode1DOptions
  -> Either Barcode1DPipelineError PaintScene
buildSceneForSymbology symbology input options = do
  selected <- normalizeSymbology symbology
  buildScene input options { barcodeSymbology = selected }

-- | Build and render with the pure ASCII backend and its default scaling.
renderAscii
  :: String
  -> Barcode1DOptions
  -> Either Barcode1DPipelineError String
renderAscii input options = do
  scene <- buildScene input options
  first AsciiBackendFailure (PaintVmAscii.renderDefault scene)

-- | Normalize a user-facing name, then render with the ASCII backend.
renderAsciiForSymbology
  :: String
  -> String
  -> Barcode1DOptions
  -> Either Barcode1DPipelineError String
renderAsciiForSymbology symbology input options = do
  scene <- buildSceneForSymbology symbology input options
  first AsciiBackendFailure (PaintVmAscii.renderDefault scene)

-- | Build and render with caller-selected ASCII scaling.
renderAsciiWithOptions
  :: String
  -> Barcode1DOptions
  -> AsciiOptions
  -> Either Barcode1DPipelineError String
renderAsciiWithOptions input options asciiOptions = do
  scene <- buildScene input options
  first AsciiBackendFailure (PaintVmAscii.render scene asciiOptions)

trim :: String -> String
trim = dropWhileEnd isSpace . dropWhile isSpace
