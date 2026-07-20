module Code128Spec (spec) where

import CodingAdventures.Code128
import CodingAdventures.PaintInstructions (PaintInstruction (..))
import Data.Aeson (toJSON)
import Data.List (nub)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata and defaults" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

    it "uses the shared render defaults" $ do
      configModuleWidth defaultBarcode1DRenderConfig `shouldBe` 4
      configQuietZoneModules defaultBarcode1DRenderConfig `shouldBe` 10

  describe "normalizeCode128B" $ do
    it "accepts printable ASCII boundaries and empty input" $ do
      normalizeCode128B "" `shouldBe` Right ""
      normalizeCode128B " ~" `shouldBe` Right " ~"

    it "rejects control, delete, and non-ASCII characters educationally" $ do
      normalizeCode128B "bad\ninput" `shouldBe`
        Left (InvalidCode128Character '\n')
      normalizeCode128B ['\DEL'] `shouldBe`
        Left (InvalidCode128Character '\DEL')
      normalizeCode128B "caf\233" `shouldBe`
        Left (InvalidCode128Character '\233')
      show (InvalidCode128Character '\n') `shouldBe`
        "Invalid Code 128 Code Set B character \"\\n\"; expected printable ASCII (32-126)"

  describe "Code Set B values and patterns" $ do
    it "maps printable ASCII to values 0 through 94" $ do
      code128BValue ' ' `shouldBe` Right 0
      code128BValue 'A' `shouldBe` Right 33
      code128BValue '~' `shouldBe` Right 94
      code128BValue '\n' `shouldBe` Left (InvalidCode128Character '\n')

    it "contains all 107 distinct standard patterns" $ do
      patterns <- expectRight (traverse code128Pattern [0 .. 106])
      length patterns `shouldBe` 107
      length (nub patterns) `shouldBe` 107
      map length (take 106 patterns) `shouldBe` replicate 106 11
      length (last patterns) `shouldBe` 13

    it "uses exact reference patterns for data, Start B, and stop" $ do
      code128Pattern 0 `shouldBe` Right "11011001100"
      code128Pattern 64 `shouldBe` Right "10100001100"
      code128Pattern 104 `shouldBe` Right "11010010000"
      code128Pattern 106 `shouldBe` Right "1100011101011"

    it "rejects values outside the complete table" $ do
      code128Pattern (-1) `shouldBe` Left (InvalidCode128Value (-1))
      code128Pattern 107 `shouldBe` Left (InvalidCode128Value 107)
      show (InvalidCode128Value 107) `shouldBe`
        "Invalid Code 128 symbol value 107; expected 0-106"

  describe "computeCode128Checksum" $ do
    it "matches the classic Code 128 example" $
      computeCode128Checksum [35, 79, 68, 69, 0, 17, 18, 24] `shouldBe` 64

    it "includes Start B for an empty payload" $
      computeCode128Checksum [] `shouldBe` 1

  describe "encodeCode128B" $ do
    it "adds Start B, data, checksum, and stop with source attribution" $ do
      encoded <- expectRight (encodeCode128B "Code 128")
      map encodedCode128Value encoded `shouldBe`
        [104, 35, 79, 68, 69, 0, 17, 18, 24, 64, 106]
      map encodedCode128SourceIndex encoded `shouldBe` [-1 .. 9]
      map encodedCode128Role encoded `shouldBe`
        [Start, Data, Data, Data, Data, Data, Data, Data, Data, Check, Stop]
      encodedCode128Label (head encoded) `shouldBe` "Start B"
      encodedCode128Label (encoded !! 9) `shouldBe` "Checksum 64"
      encodedCode128Label (last encoded) `shouldBe` "Stop"

    it "encodes empty input as start, checksum, and stop" $ do
      encoded <- expectRight (encodeCode128B "")
      map encodedCode128Value encoded `shouldBe` [104, 1, 106]
      map encodedCode128SourceIndex encoded `shouldBe` [-1, 0, 1]
      map encodedCode128Role encoded `shouldBe` [Start, Check, Stop]

    it "propagates input validation failures" $
      encodeCode128B "A\n" `shouldBe` Left (InvalidCode128Character '\n')

  describe "expandCode128Runs" $ do
    it "expands six runs per regular symbol and seven for stop" $ do
      runs <- expectRight (expandCode128Runs "Hi")
      length runs `shouldBe` 31
      sum (map runModules runs) `shouldBe` 57
      map runRole (take 6 runs) `shouldBe` replicate 6 Start
      map runRole (take 6 (drop 6 runs)) `shouldBe` replicate 6 Data
      map runRole (take 6 (drop 12 runs)) `shouldBe` replicate 6 Data
      map runRole (take 6 (drop 18 runs)) `shouldBe` replicate 6 Check
      map runRole (drop 24 runs) `shouldBe` replicate 7 Stop

    it "preserves exact labels, indices, and global color alternation" $ do
      runs <- expectRight (expandCode128Runs "Hi")
      map runSourceLabel (take 6 runs) `shouldBe` replicate 6 "Start B"
      map runSourceIndex (take 6 (drop 6 runs)) `shouldBe` replicate 6 0
      map runSourceLabel (drop 24 runs) `shouldBe` replicate 7 "Stop"
      map runColor runs `shouldBe` take (length runs) (cycle [Bar, Space])

    it "propagates validation failures" $
      expandCode128Runs "\233" `shouldBe` Left (InvalidCode128Character '\233')

  describe "layoutCode128" $ do
    it "renders exact default geometry through the shared layer" $ do
      scene <- expectRight (layoutCode128 "A" defaultPaintBarcode1DOptions)
      psWidth scene `shouldBe` 264
      psHeight scene `shouldBe` 120
      psBg scene `shouldBe` "#ffffff"
      length (psInstructions scene) `shouldBe` 13
      map prX (take 2 (psInstructions scene)) `shouldBe` [40, 52]

    it "records encoded data, code set, checksum, label, and symbols" $ do
      scene <- expectRight
        (layoutCode128 "Code 128" defaultPaintBarcode1DOptions)
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("code128" :: String))
      Map.lookup "codeSet" (psMeta scene) `shouldBe`
        Just (toJSON ("B" :: String))
      Map.lookup "checksum" (psMeta scene) `shouldBe` Just (toJSON (64 :: Int))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("Code 128" :: String))
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Code 128 barcode for Code 128" :: String))
      Map.lookup "symbolCount" (psMeta scene) `shouldBe` Just (toJSON (11 :: Int))

    it "preserves custom rendering and caller metadata with standard keys authoritative" $ do
      let config = defaultBarcode1DRenderConfig
            { configModuleWidth = 2
            , configBarHeight = 50
            , configQuietZoneModules = 5
            , configForeground = "navy"
            , configBackground = "transparent"
            }
          options = defaultPaintBarcode1DOptions
            { optionsRenderConfig = config
            , optionsLabel = Just "Library label"
            , optionsMetadata = Map.fromList
                [ ("batch", toJSON (7 :: Int))
                , ("encodedText", toJSON ("wrong" :: String))
                , ("codeSet", toJSON ("wrong" :: String))
                , ("checksum", toJSON ((-1) :: Int))
                , ("symbology", toJSON ("wrong" :: String))
                ]
            }
      scene <- expectRight (layoutCode128 "A" options)
      (psWidth scene, psHeight scene, psBg scene) `shouldBe`
        (112, 50, "transparent")
      map prFill (psInstructions scene) `shouldBe` replicate 13 "navy"
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Library label" :: String))
      Map.lookup "batch" (psMeta scene) `shouldBe` Just (toJSON (7 :: Int))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("A" :: String))
      Map.lookup "codeSet" (psMeta scene) `shouldBe`
        Just (toJSON ("B" :: String))
      Map.lookup "checksum" (psMeta scene) `shouldBe` Just (toJSON (34 :: Int))
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("code128" :: String))

    it "supports the draw alias" $
      drawCode128 "A" defaultPaintBarcode1DOptions `shouldBe`
        layoutCode128 "A" defaultPaintBarcode1DOptions

    it "wraps shared layout validation failures" $ do
      let invalidGeometry = defaultPaintBarcode1DOptions
            { optionsRenderConfig = defaultBarcode1DRenderConfig
                { configModuleWidth = 0 }
            }
          humanText = defaultPaintBarcode1DOptions
            { optionsHumanReadableText = Just "A" }
      layoutCode128 "A" invalidGeometry `shouldBe`
        Left (Code128LayoutError (InvalidRenderConfiguration "moduleWidth" 0))
      layoutCode128 "A" humanText `shouldBe`
        Left (Code128LayoutError HumanReadableTextUnsupported)

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left err) = fail ("unexpected error: " ++ show err)
