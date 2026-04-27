module NoteFrequencySpec (spec) where

import Test.Hspec
import NoteFrequency

spec :: Spec
spec = do
    describe "parseNote" $ do
        it "extracts fields" $ do
            parseNote "C#5" `shouldBe` Right (Note 'C' "#" 5)

        it "normalizes lowercase letters" $ do
            fmap show (parseNote "g4") `shouldBe` Right "G4"

        it "rejects malformed notes" $ do
            map parseNote ["", "A", "H4", "#4", "4A", "A##4", "Bb"] `shouldSatisfy` all isLeft

        it "rejects unsupported spellings" $ do
            parseNote "E#4" `shouldSatisfy` isLeft

    describe "semitonesFromA4" $ do
        it "matches the reference examples" $ do
            fmap semitonesFromA4 (parseNote "A4") `shouldBe` Right 0
            fmap semitonesFromA4 (parseNote "A5") `shouldBe` Right 12
            fmap semitonesFromA4 (parseNote "A3") `shouldBe` Right (-12)
            fmap semitonesFromA4 (parseNote "C4") `shouldBe` Right (-9)

    describe "frequency mapping" $ do
        it "matches the reference examples" $ do
            fmap frequency (parseNote "A4") `shouldBe` Right 440.0
            fmap frequency (parseNote "A5") `shouldBe` Right 880.0
            fmap frequency (parseNote "A3") `shouldBe` Right 220.0
            fmap (\value -> abs (value - 261.6255653005986) < 1e-12) (noteToFrequency "C4") `shouldBe` Right True
            fmap (\value -> abs (value - either (const 0) id (noteToFrequency "Db4")) < 1e-12) (noteToFrequency "C#4") `shouldBe` Right True

isLeft :: Either a b -> Bool
isLeft (Left _) = True
isLeft _ = False
