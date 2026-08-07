module ItfSpec (spec) where

import CodingAdventures.Itf
import CodingAdventures.PaintInstructions (PaintInstruction (..))
import Data.Aeson (toJSON)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

  describe "normalizeItf" $ do
    it "accepts non-empty even-length ASCII digits unchanged" $ do
      normalizeItf "00" `shouldBe` Right "00"
      normalizeItf "123456" `shouldBe` Right "123456"

    it "rejects empty, non-ASCII, whitespace, and non-digit input" $ do
      let expected = Left (InvalidItfInput "ITF input must contain digits only")
      normalizeItf "" `shouldBe` expected
      normalizeItf "12A4" `shouldBe` expected
      normalizeItf "12 4" `shouldBe` expected
      normalizeItf "１２" `shouldBe` expected

    it "rejects odd-length digit input without padding" $
      normalizeItf "12345" `shouldBe`
        Left (InvalidItfInput "ITF input must contain an even number of digits")

  describe "encodeItf" $ do
    it "encodes the shared reference pair" $
      encodeItf "12" `shouldBe` Right
        [ EncodedPair
            { encodedPairDigits = "12"
            , encodedPairBarPattern = "10001"
            , encodedPairSpacePattern = "01001"
            , encodedPairBinaryPattern = "111010001010111000"
            , encodedPairSourceIndex = 0
            }
        ]

    it "preserves the complete standard digit table" $ do
      pairs <- expectRight (encodeItf "00112233445566778899")
      map encodedPairBarPattern pairs `shouldBe` digitTable
      map encodedPairSpacePattern pairs `shouldBe` digitTable

    it "assigns stable pair source indexes" $ do
      pairs <- expectRight (encodeItf "123456")
      map encodedPairDigits pairs `shouldBe` ["12", "34", "56"]
      map encodedPairSourceIndex pairs `shouldBe` [0, 1, 2]
      map (length . encodedPairBinaryPattern) pairs `shouldBe` [18, 18, 18]

    it "propagates validation failures" $
      encodeItf "123" `shouldBe`
        Left (InvalidItfInput "ITF input must contain an even number of digits")

  describe "expandItfRuns" $ do
    it "emits standard start and stop patterns with semantic roles" $ do
      runs <- expectRight (expandItfRuns "12")
      take 4 runs `shouldBe`
        [ Barcode1DRun Bar 1 "start" (-1) Start
        , Barcode1DRun Space 1 "start" (-1) Start
        , Barcode1DRun Bar 1 "start" (-1) Start
        , Barcode1DRun Space 1 "start" (-1) Start
        ]
      drop (length runs - 3) runs `shouldBe`
        [ Barcode1DRun Bar 3 "stop" (-2) Stop
        , Barcode1DRun Space 1 "stop" (-2) Stop
        , Barcode1DRun Bar 1 "stop" (-2) Stop
        ]

    it "interleaves first-digit bars with second-digit spaces" $ do
      runs <- expectRight (expandItfRuns "12")
      let dataRuns = filter ((== Data) . runRole) runs
      map runModules dataRuns `shouldBe` [3, 1, 1, 3, 1, 1, 1, 1, 3, 3]
      map runColor dataRuns `shouldBe` take 10 (cycle [Bar, Space])
      map runSourceLabel dataRuns `shouldBe` replicate 10 "12"

    it "produces the exact shared module and run counts" $ do
      runs <- expectRight (expandItfRuns "123456")
      sum (map runModules runs) `shouldBe` 63
      length runs `shouldBe` 37

    it "propagates input validation" $
      expandItfRuns "abc" `shouldBe`
        Left (InvalidItfInput "ITF input must contain digits only")

  describe "layoutItf" $ do
    it "renders through the shared default geometry" $ do
      scene <- expectRight (layoutItf "12" defaultPaintBarcode1DOptions)
      psWidth scene `shouldBe` 188
      psHeight scene `shouldBe` 120
      psBg scene `shouldBe` "#ffffff"
      length (psInstructions scene) `shouldBe` 9
      map prX (take 2 (psInstructions scene)) `shouldBe` [40, 48]

    it "records symbology, pair count, label, and explicit symbols" $ do
      scene <- expectRight (layoutItf "123456" defaultPaintBarcode1DOptions)
      Map.lookup "symbology" (psMeta scene) `shouldBe` Just (toJSON ("itf" :: String))
      Map.lookup "pairCount" (psMeta scene) `shouldBe` Just (toJSON (3 :: Int))
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("ITF barcode for 123456" :: String))
      Map.lookup "symbolCount" (psMeta scene) `shouldBe` Just (toJSON (5 :: Int))
      Map.lookup "contentModules" (psMeta scene) `shouldBe` Just (toJSON (63 :: Int))

    it "preserves custom rendering, labels, and metadata" $ do
      let config = defaultBarcode1DRenderConfig
            { configModuleWidth = 2
            , configBarHeight = 50
            , configQuietZoneModules = 5
            , configForeground = "navy"
            , configBackground = "transparent"
            }
          options = defaultPaintBarcode1DOptions
            { optionsRenderConfig = config
            , optionsLabel = Just "Warehouse pair"
            , optionsMetadata = Map.fromList
                [ ("batch", toJSON (7 :: Int))
                , ("pairCount", toJSON (999 :: Int))
                , ("symbology", toJSON ("wrong" :: String))
                ]
            }
      scene <- expectRight (layoutItf "12" options)
      (psWidth scene, psHeight scene, psBg scene) `shouldBe` (74, 50, "transparent")
      map prFill (psInstructions scene) `shouldBe` replicate 9 "navy"
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Warehouse pair" :: String))
      Map.lookup "batch" (psMeta scene) `shouldBe` Just (toJSON (7 :: Int))
      Map.lookup "pairCount" (psMeta scene) `shouldBe` Just (toJSON (1 :: Int))
      Map.lookup "symbology" (psMeta scene) `shouldBe` Just (toJSON ("itf" :: String))

    it "supports the draw alias" $
      drawItf "12" defaultPaintBarcode1DOptions `shouldBe`
        layoutItf "12" defaultPaintBarcode1DOptions

    it "wraps shared layout validation failures" $ do
      let invalidGeometry = defaultPaintBarcode1DOptions
            { optionsRenderConfig = defaultBarcode1DRenderConfig
                { configModuleWidth = 0 }
            }
          humanText = defaultPaintBarcode1DOptions
            { optionsHumanReadableText = Just "12" }
      layoutItf "12" invalidGeometry `shouldBe`
        Left (ItfLayoutError (InvalidRenderConfiguration "moduleWidth" 0))
      layoutItf "12" humanText `shouldBe`
        Left (ItfLayoutError HumanReadableTextUnsupported)

digitTable :: [String]
digitTable =
  [ "00110", "10001", "01001", "11000", "00101"
  , "10100", "01100", "00011", "10010", "01010"
  ]

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left err) = fail ("unexpected error: " ++ show err)
