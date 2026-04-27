module Pbkdf2Spec (spec) where

import Data.Word (Word8)
import Pbkdf2
import Test.Hspec

spec :: Spec
spec = describe "Pbkdf2" $ do
    it "matches the RFC 6070 SHA-1 vector" $ do
        pbkdf2Hex Pbkdf2Sha1 (ascii "password") (ascii "salt") 1 20
            `shouldBe` Right "0c60c80f961f0e71f3a9b524af6012062fe037a6"

    it "matches the common SHA-256 vector" $ do
        pbkdf2Hex Pbkdf2Sha256 (ascii "password") (ascii "salt") 1 32
            `shouldBe` Right "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"

    it "derives SHA-512 output" $ do
        pbkdf2Hex Pbkdf2Sha512 (ascii "password") (ascii "salt") 1 64
            `shouldBe` Right "867f70cf1ade02cff3752599a3a53dc4af34c7a669815ae5d513554e1c8cf252c02d470a285a0501bad999bfe943c08f050235d7d68b1da55e63f73b60a57fce"

    it "rejects zero iterations" $ do
        pbkdf2 Pbkdf2Sha256 (ascii "password") (ascii "salt") 0 32
            `shouldBe` Left "PBKDF2 iterations must be positive"

ascii :: String -> [Word8]
ascii = map (fromIntegral . fromEnum)
