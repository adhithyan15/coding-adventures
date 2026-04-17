module AesModes
    ( description
    , pkcs7Pad
    , pkcs7Unpad
    , ecbEncrypt
    , ecbDecrypt
    , cbcEncrypt
    , cbcDecrypt
    , ctrEncrypt
    , ctrDecrypt
    , gcmEncrypt
    , gcmDecrypt
    ) where

import Aes (decryptBlock, encryptBlock)
import Data.Bits
    ( (.|.)
    , shiftL
    , shiftR
    , testBit
    , xor
    )
import Data.List (foldl')
import Data.Word (Word8)

description :: String
description = "AES modes of operation including ECB, CBC, CTR, and GCM"

blockSize :: Int
blockSize = 16

pkcs7Pad :: [Word8] -> [Word8]
pkcs7Pad inputBytes =
    inputBytes ++ replicate padLength (fromIntegral padLength)
  where
    padLength = blockSize - (length inputBytes `mod` blockSize)

pkcs7Unpad :: [Word8] -> Either String [Word8]
pkcs7Unpad inputBytes
    | null inputBytes || length inputBytes `mod` blockSize /= 0 =
        Left "Invalid padded data: length must be a positive multiple of 16"
    | padLength == 0 || padLength > blockSize =
        Left "Invalid PKCS#7 padding"
    | any (/= fromIntegral padLength) (drop (length inputBytes - padLength) inputBytes) =
        Left "Invalid PKCS#7 padding"
    | otherwise =
        Right (take (length inputBytes - padLength) inputBytes)
  where
    padLength = fromIntegral (last inputBytes)

ecbEncrypt :: [Word8] -> [Word8] -> Either String [Word8]
ecbEncrypt plaintext keyBytes =
    fmap concat (mapM (`encryptBlock` keyBytes) (chunksOf 16 (pkcs7Pad plaintext)))

ecbDecrypt :: [Word8] -> [Word8] -> Either String [Word8]
ecbDecrypt ciphertext keyBytes
    | null ciphertext || length ciphertext `mod` blockSize /= 0 =
        Left "ECB ciphertext must be a non-empty multiple of 16 bytes"
    | otherwise = do
        decrypted <- fmap concat (mapM (`decryptBlock` keyBytes) (chunksOf 16 ciphertext))
        pkcs7Unpad decrypted

cbcEncrypt :: [Word8] -> [Word8] -> [Word8] -> Either String [Word8]
cbcEncrypt plaintext keyBytes iv
    | length iv /= blockSize =
        Left ("CBC IV must be 16 bytes, got " ++ show (length iv))
    | otherwise =
        fmap fst (foldM encryptChunk ([], iv) (chunksOf 16 (pkcs7Pad plaintext)))
  where
    encryptChunk (accumulator, previousBlock) chunk = do
        encrypted <- encryptBlock (xorBytes chunk previousBlock) keyBytes
        Right (accumulator ++ encrypted, encrypted)

cbcDecrypt :: [Word8] -> [Word8] -> [Word8] -> Either String [Word8]
cbcDecrypt ciphertext keyBytes iv
    | length iv /= blockSize =
        Left ("CBC IV must be 16 bytes, got " ++ show (length iv))
    | null ciphertext || length ciphertext `mod` blockSize /= 0 =
        Left "CBC ciphertext must be a non-empty multiple of 16 bytes"
    | otherwise = do
        decrypted <- fmap fst (foldM decryptChunk ([], iv) (chunksOf 16 ciphertext))
        pkcs7Unpad decrypted
  where
    decryptChunk (accumulator, previousBlock) chunk = do
        decrypted <- decryptBlock chunk keyBytes
        let plaintext = xorBytes decrypted previousBlock
        Right (accumulator ++ plaintext, chunk)

ctrEncrypt :: [Word8] -> [Word8] -> [Word8] -> Either String [Word8]
ctrEncrypt inputBytes keyBytes nonce
    | length nonce /= 12 =
        Left ("CTR nonce must be 12 bytes, got " ++ show (length nonce))
    | otherwise =
        fmap concat (sequence (zipWith xorWithCounter [1 ..] (chunksOf 16 inputBytes)))
  where
    xorWithCounter counter chunk = do
        keystream <- encryptBlock (buildCounterBlock nonce counter) keyBytes
        Right (zipWith xor chunk keystream)

ctrDecrypt :: [Word8] -> [Word8] -> [Word8] -> Either String [Word8]
ctrDecrypt = ctrEncrypt

gcmEncrypt :: [Word8] -> [Word8] -> [Word8] -> [Word8] -> Either String ([Word8], [Word8])
gcmEncrypt plaintext keyBytes iv aad
    | length iv /= 12 =
        Left ("GCM IV must be 12 bytes, got " ++ show (length iv))
    | otherwise = do
        hashSubkey <- encryptBlock (replicate 16 0) keyBytes
        let j0 = iv ++ [0, 0, 0, 1]
        ciphertext <- ctrFromCounter plaintext keyBytes j0 2
        encryptedJ0 <- encryptBlock j0 keyBytes
        let tag = xorBytes (ghash hashSubkey aad ciphertext) encryptedJ0
        Right (ciphertext, tag)

gcmDecrypt :: [Word8] -> [Word8] -> [Word8] -> [Word8] -> [Word8] -> Either String [Word8]
gcmDecrypt ciphertext keyBytes iv aad tag
    | length iv /= 12 =
        Left ("GCM IV must be 12 bytes, got " ++ show (length iv))
    | length tag /= 16 =
        Left ("GCM tag must be 16 bytes, got " ++ show (length tag))
    | otherwise = do
        hashSubkey <- encryptBlock (replicate 16 0) keyBytes
        let j0 = iv ++ [0, 0, 0, 1]
        encryptedJ0 <- encryptBlock j0 keyBytes
        let computedTag = xorBytes (ghash hashSubkey aad ciphertext) encryptedJ0
        if constantTimeEq computedTag tag
            then ctrFromCounter ciphertext keyBytes j0 2
            else Left "GCM authentication failed: tag mismatch"

ctrFromCounter :: [Word8] -> [Word8] -> [Word8] -> Word32Like -> Either String [Word8]
ctrFromCounter inputBytes keyBytes initialCounterBlock initialCounter =
    fmap concat (sequence (zipWith xorWithCounter [initialCounter ..] (chunksOf 16 inputBytes)))
  where
    nonce = take 12 initialCounterBlock
    xorWithCounter counter chunk = do
        keystream <- encryptBlock (buildCounterBlock nonce counter) keyBytes
        Right (zipWith xor chunk keystream)

type Word32Like = Int

buildCounterBlock :: [Word8] -> Word32Like -> [Word8]
buildCounterBlock nonce counter =
    nonce ++ word32ToBytesBE counter

word32ToBytesBE :: Int -> [Word8]
word32ToBytesBE value =
    [ fromIntegral (shiftR value 24)
    , fromIntegral (shiftR value 16)
    , fromIntegral (shiftR value 8)
    , fromIntegral value
    ]

ghash :: [Word8] -> [Word8] -> [Word8] -> [Word8]
ghash hashSubkey aad ciphertext =
    integerToBytes16BE finalValue
  where
    hValue = bytesToIntegerBE hashSubkey
    process accumulator block = gf128Mul (accumulator `xor` bytesToIntegerBE block) hValue
    aadBlocks = map padTo16 (chunksOf 16 aad)
    ciphertextBlocks = map padTo16 (chunksOf 16 ciphertext)
    accumulated = foldl' process 0 (aadBlocks ++ ciphertextBlocks)
    lengthBlock =
        integerToBytes16BE
            ((fromIntegral (length aad) * 8 `shiftL` 64) + fromIntegral (length ciphertext) * 8)
    finalValue = process accumulated lengthBlock

gf128Mul :: Integer -> Integer -> Integer
gf128Mul xValue yValue =
    go 0 yValue 0
  where
    reduction = bytesToIntegerBE [0xe1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    go result vValue bitIndex
        | bitIndex >= 128 = result
        | otherwise =
            let result' =
                    if testBit xValue (127 - bitIndex)
                        then result `xor` vValue
                        else result
                carry = odd vValue
                vValue' =
                    if carry
                        then shiftR vValue 1 `xor` reduction
                        else shiftR vValue 1
             in go result' vValue' (bitIndex + 1)

bytesToIntegerBE :: [Word8] -> Integer
bytesToIntegerBE =
    foldl' (\accumulator byteValue -> shiftL accumulator 8 + fromIntegral byteValue) 0

integerToBytes16BE :: Integer -> [Word8]
integerToBytes16BE value =
    [ fromIntegral (shiftR value (8 * offset))
    | offset <- [15, 14 .. 0]
    ]

padTo16 :: [Word8] -> [Word8]
padTo16 chunk =
    take 16 (chunk ++ repeat 0)

xorBytes :: [Word8] -> [Word8] -> [Word8]
xorBytes = zipWith xor

constantTimeEq :: [Word8] -> [Word8] -> Bool
constantTimeEq left right =
    length left == length right
        && foldl' (.|.) 0 (zipWith xor left right) == 0

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf size values =
    let (chunk, rest) = splitAt size values
     in chunk : chunksOf size rest

foldM :: (b -> a -> Either String b) -> b -> [a] -> Either String b
foldM _ accumulator [] = Right accumulator
foldM step accumulator (value : rest) =
    case step accumulator value of
        Left err -> Left err
        Right accumulator' -> foldM step accumulator' rest
