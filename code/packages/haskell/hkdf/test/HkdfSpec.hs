module HkdfSpec (spec) where

import Data.Word (Word8)
import Hkdf
import Numeric (showHex)
import Test.Hspec

spec :: Spec
spec = describe "Hkdf" $ do
    it "matches the RFC 5869 SHA-256 extract vector" $ do
        bytesToHex (hkdfExtract HkdfSha256 salt ikm)
            `shouldBe` "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"

    it "matches the RFC 5869 SHA-256 expand vector" $ do
        hkdfExpand HkdfSha256 prk info 42
            `shouldBe` Right okmSha256

    it "derives SHA-512 output" $ do
        hkdf HkdfSha512 (ascii "salt") (ascii "input") (ascii "info") 42
            `shouldBe` Right okmSha512

    it "rejects zero-length output" $ do
        hkdfExpand HkdfSha256 prk info 0
            `shouldBe` Left "HKDF output length must be positive"

salt :: [Word8]
salt = hexToBytes "000102030405060708090a0b0c"

ikm :: [Word8]
ikm = hexToBytes "0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b"

info :: [Word8]
info = hexToBytes "f0f1f2f3f4f5f6f7f8f9"

prk :: [Word8]
prk = hexToBytes "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"

okmSha256 :: [Word8]
okmSha256 = hexToBytes "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

okmSha512 :: [Word8]
okmSha512 = hexToBytes "93976f7542be922e353cf5440313ab4a877870039432e019c3b87b806713980b0781ec4dbe263624a457"

bytesToHex :: [Word8] -> String
bytesToHex =
    concatMap renderByte
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered

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

ascii :: String -> [Word8]
ascii = map (fromIntegral . fromEnum)
