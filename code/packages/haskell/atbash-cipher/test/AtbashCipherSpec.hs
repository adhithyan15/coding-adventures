module AtbashCipherSpec (spec) where

import AtbashCipher
import Test.Hspec

spec :: Spec
spec = do
    describe "encrypt" $ do
        it "matches the classic HELLO example" $
            encrypt "HELLO" `shouldBe` "SVOOL"
        it "preserves letter case" $
            encrypt "Hello World" `shouldBe` "Svool Dliow"
        it "preserves punctuation, whitespace, and digits" $
            encrypt "Hello, World! 123" `shouldBe` "Svool, Dliow! 123"
        it "maps the complete alphabets in reverse order" $ do
            encrypt "ABCDEFGHIJKLMNOPQRSTUVWXYZ" `shouldBe`
                "ZYXWVUTSRQPONMLKJIHGFEDCBA"
            encrypt "abcdefghijklmnopqrstuvwxyz" `shouldBe`
                "zyxwvutsrqponmlkjihgfedcba"
        it "handles empty input" $
            encrypt "" `shouldBe` ""
        it "passes non-ASCII characters through" $
            encrypt "caf\x00e9 \x2764" `shouldBe` "xzu\x00e9 \x2764"

    describe "decrypt" $ do
        it "recovers the classic example" $
            decrypt "SVOOL" `shouldBe` "HELLO"
        it "has the same substitution as encrypt" $
            decrypt "Svool, Dliow!" `shouldBe` "Hello, World!"

    describe "Atbash invariants" $ do
        it "is its own inverse" $ do
            let original = "The Quick Brown Fox jumps over 13 lazy dogs."
            decrypt (encrypt original) `shouldBe` original
        it "maps the middle letters across the alphabet boundary" $ do
            encrypt "MNmn" `shouldBe` "NMnm"
        it "has no fixed point among ASCII letters" $ do
            let letters = ['A' .. 'Z'] ++ ['a' .. 'z']
            zipWith (/=) letters (encrypt letters) `shouldSatisfy` and
