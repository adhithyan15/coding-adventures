module X25519Spec (spec) where

import Data.Word (Word8)
import Test.Hspec
import X25519

spec :: Spec
spec = describe "X25519" $ do
    it "derives Alice's public key from the RFC vector" $ do
        x25519Base alicePrivate
            `shouldBe` Right alicePublic

    it "derives Bob's public key from the RFC vector" $ do
        x25519Base bobPrivate
            `shouldBe` Right bobPublic

    it "derives the RFC shared secret" $ do
        x25519 alicePrivate bobPublic
            `shouldBe` Right sharedSecret

    it "agrees on the same shared secret from both sides" $ do
        let sharedAB = x25519 alicePrivate bobPublic
        let sharedBA = x25519 bobPrivate alicePublic
        sharedAB `shouldBe` Right sharedSecret
        sharedBA `shouldBe` Right sharedSecret

    it "generateKeypair matches x25519Base" $ do
        generateKeypair alicePrivate
            `shouldBe` Right alicePublic

alicePrivate :: [Word8]
alicePrivate = hexToBytes "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a"

alicePublic :: [Word8]
alicePublic = hexToBytes "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"

bobPrivate :: [Word8]
bobPrivate = hexToBytes "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb"

bobPublic :: [Word8]
bobPublic = hexToBytes "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"

sharedSecret :: [Word8]
sharedSecret = hexToBytes "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"

hexToBytes :: String -> [Word8]
hexToBytes [] = []
hexToBytes (a : b : rest) = fromIntegral (hexDigit a * 16 + hexDigit b) : hexToBytes rest
hexToBytes _ = []

hexDigit :: Char -> Int
hexDigit character
    | character >= '0' && character <= '9' = fromEnum character - fromEnum '0'
    | character >= 'a' && character <= 'f' = 10 + fromEnum character - fromEnum 'a'
    | character >= 'A' && character <= 'F' = 10 + fromEnum character - fromEnum 'A'
    | otherwise = 0
