module Chacha20Poly1305Spec (spec) where

import Chacha20Poly1305
import Data.Bits (xor)
import Data.Word (Word8, Word32)
import Test.Hspec

spec :: Spec
spec = describe "Chacha20Poly1305" $ do
    it "matches the RFC 8439 ChaCha20 keystream block" $ do
        chacha20Encrypt (replicate 64 0) key nonce counter
            `shouldBe` Right expectedKeystream

    it "matches the RFC 8439 Poly1305 vector" $ do
        poly1305Mac (ascii "Cryptographic Forum Research Group") polyKey
            `shouldBe` Right expectedTag

    it "round-trips AEAD encryption" $ do
        let plaintext = ascii "ChaCha20-Poly1305 protects messages."
        let aad = ascii "aad"
        (aeadEncrypt plaintext key nonce aad >>= (\(ciphertext, tag) -> aeadDecrypt ciphertext key nonce aad tag))
            `shouldBe` Right plaintext

    it "rejects tampered AEAD ciphertext" $ do
        let plaintext = ascii "tamper"
        case aeadEncrypt plaintext key nonce [] of
            Left err -> expectationFailure err
            Right (ciphertext, tag) ->
                aeadDecrypt (flipFirstBit ciphertext) key nonce [] tag
                    `shouldSatisfy` isLeft

key :: [Word8]
key = hexToBytes "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"

nonce :: [Word8]
nonce = hexToBytes "000000090000004a00000000"

counter :: Word32
counter = 1

expectedKeystream :: [Word8]
expectedKeystream =
    hexToBytes
        "10f1e7e4d13b5915500fdd1fa32071c4"
            ++ hexToBytes
                "c7d1f4c733c068030422aa9ac3d46c4e"
            ++ hexToBytes
                "d2826446079faa0914c2d705d98b02a2"
            ++ hexToBytes
                "b5129cd1de164eb9cbd083e8a2503c4e"

polyKey :: [Word8]
polyKey = hexToBytes "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b"

expectedTag :: [Word8]
expectedTag = hexToBytes "a8061dc1305136c6c22b8baf0c0127a9"

flipFirstBit :: [Word8] -> [Word8]
flipFirstBit [] = []
flipFirstBit (value : rest) = (value `xor` 1) : rest

isLeft :: Either a b -> Bool
isLeft (Left _) = True
isLeft _ = False

ascii :: String -> [Word8]
ascii = map (fromIntegral . fromEnum)

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
