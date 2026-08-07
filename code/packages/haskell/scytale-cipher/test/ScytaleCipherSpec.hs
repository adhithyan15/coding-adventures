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
            bruteForce ciphertext `shouldContain`
                [BruteForceResult 3 "HELLO WORLD"]
        it "tries every key from two through half the length" $
            map bruteForceKey (bruteForce "ABCDEFGHIJ") `shouldBe` [2, 3, 4, 5]
        it "returns no candidates for short text" $ do
            bruteForce "AB" `shouldBe` []
            bruteForce "ABC" `shouldBe` []
  where
    checkRoundTrip original key = do
        let encrypted = encrypt original key
        (encrypted >>= (`decrypt` key)) `shouldBe` Right original
