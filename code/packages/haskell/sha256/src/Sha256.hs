-- | Pure one-shot and bounded incremental SHA-256 hashing.
module Sha256
    ( description
    , Sha256Context
    , sha256Init
    , sha256Update
    , sha256Finalize
    , sha256FinalizeHex
    , sha256Copy
    , sha256
    , sha256Hex
    ) where

import Data.Bits
    ( (.&.)
    , complement
    , rotateR
    , shiftR
    , xor
    )
import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Data.List (foldl')
import Data.Word (Word8, Word32, Word64)
import Numeric (showHex)
import Sha256.Internal (checkedAdvanceBytes)

-- | Human-readable package description.
description :: String
description = "SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch"

data HashState = HashState
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32

data WorkingState = WorkingState
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32
    !Word32

-- | Opaque immutable state for incremental SHA-256 hashing.
--
-- The retained buffer is always shorter than one 64-byte compression block.
-- Its bytes are copied on update so a small remainder never keeps a caller's
-- larger input chunk alive.
data Sha256Context = Sha256Context !HashState !Word64 !ByteString

initialState :: HashState
initialState =
    HashState
        0x6A09E667
        0xBB67AE85
        0x3C6EF372
        0xA54FF53A
        0x510E527F
        0x9B05688C
        0x1F83D9AB
        0x5BE0CD19

roundConstants :: [Word32]
roundConstants =
    [ 0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5
    , 0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5
    , 0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3
    , 0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174
    , 0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC
    , 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA
    , 0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7
    , 0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967
    , 0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13
    , 0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85
    , 0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3
    , 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070
    , 0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5
    , 0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3
    , 0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208
    , 0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2
    ]

-- | An initialized incremental SHA-256 context.
sha256Init :: Sha256Context
sha256Init = Sha256Context initialState 0 BS.empty

-- | Feed one exact byte chunk into a context.
--
-- Empty updates are identity. Complete blocks are compressed immediately and
-- only a copied suffix shorter than 64 bytes is retained. Messages must remain
-- shorter than @2^64@ bits; an update that exceeds that FIPS 180-4 domain fails
-- with a deterministic error instead of wrapping the encoded length.
sha256Update :: Sha256Context -> ByteString -> Sha256Context
sha256Update context chunk
    | BS.null chunk = context
sha256Update (Sha256Context state totalBytes buffered) chunk =
    Sha256Context nextState nextTotalBytes ownedRemainder
  where
    combined
        | BS.null buffered = chunk
        | otherwise = BS.append buffered chunk
    completeLength = BS.length combined - (BS.length combined `mod` 64)
    (completeBytes, remainder) = BS.splitAt completeLength combined
    nextState = compressBlocks state completeBytes
    nextTotalBytes =
        case checkedAdvanceBytes totalBytes (fromIntegral (BS.length chunk)) of
            Just byteCount -> byteCount
            Nothing -> error "sha256Update: message length must be less than 2^64 bits"
    ownedRemainder = BS.copy remainder

-- | Return the 32-byte digest without consuming the context.
sha256Finalize :: Sha256Context -> ByteString
sha256Finalize (Sha256Context state totalBytes buffered) =
    renderState (compressBlocks state finalBlocks)
  where
    remainderLength = BS.length buffered
    zeroCount = (56 - ((remainderLength + 1) `mod` 64)) `mod` 64
    bitLength = totalBytes * 8
    finalBlocks =
        BS.concat
            [ buffered
            , BS.singleton 0x80
            , BS.replicate zeroCount 0
            , BS.pack (word64ToBytes bitLength)
            ]

-- | Return the lowercase hexadecimal digest without consuming the context.
sha256FinalizeHex :: Sha256Context -> String
sha256FinalizeHex = renderHex . BS.unpack . sha256Finalize

-- | Branch an immutable context. Copying is O(1) and identity-safe.
sha256Copy :: Sha256Context -> Sha256Context
sha256Copy = id

-- | Hash a complete legacy list of bytes.
sha256 :: [Word8] -> [Word8]
sha256 = BS.unpack . sha256Finalize . sha256Update sha256Init . BS.pack

-- | Hash a complete legacy list of bytes and render lowercase hexadecimal.
sha256Hex :: [Word8] -> String
sha256Hex = sha256FinalizeHex . sha256Update sha256Init . BS.pack

renderHex :: [Word8] -> String
renderHex = concatMap renderByte
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered

compressBlocks :: HashState -> ByteString -> HashState
compressBlocks state bytes
    | BS.null bytes = state
    | otherwise =
        let (block, rest) = BS.splitAt 64 bytes
            nextState = compress state block
         in nextState `seq` compressBlocks nextState rest

compress :: HashState -> ByteString -> HashState
compress (HashState a0 b0 c0 d0 e0 f0 g0 h0) block =
    HashState
        (a0 + aFinal)
        (b0 + bFinal)
        (c0 + cFinal)
        (d0 + dFinal)
        (e0 + eFinal)
        (f0 + fFinal)
        (g0 + gFinal)
        (h0 + hFinal)
  where
    scheduleWords = messageSchedule block
    WorkingState aFinal bFinal cFinal dFinal eFinal fFinal gFinal hFinal =
        foldl'
            roundStep
            (WorkingState a0 b0 c0 d0 e0 f0 g0 h0)
            (zip roundConstants scheduleWords)

roundStep :: WorkingState -> (Word32, Word32) -> WorkingState
roundStep (WorkingState a b c d e f g h) (constantValue, wordValue) =
    WorkingState
        (t1 + t2)
        a
        b
        c
        (d + t1)
        e
        f
        g
  where
    t1 = h + bigSigma1 e + choose e f g + constantValue + wordValue
    t2 = bigSigma0 a + majority a b c

messageSchedule :: ByteString -> [Word32]
messageSchedule block =
    initialWords ++ expand initialWords
  where
    initialWords = map bytesToWord32 (chunksOf 4 (BS.unpack block))
    expand wordsSoFar
        | length wordsSoFar == 64 = []
        | otherwise =
            let nextValue =
                    smallSigma1 (wordsSoFar !! 14)
                        + (wordsSoFar !! 9)
                        + smallSigma0 (wordsSoFar !! 1)
                        + (wordsSoFar !! 0)
                rotated = drop 1 wordsSoFar ++ [nextValue]
             in nextValue : expand rotated

choose :: Word32 -> Word32 -> Word32 -> Word32
choose x y z = (x .&. y) `xor` (complement x .&. z)

majority :: Word32 -> Word32 -> Word32 -> Word32
majority x y z = (x .&. y) `xor` (x .&. z) `xor` (y .&. z)

bigSigma0 :: Word32 -> Word32
bigSigma0 x = rotateRight32 2 x `xor` rotateRight32 13 x `xor` rotateRight32 22 x

bigSigma1 :: Word32 -> Word32
bigSigma1 x = rotateRight32 6 x `xor` rotateRight32 11 x `xor` rotateRight32 25 x

smallSigma0 :: Word32 -> Word32
smallSigma0 x = rotateRight32 7 x `xor` rotateRight32 18 x `xor` (x `shiftR` 3)

smallSigma1 :: Word32 -> Word32
smallSigma1 x = rotateRight32 17 x `xor` rotateRight32 19 x `xor` (x `shiftR` 10)

rotateRight32 :: Int -> Word32 -> Word32
rotateRight32 count value = rotateR value count

renderState :: HashState -> ByteString
renderState (HashState a b c d e f g h) =
    BS.pack (concatMap word32ToBytes [a, b, c, d, e, f, g, h])

word32ToBytes :: Word32 -> [Word8]
word32ToBytes wordValue =
    [ fromIntegral (wordValue `shiftR` 24)
    , fromIntegral (wordValue `shiftR` 16)
    , fromIntegral (wordValue `shiftR` 8)
    , fromIntegral wordValue
    ]

word64ToBytes :: Word64 -> [Word8]
word64ToBytes wordValue =
    [ fromIntegral (wordValue `shiftR` 56)
    , fromIntegral (wordValue `shiftR` 48)
    , fromIntegral (wordValue `shiftR` 40)
    , fromIntegral (wordValue `shiftR` 32)
    , fromIntegral (wordValue `shiftR` 24)
    , fromIntegral (wordValue `shiftR` 16)
    , fromIntegral (wordValue `shiftR` 8)
    , fromIntegral wordValue
    ]

bytesToWord32 :: [Word8] -> Word32
bytesToWord32 [a, b, c, d] =
    fromIntegral a * 2 ^ (24 :: Int)
        + fromIntegral b * 2 ^ (16 :: Int)
        + fromIntegral c * 2 ^ (8 :: Int)
        + fromIntegral d
bytesToWord32 _ = 0

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf size values =
    let (chunk, rest) = splitAt size values
     in chunk : chunksOf size rest
