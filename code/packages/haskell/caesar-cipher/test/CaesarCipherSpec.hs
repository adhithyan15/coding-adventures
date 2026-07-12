module CaesarCipherSpec (spec) where

import CaesarCipher
import Data.List (maximumBy, minimumBy)
import Data.Ord (comparing)
import Test.Hspec

spec :: Spec
spec = do
    describe "encrypt" $ do
        it "shifts uppercase and lowercase while preserving case" $ do
            encrypt "HELLO" 3 `shouldBe` "KHOOR"
            encrypt "hello" 3 `shouldBe` "khoor"
            encrypt "Hello, World!" 3 `shouldBe` "Khoor, Zruog!"
        it "passes non-letters and non-ASCII characters through" $ do
            encrypt "a-b-c 123!" 1 `shouldBe` "b-c-d 123!"
            encrypt "caf\x00e9 \x2764" 5 `shouldBe` "hfk\x00e9 \x2764"
        it "normalizes zero, full, large, and negative shifts" $ do
            encrypt "The Quick Brown Fox" 0 `shouldBe` "The Quick Brown Fox"
            encrypt "The Quick Brown Fox" 26 `shouldBe` "The Quick Brown Fox"
            encrypt "ABC" 52 `shouldBe` "ABC"
            encrypt "ABC" 29 `shouldBe` "DEF"
            encrypt "ABC" (-1) `shouldBe` "ZAB"
            encrypt "ABC" (-27) `shouldBe` "ZAB"
        it "wraps at the alphabet boundary" $ do
            encrypt "XYZ" 3 `shouldBe` "ABC"
            encrypt "xyz" 3 `shouldBe` "abc"

    describe "decrypt" $ do
        it "inverts the classic example" $
            decrypt "KHOOR" 3 `shouldBe` "HELLO"
        it "round-trips every shift from -30 through 30" $ do
            let original = "Attack at dawn! (meet by the OLD oak, 5pm)"
            mapM_ (checkRoundTrip original) [-30 .. 30]

    describe "rot13" $ do
        it "matches known values" $ do
            rot13 "Hello" `shouldBe` "Uryyb"
            rot13 "123!" `shouldBe` "123!"
        it "is its own inverse" $ do
            let text = "The Quick Brown Fox jumps over 13 lazy dogs."
            rot13 (rot13 text) `shouldBe` text
        it "equals encrypt with shift 13" $
            rot13 "Spoiler: the butler did it." `shouldBe`
                encrypt "Spoiler: the butler did it." 13

    describe "bruteForce" $ do
        it "returns all 25 non-trivial shifts in order" $
            map bruteForceShift (bruteForce "KHOOR") `shouldBe` [1 .. 25]
        it "contains the correct plaintext" $ do
            let result = bruteForce "KHOOR" !! 2
            result `shouldBe` BruteForceResult 3 "HELLO"
        it "preserves non-alphabetic ciphertext in every candidate" $
            map bruteForcePlaintext (bruteForce "123!!!") `shouldBe`
                replicate 25 "123!!!"

    describe "frequencyAnalysis" $ do
        it "recovers a shift from a pangram" $ do
            let plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
            frequencyAnalysis (encrypt plaintext 3) `shouldBe` (3, plaintext)
        it "recovers a shift from longer English text" $ do
            let plaintext =
                    "IN CRYPTOGRAPHY A CAESAR CIPHER ALSO KNOWN AS SHIFT CIPHER " ++
                    "IS ONE OF THE SIMPLEST AND MOST WIDELY KNOWN ENCRYPTION " ++
                    "TECHNIQUES IT IS A TYPE OF SUBSTITUTION CIPHER"
            frequencyAnalysis (encrypt plaintext 17) `shouldBe` (17, plaintext)
        it "defaults to shift one without alphabetic signal" $
            frequencyAnalysis "12345 !!! ???" `shouldBe` (1, "12345 !!! ???")

    describe "englishFrequencies" $ do
        it "contains 26 positive entries that sum to approximately one" $ do
            length englishFrequencies `shouldBe` 26
            all (> 0.0) englishFrequencies `shouldBe` True
            abs (sum englishFrequencies - 1.0) `shouldSatisfy` (< 0.01)
        it "identifies E as most common and Z as least common" $ do
            maximumBy (comparing snd) (zip [0 :: Int ..] englishFrequencies)
                `shouldBe` (4, englishFrequencies !! 4)
            minimumBy (comparing snd) (zip [0 :: Int ..] englishFrequencies)
                `shouldBe` (25, englishFrequencies !! 25)
  where
    checkRoundTrip original shift =
        decrypt (encrypt original shift) shift `shouldBe` original
