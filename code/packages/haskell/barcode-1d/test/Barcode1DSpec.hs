module Barcode1DSpec (spec) where

import CodingAdventures.Barcode1D
import CodingAdventures.PaintInstructions (PaintInstruction (..))
import Data.Aeson (toJSON)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "package metadata and defaults" $ do
    it "reports the package version and pure backend" $ do
      version `shouldBe` "0.1.0"
      currentBackend `shouldBe` "ascii"

    it "defaults to Code 39 and shared paint settings" $ do
      barcodeSymbology defaultBarcode1DOptions `shouldBe` Code39
      barcodePaintOptions defaultBarcode1DOptions
        `shouldBe` defaultPaintBarcode1DOptions
      barcodeCodabarGuards defaultBarcode1DOptions
        `shouldBe` defaultCodabarGuards
      defaultRenderConfig `shouldBe` defaultBarcode1DRenderConfig

  describe "normalizeSymbology" $ do
    it "accepts every supported spelling" $ do
      normalizeSymbology " codabar " `shouldBe` Right Codabar
      normalizeSymbology "CODE_128" `shouldBe` Right Code128
      normalizeSymbology "code-39" `shouldBe` Right Code39
      normalizeSymbology "EAN-13" `shouldBe` Right Ean13
      normalizeSymbology "itf" `shouldBe` Right Itf
      normalizeSymbology "UPC_A" `shouldBe` Right UpcA

    it "uses Code 39 for an empty normalized name" $ do
      normalizeSymbology "" `shouldBe` Right Code39
      normalizeSymbology "  " `shouldBe` Right Code39

    it "rejects unsupported names without losing the input" $
      normalizeSymbology "qr-code" `shouldBe`
        Left (UnsupportedSymbology "qr-code")

  describe "buildScene" $ do
    it "routes all six native encoders" $ do
      assertRoute Code39 "A" "code39"
      assertRoute Codabar "123" "codabar"
      assertRoute Code128 "AB" "code128"
      assertRoute Ean13 "400638133393" "ean-13"
      assertRoute Itf "1234" "itf"
      assertRoute UpcA "03600029145" "upc-a"

    it "supports normalized string routing" $ do
      scene <- expectRight
        (buildSceneForSymbology "EAN_13" "400638133393"
          defaultBarcode1DOptions)
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("ean-13" :: String))

    it "forwards custom Codabar guards" $ do
      let options = defaultBarcode1DOptions
            { barcodeSymbology = Codabar
            , barcodeCodabarGuards = CodabarGuards 'B' 'D'
            }
      scene <- expectRight (buildScene "123" options)
      Map.lookup "start" (psMeta scene) `shouldBe`
        Just (toJSON ("B" :: String))
      Map.lookup "stop" (psMeta scene) `shouldBe`
        Just (toJSON ("D" :: String))

    it "forwards shared geometry and paint options" $ do
      let renderConfig = defaultRenderConfig
            { configBarHeight = 48
            , configForeground = "navy"
            , configBackground = "transparent"
            }
          paintOptions = defaultPaintBarcode1DOptions
            { optionsRenderConfig = renderConfig }
          options = defaultBarcode1DOptions
            { barcodePaintOptions = paintOptions }
      scene <- expectRight (buildScene "A" options)
      psHeight scene `shouldBe` 48
      psBg scene `shouldBe` "transparent"
      map prFill (psInstructions scene) `shouldSatisfy`
        all (== "navy")

    it "preserves the originating encoder error" $
      buildScene "@" defaultBarcode1DOptions `shouldBe`
        Left (Code39Failure (InvalidCode39Character '@'))

  describe "ASCII rendering" $ do
    it "renders the default pipeline" $ do
      output <- expectRight (renderAscii "A" defaultBarcode1DOptions)
      output `shouldSatisfy` elem '\x2588'

    it "renders through normalized string routing" $ do
      output <- expectRight
        (renderAsciiForSymbology "itf" "1234" defaultBarcode1DOptions)
      output `shouldSatisfy` elem '\x2588'

    it "wraps ASCII backend failures" $ do
      let invalidOptions = defaultAsciiOptions { scaleX = 0 }
      renderAsciiWithOptions "A" defaultBarcode1DOptions invalidOptions
        `shouldBe` Left (AsciiBackendFailure (InvalidScaleX 0))

assertRoute :: Symbology -> String -> String -> Expectation
assertRoute symbology input expected = do
  let options = defaultBarcode1DOptions { barcodeSymbology = symbology }
  scene <- expectRight (buildScene input options)
  Map.lookup "symbology" (psMeta scene) `shouldBe`
    Just (toJSON expected)

expectRight :: (Show error) => Either error value -> IO value
expectRight result = case result of
  Right value -> pure value
  Left err -> expectationFailure ("expected Right, got Left " ++ show err)
    >> fail "unreachable"
