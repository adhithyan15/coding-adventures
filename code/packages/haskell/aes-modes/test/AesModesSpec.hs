module AesModesSpec (spec) where

import AesModes
import Data.Bits (xor)
import Data.Word (Word8)
import Test.Hspec

spec :: Spec
spec = describe "AesModes" $ do
    it "pads and unpads with PKCS#7" $ do
        pkcs7Unpad (pkcs7Pad [1, 2, 3, 4, 5]) `shouldBe` Right [1, 2, 3, 4, 5]

    it "round-trips ECB" $ do
        let plaintext = ascii "Sixteen byte msg plus tail"
        (ecbEncrypt plaintext key >>= (`ecbDecrypt` key))
            `shouldBe` Right plaintext

    it "round-trips CBC" $ do
        let plaintext = ascii "CBC mode keeps chaining honest."
        (cbcEncrypt plaintext key iv >>= (\ciphertext -> cbcDecrypt ciphertext key iv))
            `shouldBe` Right plaintext

    it "round-trips CTR" $ do
        let plaintext = ascii "CTR mode behaves like a stream cipher."
        (ctrEncrypt plaintext key nonce12 >>= (\ciphertext -> ctrDecrypt ciphertext key nonce12))
            `shouldBe` Right plaintext

    it "round-trips GCM and authenticates AAD" $ do
        let plaintext = ascii "GCM gives confidentiality and integrity."
        let aad = ascii "header"
        (gcmEncrypt plaintext key nonce12 aad >>= (\(ciphertext, tag) -> gcmDecrypt ciphertext key nonce12 aad tag))
            `shouldBe` Right plaintext

    it "rejects tampered GCM ciphertext" $ do
        let plaintext = ascii "tamper me"
        let aad = ascii "aad"
        case gcmEncrypt plaintext key nonce12 aad of
            Left err -> expectationFailure err
            Right (ciphertext, tag) ->
                gcmDecrypt (flipFirstBit ciphertext) key nonce12 aad tag
                    `shouldSatisfy` isLeft

key :: [Word8]
key = hexToBytes "2b7e151628aed2a6abf7158809cf4f3c"

iv :: [Word8]
iv = replicate 16 0

nonce12 :: [Word8]
nonce12 = take 12 [0 ..]

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
