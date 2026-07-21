module UpcASpec (spec) where

import CodingAdventures.PaintInstructions (PaintInstruction (..))
import CodingAdventures.UpcA
import Data.Aeson (toJSON)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

  describe "upcADigitPattern" $ do
    it "contains all ten exact left patterns" $
      traverse (upcADigitPattern LeftEncoding) ['0' .. '9'] `shouldBe`
        Right
          [ "0001101", "0011001", "0010011", "0111101", "0100011"
          , "0110001", "0101111", "0111011", "0110111", "0001011"
          ]

    it "contains all ten exact right patterns" $
      traverse (upcADigitPattern RightEncoding) ['0' .. '9'] `shouldBe`
        Right
          [ "1110010", "1100110", "1101100", "1000010", "1011100"
          , "1001110", "1010000", "1000100", "1001000", "1110100"
          ]

    it "rejects non-digit pattern lookups" $
      upcADigitPattern LeftEncoding 'x' `shouldBe`
        Left (InvalidUpcACharacter 0 'x')

  describe "computeUpcACheckDigit" $ do
    it "matches shared reference values" $ do
      computeUpcACheckDigit "03600029145" `shouldBe` Right '2'
      computeUpcACheckDigit "04210000526" `shouldBe` Right '4'
      computeUpcACheckDigit "12345678901" `shouldBe` Right '2'
      computeUpcACheckDigit "00000000000" `shouldBe` Right '0'

    it "requires exactly eleven ASCII digits" $ do
      computeUpcACheckDigit "123" `shouldBe` Left (InvalidUpcALength 3)
      computeUpcACheckDigit "03600A29145" `shouldBe`
        Left (InvalidUpcACharacter 5 'A')

  describe "normalizeUpcA" $ do
    it "appends a missing check digit" $
      normalizeUpcA "03600029145" `shouldBe` Right "036000291452"

    it "preserves a valid supplied check digit" $
      normalizeUpcA "036000291452" `shouldBe` Right "036000291452"

    it "rejects the wrong supplied check digit" $
      normalizeUpcA "036000291453" `shouldBe`
        Left (InvalidUpcACheckDigit '2' '3')

    it "rejects empty, short, long, non-ASCII, and mixed input" $ do
      normalizeUpcA "" `shouldBe` Left (InvalidUpcALength 0)
      normalizeUpcA "1234567890" `shouldBe` Left (InvalidUpcALength 10)
      normalizeUpcA "1234567890123" `shouldBe` Left (InvalidUpcALength 13)
      normalizeUpcA "1234567890A" `shouldBe`
        Left (InvalidUpcACharacter 10 'A')
      normalizeUpcA "１２３４５６７８９０１" `shouldBe`
        Left (InvalidUpcACharacter 0 '１')

    it "shows educational errors" $ do
      show (InvalidUpcACharacter 2 'x') `shouldBe`
        "Invalid UPC-A character \"x\" at index 2; expected an ASCII digit"
      show (InvalidUpcALength 4) `shouldBe`
        "Invalid UPC-A length 4; expected 11 payload digits or 12 complete digits"
      show (InvalidUpcACheckDigit '2' '3') `shouldBe`
        "Invalid UPC-A check digit: expected 2 but received 3"

  describe "encodeUpcA" $ do
    it "encodes six left and six right digits with source attribution" $ do
      encoded <- expectRight (encodeUpcA "03600029145")
      map encodedUpcADigit encoded `shouldBe` "036000291452"
      map encodedUpcASourceIndex encoded `shouldBe` [0 .. 11]
      map encodedUpcAEncoding encoded `shouldBe`
        replicate 6 LeftEncoding ++ replicate 6 RightEncoding
      map encodedUpcARole encoded `shouldBe` replicate 11 Data ++ [Check]

    it "uses the exact patterns for a reference code" $ do
      encoded <- expectRight (encodeUpcA "03600029145")
      map encodedUpcAPattern encoded `shouldBe`
        [ "0001101", "0111101", "0101111", "0001101", "0001101", "0001101"
        , "1101100", "1110100", "1100110", "1011100", "1001110", "1101100"
        ]

    it "produces identical records for computed and supplied checks" $
      encodeUpcA "03600029145" `shouldBe` encodeUpcA "036000291452"

  describe "expandUpcARuns" $ do
    it "builds the exact 95-module guard and digit stream" $ do
      encoded <- expectRight (encodeUpcA "03600029145")
      runs <- expectRight (expandUpcARuns "03600029145")
      let expected = "101"
            ++ concatMap encodedUpcAPattern (take 6 encoded)
            ++ "01010"
            ++ concatMap encodedUpcAPattern (drop 6 encoded)
            ++ "101"
      modulesFromRuns runs `shouldBe` expected
      length expected `shouldBe` 95
      length runs `shouldBe` 59

    it "attributes guards, data, and the check digit" $ do
      runs <- expectRight (expandUpcARuns "03600029145")
      map runRole (take 3 runs) `shouldBe` replicate 3 Guard
      map runSourceLabel (take 3 runs) `shouldBe` replicate 3 "start"
      map runRole (take 5 (drop 27 runs)) `shouldBe` replicate 5 Guard
      map runSourceLabel (take 5 (drop 27 runs)) `shouldBe` replicate 5 "center"
      map runRole (take 4 (drop 52 runs)) `shouldBe` replicate 4 Check
      map runSourceLabel (take 4 (drop 52 runs)) `shouldBe` replicate 4 "2"
      map runRole (drop 56 runs) `shouldBe` replicate 3 Guard
      map runSourceLabel (drop 56 runs) `shouldBe` replicate 3 "end"

    it "alternates colors across every symbol boundary" $ do
      runs <- expectRight (expandUpcARuns "03600029145")
      and (zipWith (/=) (map runColor runs) (drop 1 (map runColor runs)))
        `shouldBe` True

    it "propagates validation failures" $
      expandUpcARuns "036000291453" `shouldBe`
        Left (InvalidUpcACheckDigit '2' '3')

  describe "layoutUpcA" $ do
    it "renders the fixed module stream through shared default geometry" $ do
      scene <- expectRight (layoutUpcA "03600029145" defaultPaintBarcode1DOptions)
      psWidth scene `shouldBe` 460
      psHeight scene `shouldBe` 120
      psBg scene `shouldBe` "#ffffff"
      length (psInstructions scene) `shouldBe` 30
      map prX (take 2 (psInstructions scene)) `shouldBe` [40, 48]

    it "records normalized data, checksum, label, and all fifteen symbols" $ do
      scene <- expectRight (layoutUpcA "03600029145" defaultPaintBarcode1DOptions)
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("upc-a" :: String))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("036000291452" :: String))
      Map.lookup "checkDigit" (psMeta scene) `shouldBe`
        Just (toJSON ("2" :: String))
      Map.lookup "contentModules" (psMeta scene) `shouldBe` Just (toJSON (95 :: Int))
      Map.lookup "symbolCount" (psMeta scene) `shouldBe` Just (toJSON (15 :: Int))
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("UPC-A barcode for 036000291452" :: String))

    it "preserves custom rendering, labels, and caller metadata" $ do
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
                , ("checkDigit", toJSON ("wrong" :: String))
                , ("encodedText", toJSON ("wrong" :: String))
                , ("symbology", toJSON ("wrong" :: String))
                ]
            }
      scene <- expectRight (layoutUpcA "03600029145" options)
      (psWidth scene, psHeight scene, psBg scene) `shouldBe` (210, 50, "transparent")
      map prFill (psInstructions scene) `shouldBe` replicate 30 "navy"
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Library label" :: String))
      Map.lookup "batch" (psMeta scene) `shouldBe` Just (toJSON (7 :: Int))
      Map.lookup "checkDigit" (psMeta scene) `shouldBe`
        Just (toJSON ("2" :: String))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("036000291452" :: String))
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("upc-a" :: String))

    it "supports the draw alias" $
      drawUpcA "03600029145" defaultPaintBarcode1DOptions `shouldBe`
        layoutUpcA "03600029145" defaultPaintBarcode1DOptions

    it "wraps shared layout validation failures" $ do
      let invalidGeometry = defaultPaintBarcode1DOptions
            { optionsRenderConfig = defaultBarcode1DRenderConfig
                { configModuleWidth = 0 }
            }
          humanText = defaultPaintBarcode1DOptions
            { optionsHumanReadableText = Just "036000291452" }
      layoutUpcA "03600029145" invalidGeometry `shouldBe`
        Left (UpcALayoutError (InvalidRenderConfiguration "moduleWidth" 0))
      layoutUpcA "03600029145" humanText `shouldBe`
        Left (UpcALayoutError HumanReadableTextUnsupported)

modulesFromRuns :: [Barcode1DRun] -> String
modulesFromRuns = concatMap expandRun
  where
    expandRun run = replicate (runModules run)
      (if runColor run == Bar then '1' else '0')

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left err) = fail ("unexpected error: " ++ show err)
