module Code39Spec (spec) where

import CodingAdventures.Code39
import CodingAdventures.PaintInstructions (PaintInstruction (..))
import Data.Aeson (toJSON)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

  describe "normalizeCode39" $ do
    it "uppercases letters while preserving digits, punctuation, and spaces" $
      normalizeCode39 "hello-123 $/+%" `shouldBe` Right "HELLO-123 $/+%"

    it "accepts empty input for a start/stop-only barcode" $
      normalizeCode39 "" `shouldBe` Right ""

    it "rejects the reserved start/stop marker" $ do
      normalizeCode39 "A*B" `shouldBe` Left ReservedCode39StartStop
      show ReservedCode39StartStop `shouldBe`
        "Input must not contain \"*\" because it is reserved for start/stop"

    it "rejects unsupported ASCII and Unicode characters educationally" $ do
      normalizeCode39 "ABC@123" `shouldBe` Left (InvalidCode39Character '@')
      normalizeCode39 "CAFÉ" `shouldBe` Left (InvalidCode39Character 'É')
      show (InvalidCode39Character '@') `shouldBe`
        "Invalid character: \"@\" is not supported by Code 39"

  describe "encodeCode39Char" $ do
    it "encodes the shared A reference pattern" $
      encodeCode39Char 'A' `shouldBe` Right
        (EncodedCharacter 'A' False "WNNNNWNNW")

    it "recognizes the inserted start and stop symbol" $
      encodeCode39Char '*' `shouldBe` Right
        (EncodedCharacter '*' True "NWNNWNWNN")

    it "rejects characters that have not been normalized" $
      encodeCode39Char 'a' `shouldBe` Left (InvalidCode39Character 'a')

    it "contains all 44 standard patterns with exactly three wide elements" $ do
      encoded <- traverse (expectRight . encodeCode39Char) standardAlphabet
      length encoded `shouldBe` 44
      map (length . encodedCharacterPattern) encoded `shouldBe` replicate 44 9
      map (length . filter (== 'W') . encodedCharacterPattern) encoded
        `shouldBe` replicate 44 3

  describe "encodeCode39" $ do
    it "wraps normalized data in start and stop markers" $ do
      encoded <- expectRight (encodeCode39 "a-1")
      map encodedCharacterValue encoded `shouldBe` "*A-1*"
      map encodedCharacterIsStartStop encoded `shouldBe`
        [True, False, False, False, True]

    it "encodes empty input as only the two delimiters" $
      fmap (map encodedCharacterValue) (encodeCode39 "") `shouldBe` Right "**"

  describe "expandCode39Runs" $ do
    it "expands one data character into three symbols and two gaps" $ do
      runs <- expectRight (expandCode39Runs "A")
      length runs `shouldBe` 29
      map runRole (take 9 runs) `shouldBe` replicate 9 Start
      runs !! 9 `shouldBe` Barcode1DRun Space 1 "*" 0 InterCharacterGap
      map runRole (take 9 (drop 10 runs)) `shouldBe` replicate 9 Data
      runs !! 19 `shouldBe` Barcode1DRun Space 1 "A" 1 InterCharacterGap
      map runRole (drop 20 runs) `shouldBe` replicate 9 Stop

    it "alternates five bars and four spaces inside every symbol" $ do
      runs <- expectRight (expandCode39Runs "A")
      let symbolRuns = take 9 runs
      map runColor symbolRuns `shouldBe` take 9 (cycle [Bar, Space])
      length (filter ((== Bar) . runColor) symbolRuns) `shouldBe` 5
      map runModules (take 9 (drop 10 runs)) `shouldBe`
        [3, 1, 1, 1, 1, 3, 1, 1, 3]

    it "uses exact shared module counts" $ do
      one <- expectRight (expandCode39Runs "A")
      empty <- expectRight (expandCode39Runs "")
      sum (map runModules one) `shouldBe` 47
      sum (map runModules empty) `shouldBe` 31
      length empty `shouldBe` 19

    it "propagates validation failures" $
      expandCode39Runs "A@" `shouldBe` Left (InvalidCode39Character '@')

  describe "layoutCode39" $ do
    it "renders through the shared default geometry" $ do
      scene <- expectRight (layoutCode39 "A" defaultPaintBarcode1DOptions)
      psWidth scene `shouldBe` 268
      psHeight scene `shouldBe` 120
      psBg scene `shouldBe` "#ffffff"
      length (psInstructions scene) `shouldBe` 15
      map prX (take 2 (psInstructions scene)) `shouldBe` [40, 56]

    it "records normalized data, label, and inferred symbols" $ do
      scene <- expectRight (layoutCode39 "ab" defaultPaintBarcode1DOptions)
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("code39" :: String))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("AB" :: String))
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Code 39 barcode for AB" :: String))
      Map.lookup "symbolCount" (psMeta scene) `shouldBe` Just (toJSON (4 :: Int))

    it "uses a concise label for empty input" $ do
      scene <- expectRight (layoutCode39 "" defaultPaintBarcode1DOptions)
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Code 39 barcode" :: String))

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
            , optionsLabel = Just "Inventory label"
            , optionsMetadata = Map.fromList
                [ ("batch", toJSON (7 :: Int))
                , ("encodedText", toJSON ("wrong" :: String))
                , ("symbology", toJSON ("wrong" :: String))
                ]
            }
      scene <- expectRight (layoutCode39 "a" options)
      (psWidth scene, psHeight scene, psBg scene) `shouldBe` (114, 50, "transparent")
      map prFill (psInstructions scene) `shouldBe` replicate 15 "navy"
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Inventory label" :: String))
      Map.lookup "batch" (psMeta scene) `shouldBe` Just (toJSON (7 :: Int))
      Map.lookup "encodedText" (psMeta scene) `shouldBe` Just (toJSON ("A" :: String))
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("code39" :: String))

    it "supports the draw alias" $
      drawCode39 "A" defaultPaintBarcode1DOptions `shouldBe`
        layoutCode39 "A" defaultPaintBarcode1DOptions

    it "wraps shared layout validation failures" $ do
      let invalidGeometry = defaultPaintBarcode1DOptions
            { optionsRenderConfig = defaultBarcode1DRenderConfig
                { configModuleWidth = 0 }
            }
          humanText = defaultPaintBarcode1DOptions
            { optionsHumanReadableText = Just "A" }
      layoutCode39 "A" invalidGeometry `shouldBe`
        Left (Code39LayoutError (InvalidRenderConfiguration "moduleWidth" 0))
      layoutCode39 "A" humanText `shouldBe`
        Left (Code39LayoutError HumanReadableTextUnsupported)

standardAlphabet :: String
standardAlphabet = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%*"

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left err) = fail ("unexpected error: " ++ show err)
