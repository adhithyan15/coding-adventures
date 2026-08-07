module BarcodeLayout1DSpec (spec) where

import CodingAdventures.BarcodeLayout1D
import CodingAdventures.PaintInstructions (PaintInstruction (..), PaintScene (..))
import Data.Aeson (toJSON)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata and defaults" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

    it "uses the shared render defaults" $
      defaultBarcode1DRenderConfig `shouldBe` Barcode1DRenderConfig
        { configModuleWidth = 4
        , configBarHeight = 120
        , configQuietZoneModules = 10
        , configIncludeHumanReadableText = False
        , configTextFontSize = 16
        , configTextMargin = 8
        , configForeground = "#000000"
        , configBackground = "#ffffff"
        }

  describe "runsFromBinaryPattern" $ do
    it "coalesces adjacent modules and preserves attribution" $ do
      let options = defaultBinaryPatternOptions "start" (-1) Guard
      runsFromBinaryPattern "110100" options `shouldBe` Right
        [ Barcode1DRun Bar 2 "start" (-1) Guard
        , Barcode1DRun Space 1 "start" (-1) Guard
        , Barcode1DRun Bar 1 "start" (-1) Guard
        , Barcode1DRun Space 2 "start" (-1) Guard
        ]

    it "rejects empty and non-binary patterns" $ do
      let options = defaultBinaryPatternOptions "bad" 0 Data
      runsFromBinaryPattern "" options `shouldBe` Left (EmptyPattern "binary")
      runsFromBinaryPattern "10A1" options `shouldBe` Left (UnsupportedBinaryToken 'A')

  describe "runsFromWidthPattern" $ do
    it "expands narrow and wide markers with alternating colors" $ do
      let options = defaultWidthPatternOptions "A" 0 Data
      runsFromWidthPattern "NWNNW" options `shouldBe` Right
        [ Barcode1DRun Bar 1 "A" 0 Data
        , Barcode1DRun Space 3 "A" 0 Data
        , Barcode1DRun Bar 1 "A" 0 Data
        , Barcode1DRun Space 1 "A" 0 Data
        , Barcode1DRun Bar 3 "A" 0 Data
        ]

    it "supports custom markers, ratios, and starting color" $ do
      let options = (defaultWidthPatternOptions "B" 2 Check)
            { widthNarrowModules = 2
            , widthWideModules = 5
            , widthNarrowMarker = 'n'
            , widthWideMarker = 'w'
            , widthStartingColor = Space
            }
      runsFromWidthPattern "nww" options `shouldBe` Right
        [ Barcode1DRun Space 2 "B" 2 Check
        , Barcode1DRun Bar 5 "B" 2 Check
        , Barcode1DRun Space 5 "B" 2 Check
        ]

    it "rejects empty, invalid-marker, and non-positive-width patterns" $ do
      let options = defaultWidthPatternOptions "bad" 0 Data
      runsFromWidthPattern "" options `shouldBe` Left (EmptyPattern "width")
      runsFromWidthPattern "NX" options `shouldBe` Left (UnsupportedWidthToken 'X')
      runsFromWidthPattern "N" (options { widthNarrowModules = 0 })
        `shouldBe` Left (InvalidModuleCount "narrowModules" 0)
      runsFromWidthPattern "W" (options { widthWideModules = -1 })
        `shouldBe` Left (InvalidModuleCount "wideModules" (-1))

  describe "computeBarcode1DLayout" $ do
    it "adds widths and quiet zones" $ do
      let runs = sampleRuns
      totalModules runs `shouldBe` 4
      computeBarcode1DLayout runs 10 Nothing `shouldBe` Right (Barcode1DLayout
        { layoutLeftQuietZoneModules = 10
        , layoutRightQuietZoneModules = 10
        , layoutContentModules = 4
        , layoutTotalModules = 24
        , layoutSymbolLayouts =
            [ Barcode1DSymbolLayout "*" 0 2 0 SymbolStart
            , Barcode1DSymbolLayout "A" 2 4 1 SymbolData
            ]
        })

    it "groups adjacent runs from the same symbol" $ do
      let runs =
            [ Barcode1DRun Bar 1 "A" 0 Data
            , Barcode1DRun Space 2 "A" 0 Data
            , Barcode1DRun Bar 1 "B" 1 Check
            ]
      fmap layoutSymbolLayouts (computeBarcode1DLayout runs 5 Nothing) `shouldBe` Right
        [ Barcode1DSymbolLayout "A" 0 3 0 SymbolData
        , Barcode1DSymbolLayout "B" 3 4 1 SymbolCheck
        ]

    it "honors explicit symbol descriptors" $ do
      let descriptors =
            [ Barcode1DSymbolDescriptor "start" 2 (-1) SymbolGuard
            , Barcode1DSymbolDescriptor "value" 2 0 SymbolData
            ]
      fmap layoutSymbolLayouts (computeBarcode1DLayout sampleRuns 10 (Just descriptors))
        `shouldBe` Right
          [ Barcode1DSymbolLayout "start" 0 2 (-1) SymbolGuard
          , Barcode1DSymbolLayout "value" 2 4 0 SymbolData
          ]

    it "rejects invalid runs, quiet zones, and descriptors" $ do
      computeBarcode1DLayout [Barcode1DRun Bar 0 "A" 0 Data] 10 Nothing
        `shouldBe` Left (InvalidModuleCount "runs[0].modules" 0)
      computeBarcode1DLayout
        [Barcode1DRun Bar 1 "A" 0 Data, Barcode1DRun Bar 1 "B" 1 Data]
        10 Nothing `shouldBe` Left (NonAlternatingRuns 1)
      computeBarcode1DLayout [] 0 Nothing `shouldBe` Left (InvalidQuietZoneModules 0)
      computeBarcode1DLayout sampleRuns 10
        (Just [Barcode1DSymbolDescriptor "short" 3 0 SymbolData])
        `shouldBe` Left (SymbolWidthMismatch 4 3)
      computeBarcode1DLayout sampleRuns 10
        (Just [Barcode1DSymbolDescriptor "bad" 0 0 SymbolData])
        `shouldBe` Left (InvalidModuleCount "symbol \"bad\" modules" 0)

    it "allows an empty content stream with quiet zones" $
      computeBarcode1DLayout [] 3 Nothing `shouldBe` Right (Barcode1DLayout
        { layoutLeftQuietZoneModules = 3
        , layoutRightQuietZoneModules = 3
        , layoutContentModules = 0
        , layoutTotalModules = 6
        , layoutSymbolLayouts = []
        })

  describe "layoutBarcode1D" $ do
    it "emits rectangles only for bars at module-scaled coordinates" $ do
      scene <- expectRight (layoutBarcode1D sampleRuns defaultPaintBarcode1DOptions)
      psWidth scene `shouldBe` 96
      psHeight scene `shouldBe` 120
      psBg scene `shouldBe` "#ffffff"
      psInstructions scene `shouldSatisfy` ((== 2) . length)
      case psInstructions scene of
        [firstBar, secondBar] -> do
          (prX firstBar, prW firstBar) `shouldBe` (40, 4)
          (prX secondBar, prW secondBar) `shouldBe` (48, 8)
        _ -> expectationFailure "expected two bar rectangles"

    it "preserves run and scene metadata with standard keys authoritative" $ do
      let options = defaultPaintBarcode1DOptions
            { optionsLabel = Just "Demo barcode"
            , optionsMetadata = Map.fromList
                [ ("symbology", toJSON ("demo" :: String))
                , ("totalModules", toJSON (999 :: Int))
                ]
            }
      scene <- expectRight (layoutBarcode1D sampleRuns options)
      let firstBar = head (psInstructions scene)
      Map.lookup "sourceLabel" (prMeta firstBar) `shouldBe` Just (toJSON ("*" :: String))
      Map.lookup "moduleStart" (prMeta firstBar) `shouldBe` Just (toJSON (10 :: Int))
      Map.lookup "moduleEnd" (prMeta firstBar) `shouldBe` Just (toJSON (11 :: Int))
      Map.lookup "role" (prMeta firstBar) `shouldBe` Just (toJSON ("start" :: String))
      Map.lookup "label" (psMeta scene) `shouldBe` Just (toJSON ("Demo barcode" :: String))
      Map.lookup "symbology" (psMeta scene) `shouldBe` Just (toJSON ("demo" :: String))
      Map.lookup "totalModules" (psMeta scene) `shouldBe` Just (toJSON (24 :: Int))

    it "applies custom geometry and paint colors" $ do
      let config = defaultBarcode1DRenderConfig
            { configModuleWidth = 2.5
            , configBarHeight = 50
            , configQuietZoneModules = 2
            , configForeground = "navy"
            , configBackground = "transparent"
            }
      scene <- expectRight (layoutBarcode1D sampleRuns
        (defaultPaintBarcode1DOptions { optionsRenderConfig = config }))
      (psWidth scene, psHeight scene, psBg scene) `shouldBe` (20, 50, "transparent")
      map prFill (psInstructions scene) `shouldBe` ["navy", "navy"]

    it "supports the draw alias" $
      drawBarcode1D sampleRuns defaultPaintBarcode1DOptions
        `shouldBe` layoutBarcode1D sampleRuns defaultPaintBarcode1DOptions

    it "rejects all non-finite and out-of-range render values" $ do
      invalidConfig (defaultBarcode1DRenderConfig { configModuleWidth = 0 })
        `shouldBe` Left (InvalidRenderConfiguration "moduleWidth" 0)
      invalidConfig (defaultBarcode1DRenderConfig { configBarHeight = -1 })
        `shouldBe` Left (InvalidRenderConfiguration "barHeight" (-1))
      invalidConfig (defaultBarcode1DRenderConfig { configQuietZoneModules = 0 })
        `shouldBe` Left (InvalidQuietZoneModules 0)
      invalidConfig (defaultBarcode1DRenderConfig { configTextFontSize = 1 / 0 })
        `shouldBe` Left (InvalidRenderConfiguration "textFontSize" (1 / 0))
      invalidConfig (defaultBarcode1DRenderConfig { configTextMargin = -1 })
        `shouldBe` Left (InvalidRenderConfiguration "textMargin" (-1))
      invalidConfig (defaultBarcode1DRenderConfig { configTextMargin = 0 / 0 })
        `shouldSatisfy` isInvalidTextMargin

    it "rejects both text configuration paths until shaping exists" $ do
      invalidConfig
        (defaultBarcode1DRenderConfig { configIncludeHumanReadableText = True })
        `shouldBe` Left HumanReadableTextUnsupported
      layoutBarcode1D sampleRuns
        (defaultPaintBarcode1DOptions { optionsHumanReadableText = Just "123" })
        `shouldBe` Left HumanReadableTextUnsupported

sampleRuns :: [Barcode1DRun]
sampleRuns =
  [ Barcode1DRun Bar 1 "*" 0 Start
  , Barcode1DRun Space 1 "*" 0 InterCharacterGap
  , Barcode1DRun Bar 2 "A" 1 Data
  ]

invalidConfig :: Barcode1DRenderConfig -> Either Barcode1DError PaintScene
invalidConfig config = layoutBarcode1D sampleRuns
  (defaultPaintBarcode1DOptions { optionsRenderConfig = config })

isInvalidTextMargin :: Either Barcode1DError PaintScene -> Bool
isInvalidTextMargin (Left (InvalidRenderConfiguration "textMargin" value)) = isNaN value
isInvalidTextMargin _ = False

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left err) = fail ("unexpected error: " ++ show err)
