module CodabarSpec (spec) where

import CodingAdventures.Codabar
import CodingAdventures.PaintInstructions (PaintInstruction (..))
import Data.Aeson (toJSON)
import Data.List (nub, sort)
import qualified Data.Map.Strict as Map
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata and defaults" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

    it "uses A guards by default" $
      defaultCodabarGuards `shouldBe` CodabarGuards 'A' 'A'

  describe "normalizeCodabar" $ do
    it "wraps body data in A guards by default" $
      normalizeCodabar "40156" defaultCodabarGuards `shouldBe` Right "A40156A"

    it "uppercases and preserves explicit guards" $
      normalizeCodabar "b40156d" defaultCodabarGuards `shouldBe` Right "B40156D"

    it "inserts distinct configured guards" $
      normalizeCodabar "40156" (CodabarGuards 'B' 'D') `shouldBe` Right "B40156D"

    it "accepts an empty body" $
      normalizeCodabar "" defaultCodabarGuards `shouldBe` Right "AA"

    it "rejects invalid configured guards educationally" $ do
      normalizeCodabar "0" (CodabarGuards 'X' 'A') `shouldBe`
        Left (InvalidCodabarGuard 'X')
      normalizeCodabar "0" (CodabarGuards 'A' 'X') `shouldBe`
        Left (InvalidCodabarGuard 'X')
      show (InvalidCodabarGuard 'X') `shouldBe`
        "Invalid Codabar guard \"X\"; expected A, B, C, or D"

    it "rejects unsupported and embedded guard characters" $ do
      normalizeCodabar "40*56" defaultCodabarGuards `shouldBe`
        Left (InvalidCodabarBodyCharacter '*')
      normalizeCodabar "40A56" defaultCodabarGuards `shouldBe`
        Left (InvalidCodabarBodyCharacter 'A')
      normalizeCodabar "A40156" defaultCodabarGuards `shouldBe`
        Left (InvalidCodabarBodyCharacter 'A')
      show (InvalidCodabarBodyCharacter '*') `shouldBe`
        "Invalid Codabar body character \"*\""

  describe "encodeCodabar" $ do
    it "marks the outer symbols and source indices" $ do
      encoded <- expectRight (encodeCodabar "40156" defaultCodabarGuards)
      map encodedCodabarCharacter encoded `shouldBe` "A40156A"
      map encodedCodabarSourceIndex encoded `shouldBe` [0 .. 6]
      map encodedCodabarRole encoded `shouldBe`
        [Start, Data, Data, Data, Data, Data, Stop]

    it "uses the exact shared reference patterns" $ do
      encoded <- expectRight (encodeCodabar "0+" defaultCodabarGuards)
      map encodedCodabarPattern encoded `shouldBe`
        ["1011001001", "101010011", "1011011011", "1011001001"]

    it "contains all 20 standard symbols" $ do
      body <- expectRight
        (encodeCodabar "0123456789-$:/.+" defaultCodabarGuards)
      guardB <- expectRight (encodeCodabar "BB" defaultCodabarGuards)
      guardsCD <- expectRight (encodeCodabar "CD" defaultCodabarGuards)
      let symbols = map encodedCodabarCharacter (body ++ guardB ++ guardsCD)
      sort (nub symbols) `shouldBe` sort "0123456789-$:/.+ABCD"

    it "encodes empty input as only two guards" $
      fmap (map encodedCodabarCharacter)
        (encodeCodabar "" defaultCodabarGuards) `shouldBe` Right "AA"

  describe "expandCodabarRuns" $ do
    it "expands one data symbol with two inter-character gaps" $ do
      runs <- expectRight (expandCodabarRuns "0" defaultCodabarGuards)
      length runs `shouldBe` 23
      map runRole (take 7 runs) `shouldBe` replicate 7 Start
      runs !! 7 `shouldBe` Barcode1DRun Space 1 "A" 0 InterCharacterGap
      map runRole (take 7 (drop 8 runs)) `shouldBe` replicate 7 Data
      runs !! 15 `shouldBe` Barcode1DRun Space 1 "0" 1 InterCharacterGap
      map runRole (drop 16 runs) `shouldBe` replicate 7 Stop

    it "uses exact shared module counts and alternating colors" $ do
      one <- expectRight (expandCodabarRuns "0" defaultCodabarGuards)
      sample <- expectRight (expandCodabarRuns "40156" defaultCodabarGuards)
      empty <- expectRight (expandCodabarRuns "" defaultCodabarGuards)
      sum (map runModules one) `shouldBe` 31
      sum (map runModules sample) `shouldBe` 71
      sum (map runModules empty) `shouldBe` 21
      map runColor one `shouldBe` take (length one) (cycle [Bar, Space])

    it "preserves custom start and stop attribution" $ do
      runs <- expectRight (expandCodabarRuns "0" (CodabarGuards 'B' 'D'))
      map runSourceLabel (take 7 runs) `shouldBe` replicate 7 "B"
      map runSourceLabel (drop 16 runs) `shouldBe` replicate 7 "D"

    it "propagates validation failures" $
      expandCodabarRuns "@" defaultCodabarGuards `shouldBe`
        Left (InvalidCodabarBodyCharacter '@')

  describe "layoutCodabar" $ do
    it "renders through the shared default geometry" $ do
      scene <- expectRight
        (layoutCodabar "0" defaultCodabarGuards defaultPaintBarcode1DOptions)
      psWidth scene `shouldBe` 204
      psHeight scene `shouldBe` 120
      psBg scene `shouldBe` "#ffffff"
      length (psInstructions scene) `shouldBe` 12
      map prX (take 2 (psInstructions scene)) `shouldBe` [40, 48]

    it "records normalized data, guards, label, and inferred symbols" $ do
      scene <- expectRight
        (layoutCodabar "40156" defaultCodabarGuards defaultPaintBarcode1DOptions)
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("codabar" :: String))
      Map.lookup "start" (psMeta scene) `shouldBe` Just (toJSON ("A" :: String))
      Map.lookup "stop" (psMeta scene) `shouldBe` Just (toJSON ("A" :: String))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("A40156A" :: String))
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Codabar barcode for A40156A" :: String))
      Map.lookup "symbolCount" (psMeta scene) `shouldBe` Just (toJSON (7 :: Int))

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
                , ("encodedText", toJSON ("wrong" :: String))
                , ("start", toJSON ("wrong" :: String))
                , ("stop", toJSON ("wrong" :: String))
                , ("symbology", toJSON ("wrong" :: String))
                ]
            }
      scene <- expectRight (layoutCodabar "0" (CodabarGuards 'B' 'D') options)
      (psWidth scene, psHeight scene, psBg scene) `shouldBe` (82, 50, "transparent")
      map prFill (psInstructions scene) `shouldBe` replicate 12 "navy"
      Map.lookup "label" (psMeta scene) `shouldBe`
        Just (toJSON ("Library label" :: String))
      Map.lookup "batch" (psMeta scene) `shouldBe` Just (toJSON (7 :: Int))
      Map.lookup "encodedText" (psMeta scene) `shouldBe`
        Just (toJSON ("B0D" :: String))
      Map.lookup "start" (psMeta scene) `shouldBe` Just (toJSON ("B" :: String))
      Map.lookup "stop" (psMeta scene) `shouldBe` Just (toJSON ("D" :: String))
      Map.lookup "symbology" (psMeta scene) `shouldBe`
        Just (toJSON ("codabar" :: String))

    it "supports the draw alias" $
      drawCodabar "0" defaultCodabarGuards defaultPaintBarcode1DOptions
        `shouldBe`
          layoutCodabar "0" defaultCodabarGuards defaultPaintBarcode1DOptions

    it "wraps shared layout validation failures" $ do
      let invalidGeometry = defaultPaintBarcode1DOptions
            { optionsRenderConfig = defaultBarcode1DRenderConfig
                { configModuleWidth = 0 }
            }
          humanText = defaultPaintBarcode1DOptions
            { optionsHumanReadableText = Just "A0A" }
      layoutCodabar "0" defaultCodabarGuards invalidGeometry `shouldBe`
        Left (CodabarLayoutError (InvalidRenderConfiguration "moduleWidth" 0))
      layoutCodabar "0" defaultCodabarGuards humanText `shouldBe`
        Left (CodabarLayoutError HumanReadableTextUnsupported)

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left err) = fail ("unexpected error: " ++ show err)
