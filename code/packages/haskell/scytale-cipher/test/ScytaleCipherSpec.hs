module ScytaleCipherSpec (spec) where

import ScytaleCipher
import Test.Hspec

spec :: Spec
spec = do
    describe "encrypt" $ do
        it "matches the HELLO WORLD reference vector" $
            encrypt "HELLO WORLD" 3 `shouldBe` Right "HLWLEOODL R "
        it "matches evenly divided reference vectors" $ do
            encrypt "ABCDEF" 2 `shouldBe` Right "ACEBDF"
            encrypt "ABCDEF" 3 `shouldBe` Right "ADBECF"
            encrypt "ABCDEFGH" 4 `shouldBe` Right "AEBFCGDH"
        it "pads incomplete rows with spaces" $
            encrypt "HELLO" 3 `shouldBe` Right "HLEOL "
        it "preserves every character through transposition" $
            encrypt "AB,CD!" 2 `shouldBe` Right "A,DBC!"
        it "returns empty text before validating a key" $
            encrypt "" 1 `shouldBe` Right ""

    describe "decrypt" $ do
        it "recovers the HELLO WORLD reference vector" $
            decrypt "HLWLEOODL R " 3 `shouldBe` Right "HELLO WORLD"
        it "recovers evenly divided reference vectors" $ do
            decrypt "ACEBDF" 2 `shouldBe` Right "ABCDEF"
            decrypt "ADBECF" 3 `shouldBe` Right "ABCDEF"
        it "supports uneven ciphertext lengths" $
            decrypt "ABCDEFGHIJ" 4 `shouldBe` Right "ADGIBEHJCF"
        it "strips only trailing padding spaces" $
            decrypt "HLEOL " 3 `shouldBe` Right "HELLO"
        it "returns empty text before validating a key" $
            decrypt "" 1 `shouldBe` Right ""
        it "matches portable Unicode, ragged, and literal-space vectors" $ do
            encrypt "A😀Bé" 3 `shouldBe` Right "Aé😀 B "
            decrypt "Aé😀 B " 3 `shouldBe` Right "A😀Bé"
            encrypt "Ae\x0301\&B" 3 `shouldBe` Right "ABe \x0301\& "
            decrypt "ABe \x0301\& " 3 `shouldBe` Right "Ae\x0301\&B"
            decrypt "ABCDEF" 4 `shouldBe` Right "ACEFBD"
            decrypt "A\tB " 2 `shouldBe` Right "AB\t"
            decrypt "A\x00a0\t \n " 3 `shouldBe` Right "A\t\n\x00a0"

    describe "round trips" $ do
        it "round-trips all valid keys for mixed text" $ do
            let original = "The quick brown fox jumps over 13 lazy dogs!"
            mapM_ (checkRoundTrip original) [2 .. length original]
        it "round-trips punctuation, newlines, and Unicode" $ do
            let original = "Hello,\nWorld! caf\x00e9 \x2764"
            (encrypt original 5 >>= (`decrypt` 5)) `shouldBe` Right original

    describe "key validation" $ do
        it "rejects keys below two" $ do
            encrypt "HELLO" 1 `shouldBe` Left "Key must be >= 2."
            decrypt "HELLO" 0 `shouldBe` Left "Key must be >= 2."
        it "rejects keys longer than non-empty text" $ do
            encrypt "HI" 3 `shouldBe` Left "Key must be <= text length."
            decrypt "HI" 3 `shouldBe` Left "Key must be <= text length."

    describe "bruteForce" $ do
        it "finds the original plaintext" $ do
            let Right ciphertext = encrypt "HELLO WORLD" 3
            bruteForce ciphertext `shouldSatisfy`
                either (const False) (elem (BruteForceResult 3 "HELLO WORLD"))
        it "tries every key from two through half the length" $
            fmap (map bruteForceKey) (bruteForce "ABCDEFGHIJ") `shouldBe` Right [2, 3, 4, 5]
        it "returns no candidates for short text" $ do
            bruteForce "AB" `shouldBe` Right []
            bruteForce "ABC" `shouldBe` Right []
        it "rejects work beyond the quadratic-output limit" $
            bruteForce (replicate (maxBruteForceTextLength + 1) 'A')
                `shouldBe` Left "scytale-brute-force-limit"
  where
    checkRoundTrip original key = do
        let encrypted = encrypt original key
        (encrypted >>= (`decrypt` key)) `shouldBe` Right original
