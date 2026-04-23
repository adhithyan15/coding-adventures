module AesSpec (spec) where

import Aes
import Data.Word (Word8)
import Test.Hspec

spec :: Spec
spec = describe "Aes" $ do
    it "encrypts the AES-128 FIPS vector" $ do
        encryptBlock plain key128
            `shouldBe` Right cipher128

    it "decrypts the AES-128 FIPS vector" $ do
        decryptBlock cipher128 key128
            `shouldBe` Right plain

    it "encrypts the AES-256 FIPS vector" $ do
        encryptBlock plain key256
            `shouldBe` Right cipher256

    it "rejects invalid key sizes" $ do
        encryptBlock plain [0 .. 14]
            `shouldSatisfy` isLeft

plain :: [Word8]
plain = hexToBytes "00112233445566778899aabbccddeeff"

key128 :: [Word8]
key128 = hexToBytes "000102030405060708090a0b0c0d0e0f"

cipher128 :: [Word8]
cipher128 = hexToBytes "69c4e0d86a7b0430d8cdb78070b4c55a"

key256 :: [Word8]
key256 = hexToBytes "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"

cipher256 :: [Word8]
cipher256 = hexToBytes "8ea2b7ca516745bfeafc49904b496089"

isLeft :: Either a b -> Bool
isLeft (Left _) = True
isLeft _ = False

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
