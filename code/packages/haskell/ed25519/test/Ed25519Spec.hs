module Ed25519Spec (spec) where

import Data.Bits (xor)
import Data.Word (Word8)
import Ed25519
import Test.Hspec

spec :: Spec
spec = describe "Ed25519" $ do
    it "matches the RFC 8032 empty-message vector" $ do
        let generated = generateKeypair vector1Seed
        generated `shouldBe` Right (vector1Public, vector1Seed <> vector1Public)
        let signature = sign [] (vector1Seed <> vector1Public)
        signature `shouldBe` Right vector1Signature
        let verified = verify [] vector1Signature vector1Public
        verified `shouldBe` True

    it "matches the RFC 8032 one-byte vector" $ do
        let generated = generateKeypair vector2Seed
        generated `shouldBe` Right (vector2Public, vector2Seed <> vector2Public)
        let signature = sign [0x72] (vector2Seed <> vector2Public)
        signature `shouldBe` Right vector2Signature
        let verified = verify [0x72] vector2Signature vector2Public
        verified `shouldBe` True

    it "rejects a tampered signature" $ do
        let tampered = flipFirstBit vector2Signature
        let verified = verify [0x72] tampered vector2Public
        verified `shouldBe` False

    it "rejects a different message" $ do
        let verified = verify [0x73] vector2Signature vector2Public
        verified `shouldBe` False

vector1Seed :: [Word8]
vector1Seed = hexToBytes "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"

vector1Public :: [Word8]
vector1Public = hexToBytes "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"

vector1Signature :: [Word8]
vector1Signature = hexToBytes "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"

vector2Seed :: [Word8]
vector2Seed = hexToBytes "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"

vector2Public :: [Word8]
vector2Public = hexToBytes "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"

vector2Signature :: [Word8]
vector2Signature = hexToBytes "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"

flipFirstBit :: [Word8] -> [Word8]
flipFirstBit [] = []
flipFirstBit (value : rest) = (value `xor` 1) : rest

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
