module VigenereCipherSpec (spec) where

import VigenereCipher
import Test.Hspec

longEnglishText :: String
longEnglishText =
    concat
        [ "The quick brown fox jumps over the lazy dog and then runs around the "
        , "entire neighborhood looking for more adventures to embark upon while "
        , "the sun slowly sets behind the distant mountains casting long shadows "
        , "across the valley below where the river winds its way through ancient "
        , "forests filled with towering oak trees and singing birds that herald "
        , "the coming of spring with their melodious songs echoing through the "
        , "canopy above where squirrels chase each other from branch to branch "
        , "gathering acorns and other nuts for the long winter months ahead when "
        , "the ground will be covered in a thick blanket of pristine white snow "
        , "and the children will build snowmen and throw snowballs at each other "
        , "laughing and playing until their parents call them inside for dinner "
        , "where warm soup and fresh bread await them on the old wooden table"
        ]

encryptedWith :: String -> String
encryptedWith key =
    case encrypt longEnglishText key of
        Right ciphertext -> ciphertext
        Left message -> error message

spec :: Spec
spec = do
    describe "encrypt" $ do
        it "matches the ATTACKATDAWN parity vector" $
            encrypt "ATTACKATDAWN" "LEMON" `shouldBe` Right "LXFOPVEFRNHR"
        it "preserves mixed case and punctuation" $
            encrypt "Hello, World!" "key" `shouldBe` Right "Rijvs, Uyvjn!"
        it "handles lowercase text and mixed-case keys" $ do
            encrypt "attackatdawn" "lemon" `shouldBe` Right "lxfopvefrnhr"
            encrypt "ATTACKATDAWN" "LeMoN" `shouldBe` Right "LXFOPVEFRNHR"
        it "cycles single-character keys and wraps the alphabet" $ do
            encrypt "ABC" "B" `shouldBe` Right "BCD"
            encrypt "AB" "Z" `shouldBe` Right "ZA"
        it "allows a key longer than the alphabetic content" $
            encrypt "AB" "LONGERKEY" `shouldBe` Right "LP"
        it "does not advance the key on non-letters" $
            encrypt "A T" "LE" `shouldBe` Right "L X"
        it "passes digits, punctuation, and Unicode through unchanged" $
            encrypt "Hello 123! caf\x00e9 \x2764" "key"
                `shouldBe` Right "Rijvs 123! akj\x00e9 \x2764"
        it "encrypts empty text with a valid key" $
            encrypt "" "KEY" `shouldBe` Right ""
        it "rejects empty and non-ASCII keys before processing text" $ do
            encrypt "hello" "" `shouldBe` Left "key must not be empty"
            encrypt "hello" "key1" `shouldBe` Left "key must contain only ASCII letters"
            encrypt "" "caf\x00e9" `shouldBe` Left "key must contain only ASCII letters"

    describe "decrypt" $ do
        it "matches the ATTACKATDAWN parity vector" $
            decrypt "LXFOPVEFRNHR" "LEMON" `shouldBe` Right "ATTACKATDAWN"
        it "restores mixed case and punctuation" $
            decrypt "Rijvs, Uyvjn!" "key" `shouldBe` Right "Hello, World!"
        it "handles lowercase and empty ciphertext" $ do
            decrypt "lxfopvefrnhr" "lemon" `shouldBe` Right "attackatdawn"
            decrypt "" "KEY" `shouldBe` Right ""
        it "rejects invalid keys" $ do
            decrypt "hello" "" `shouldBe` Left "key must not be empty"
            decrypt "hello" "ke y" `shouldBe` Left "key must contain only ASCII letters"

    describe "round trips" $ do
        it "round-trips representative text and keys" $
            mapM_
                checkRoundTrip
                [ ("ATTACKATDAWN", "LEMON")
                , ("Hello, World!", "key")
                , ("The quick brown fox!", "SECRET")
                , ("abc def ghi", "xyz")
                , ("MiXeD CaSe 123", "AbCdE")
                , ("a", "z")
                , ("ZZZZZZ", "A")
                ]
        it "round-trips punctuation, newlines, and Unicode" $
            checkRoundTrip ("Hello,\nWorld! caf\x00e9 \x2764", "Mixed")

    describe "findKeyLength" $ do
        it "returns one for insufficient alphabetic signal" $ do
            findKeyLength "" `shouldBe` 1
            findKeyLength "A" `shouldBe` 1
            findKeyLength "A!B" `shouldBe` 1
        it "returns one when every candidate has zero coincidence" $
            findKeyLengthWithLimit "ABCD" 20 `shouldBe` 1
        it "recovers three-, five-, and six-letter key lengths" $ do
            findKeyLength (encryptedWith "KEY") `shouldBe` 3
            findKeyLength (encryptedWith "LEMON") `shouldBe` 5
            findKeyLength (encryptedWith "SECRET") `shouldBe` 6
        it "respects an explicit maximum length" $ do
            let result = findKeyLengthWithLimit (encryptedWith "LEMON") 3
            result `shouldSatisfy` (\value -> value >= 1 && value <= 3)
            findKeyLengthWithLimit (encryptedWith "LEMON") 1 `shouldBe` 1

    describe "findKey" $ do
        it "recovers known keys with chi-squared analysis" $ do
            findKey (encryptedWith "KEY") 3 `shouldBe` "KEY"
            findKey (encryptedWith "LEMON") 5 `shouldBe` "LEMON"
            findKey (encryptedWith "SECRET") 6 `shouldBe` "SECRET"
        it "returns an empty key for non-positive lengths" $ do
            findKey "ABC" 0 `shouldBe` ""
            findKey "ABC" (-1) `shouldBe` ""
        it "uses A for positions with no ciphertext group" $
            findKey "E" 3 `shouldBe` "AAA"

    describe "breakCipher" $ do
        it "recovers LEMON and the original plaintext" $
            breakCipher (encryptedWith "LEMON")
                `shouldBe` BreakResult "LEMON" longEnglishText
        it "recovers SECRET and the original plaintext" $
            breakCipher (encryptedWith "SECRET")
                `shouldBe` BreakResult "SECRET" longEnglishText
        it "returns a stable short-text result" $
            breakCipher "" `shouldBe` BreakResult "A" ""

    describe "englishFrequencies" $
        it "contains one positive value per ASCII letter" $ do
            length englishFrequencies `shouldBe` 26
            englishFrequencies `shouldSatisfy` all (> 0.0)
  where
    checkRoundTrip (text, key) =
        (encrypt text key >>= (`decrypt` key)) `shouldBe` Right text
