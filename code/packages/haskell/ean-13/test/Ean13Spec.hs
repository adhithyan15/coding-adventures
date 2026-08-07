module Ean13Spec (spec) where

import CodingAdventures.Ean13
import CodingAdventures.PaintInstructions (PaintInstruction (..))
import Data.Aeson (toJSON)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

  describe "ean13DigitPattern" $ do
    it "contains all ten exact L patterns" $
      traverse (ean13DigitPattern Ean13L) ['0' .. '9'] `shouldBe`
        Right
          [ "0001101", "0011001", "0010011", "0111101", "0100011"
          , "0110001", "0101111", "0111011", "0110111", "0001011"
          ]

    it "contains all ten exact G patterns" $
      traverse (ean13DigitPattern Ean13G) ['0' .. '9'] `shouldBe`
        Right
          [ "0100111", "0110011", "0011011", "0100001", "0011101"
          , "0111001", "0000101", "0010001", "0001001", "0010111"
          ]

    it "contains all ten exact R patterns" $
      traverse (ean13DigitPattern Ean13R) ['0' .. '9'] `shouldBe`
        Right
          [ "1110010", "1100110", "1101100", "1000010", "1011100"
          , "1001110", "1010000", "1000100", "1001000", "1110100"
          ]

    it "rejects non-digit pattern lookups" $
      ean13DigitPattern Ean13L 'x' `shouldBe`
        Left (InvalidEan13Character 0 'x')

  describe "computeEan13CheckDigit" $ do
    it "matches shared reference values" $ do
      computeEan13CheckDigit "400638133393" `shouldBe` Right '1'
      computeEan13CheckDigit "590123412345" `shouldBe` Right '7'
      computeEan13CheckDigit "978020137962" `shouldBe` Right '4'
      computeEan13CheckDigit "000000000000" `shouldBe` Right '0'

    it "requires exactly twelve ASCII digits" $ do
      computeEan13CheckDigit "123" `shouldBe` Left (InvalidEan13Length 3)
      computeEan13CheckDigit "40063813339A" `shouldBe`
        Left (InvalidEan13Character 11 'A')

  describe "normalizeEan13" $ do
    it "appends a missing check digit" $
      normalizeEan13 "400638133393" `shouldBe` Right "4006381333931"

    it "preserves a valid supplied check digit" $
      normalizeEan13 "4006381333931" `shouldBe` Right "4006381333931"

    it "rejects the wrong supplied check digit" $
      normalizeEan13 "4006381333932" `shouldBe`
        Left (InvalidEan13CheckDigit '1' '2')

    it "rejects empty, short, long, non-ASCII, and mixed input" $ do
      normalizeEan13 "" `shouldBe` Left (InvalidEan13Length 0)
      normalizeEan13 "12345678901" `shouldBe` Left (InvalidEan13Length 11)
      normalizeEan13 "12345678901234" `shouldBe` Left (InvalidEan13Length 14)
      normalizeEan13 "40063813339A" `shouldBe`
        Left (InvalidEan13Character 11 'A')
      normalizeEan13 "４００６３８１３３３９３" `shouldBe`
        Left (InvalidEan13Character 0 '４')

    it "shows educational errors" $ do
      show (InvalidEan13Character 2 'x') `shouldBe`
        "Invalid EAN-13 character \"x\" at index 2; expected an ASCII digit"
      show (InvalidEan13Length 4) `shouldBe`
        "Invalid EAN-13 length 4; expected 12 payload digits or 13 complete digits"
      show (InvalidEan13CheckDigit '1' '2') `shouldBe`
        "Invalid EAN-13 check digit: expected 1 but received 2"

  describe "leftParityPattern" $ do
    it "contains every standard leading-digit parity sequence" $
      traverse parityFor ['0' .. '9'] `shouldBe`
        Right
          [ "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG"
          , "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL"
          ]

    it "uses the reference leading digit and accepts a supplied check" $ do
      leftParityPattern "400638133393" `shouldBe` Right "LGLLGG"
      leftParityPattern "4006381333931" `shouldBe` Right "LGLLGG"

  describe "encodeEan13" $ do
    it "encodes only the twelve visible digits with source attribution" $ do
      encoded <- expectRight (encodeEan13 "400638133393")
      map encodedEan13Digit encoded `shouldBe` "006381333931"
      map encodedEan13SourceIndex encoded `shouldBe` [1 .. 12]
      map encodedEan13Encoding encoded `shouldBe`
        [Ean13L, Ean13G, Ean13L, Ean13L, Ean13G, Ean13G]
          ++ replicate 6 Ean13R
      map encodedEan13Role encoded `shouldBe` replicate 11 Data ++ [Check]

    it "uses exact parity-selected patterns for the reference code" $ do
      encoded <- expectRight (encodeEan13 "400638133393")
      map encodedEan13Pattern encoded `shouldBe`
        [ "0001101", "0100111", "0101111", "0111101", "0001001", "0110011"
        , "1000010", "1000010", "1000010", "1110100", "1000010", "1100110"
        ]

    it "produces identical records for computed and supplied checks" $
      encodeEan13 "400638133393" `shouldBe` encodeEan13 "4006381333931"

  describe "expandEan13Runs" $ do
    it "builds the exact 95-module guard and visible-digit stream" $ do
      encoded <- expectRight (encodeEan13 "400638133393")
      runs <- expectRight (expandEan13Runs "400638133393")
      let expected = "101"
            ++ concatMap encodedEan13Pattern (take 6 encoded)
            ++ "01010"
            ++ concatMap encodedEan13Pattern (drop 6 encoded)
            ++ "101"
      modulesFromRuns runs `shouldBe` expected
      length expected `shouldBe` 95
      length runs `shouldBe` 59

    it "attributes guards, visible data, and the check digit" $ do
      runs <- expectRight (expandEan13Runs "400638133393")
      map runRole (take 3 runs) `shouldBe` replicate 3 Guard
      map runSourceLabel (take 3 runs) `shouldBe` replicate 3 "start"
      map runRole (take 5 (drop 27 runs)) `shouldBe` replicate 5 Guard
      map runSourceLabel (take 5 (drop 27 runs)) `shouldBe` replicate 5 "center"
      map runRole (take 4 (drop 52 runs)) `shouldBe` replicate 4 Check
      map runSourceLabel (take 4 (drop 52 runs)) `shouldBe` replicate 4 "1"
      map runRole (drop 56 runs) `shouldBe` replicate 3 Guard
      map runSourceLabel (drop 56 runs) `shouldBe` replicate 3 "end"
      0 `shouldNotSatisfy` (\sourceIndex -> sourceIndex `elem` map runSourceIndex runs)

    it "alternates colors across every symbol boundary" $ do
      runs <- expectRight (expandEan13Runs "400638133393")
      and (zipWith (/=) (map runColor runs) (drop 1 (map runColor runs)))
        `shouldBe` True

    it "propagates validation failures" $
      expandEan13Runs "4006381333932" `shouldBe`
        Left (InvalidEan13CheckDigit '1' '2')

  describe "layoutEan13" $ do
    it "renders the fixed module stream through shared default geometry" $ do
      scene <- expectRight (layoutEan13 "400638133393" defaultPaintBarcode1DOptions)
      psWidth scene `shouldBe` 460
      psHeight scene `shouldBe` 120
      psBg scene `shouldBe` "#ffffff"
      length (psInstructions scene) `shouldBe` 30
      map prX (take 2 (psInstructions scene)) `shouldBe` [40, 48]

    it "records normalized data, parity, checksum, label, and fifteen symbols" $ do
      scene <- expectRight (layoutEan13 "400638133393" defaultPaintBarcode1DOptions)
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("ean-13" :: String))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("4006381333931" :: String))
      Map.lookup "leadingDigit" (psMeta scene) `shouldBe`
        Just (toJSON ("4" :: String))
      Map.lookup "leftParity" (psMeta scene) `shouldBe`
        Just (toJSON ("LGLLGG" :: String))
      Map.lookup "checkDigit" (psMeta scene) `shouldBe`
        Just (toJSON ("1" :: String))
      Map.lookup "contentModules" (psMeta scene) `shouldBe` Just (toJSON (95 :: Int))
      Map.lookup "symbolCount" (psMeta scene) `shouldBe` Just (toJSON (15 :: Int))
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("EAN-13 barcode for 4006381333931" :: String))

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
                , ("leadingDigit", toJSON ("wrong" :: String))
                , ("leftParity", toJSON ("wrong" :: String))
                , ("symbology", toJSON ("wrong" :: String))
                ]
            }
      scene <- expectRight (layoutEan13 "400638133393" options)
      (psWidth scene, psHeight scene, psBg scene) `shouldBe` (210, 50, "transparent")
      map prFill (psInstructions scene) `shouldBe` replicate 30 "navy"
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Library label" :: String))
      Map.lookup "batch" (psMeta scene) `shouldBe` Just (toJSON (7 :: Int))
      Map.lookup "checkDigit" (psMeta scene) `shouldBe`
        Just (toJSON ("1" :: String))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("4006381333931" :: String))
      Map.lookup "leadingDigit" (psMeta scene) `shouldBe`
        Just (toJSON ("4" :: String))
      Map.lookup "leftParity" (psMeta scene) `shouldBe`
        Just (toJSON ("LGLLGG" :: String))
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("ean-13" :: String))

    it "supports the draw alias" $
      drawEan13 "400638133393" defaultPaintBarcode1DOptions `shouldBe`
        layoutEan13 "400638133393" defaultPaintBarcode1DOptions

    it "wraps shared layout validation failures" $ do
      let invalidGeometry = defaultPaintBarcode1DOptions
            { optionsRenderConfig = defaultBarcode1DRenderConfig
                { configModuleWidth = 0 }
            }
          humanText = defaultPaintBarcode1DOptions
            { optionsHumanReadableText = Just "4006381333931" }
      layoutEan13 "400638133393" invalidGeometry `shouldBe`
        Left (Ean13LayoutError (InvalidRenderConfiguration "moduleWidth" 0))
      layoutEan13 "400638133393" humanText `shouldBe`
        Left (Ean13LayoutError HumanReadableTextUnsupported)

parityFor :: Char -> Either Ean13Error String
parityFor leadingDigit = leftParityPattern ([leadingDigit] ++ replicate 11 '0')

modulesFromRuns :: [Barcode1DRun] -> String
modulesFromRuns = concatMap expandRun
  where
    expandRun run = replicate (runModules run)
      (if runColor run == Bar then '1' else '0')

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left err) = fail ("unexpected error: " ++ show err)
