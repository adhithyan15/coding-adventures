{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
-- | Pure-Haskell Argon2i (RFC 9106) memory-hard password hashing.
--
-- Argon2i is the data-INDEPENDENT variant of Argon2: reference-block
-- indices are derived from a deterministic address stream that depends
-- only on public parameters @(pass, lane, slice, m', t_total, TYPE_I,
-- counter)@, NOT on the password.  That eliminates the timing side
-- channel that Argon2d has but costs some GPU/ASIC resistance.  Use
-- Argon2i when side-channel safety is required.  Prefer Argon2id as
-- the general-purpose password-hashing default.
--
-- Reference: <https://datatracker.ietf.org/doc/html/rfc9106>
module Argon2i
    ( argon2i
    , argon2iHex
    , argon2Version
    ) where

import Blake2b (Params (..), blake2bWith, defaultParams)
import qualified Data.Array as A
import Data.Array.Unboxed (UArray, listArray, elems, (!), (//))
import Data.Bits ((.&.), (.|.), rotateR, shiftL, shiftR, xor)
import Data.Char (intToDigit)
import Data.Word (Word8, Word64)

-- ---------------------------------------------------------------------
-- Constants (RFC 9106 §3)
-- ---------------------------------------------------------------------
blockSize, blockWords, syncPoints, addressesPerBlock, typeI :: Int
blockSize         = 1024
blockWords        = 128
syncPoints        = 4
addressesPerBlock = 128
typeI             = 1

argon2Version :: Int
argon2Version = 0x13

mask32 :: Word64
mask32 = 0xFFFFFFFF

type Block = UArray Int Word64

-- ---------------------------------------------------------------------
-- G-mixer, permutation P, compression G (same as Argon2d).
-- ---------------------------------------------------------------------
gMix :: Word64 -> Word64 -> Word64 -> Word64
     -> (Word64, Word64, Word64, Word64)
gMix va vb vc vd =
    let va1 = va + vb + 2 * (va .&. mask32) * (vb .&. mask32)
        vd1 = rotateR (vd `xor` va1) 32
        vc1 = vc + vd1 + 2 * (vc .&. mask32) * (vd1 .&. mask32)
        vb1 = rotateR (vb `xor` vc1) 24
        va2 = va1 + vb1 + 2 * (va1 .&. mask32) * (vb1 .&. mask32)
        vd2 = rotateR (vd1 `xor` va2) 16
        vc2 = vc1 + vd2 + 2 * (vc1 .&. mask32) * (vd2 .&. mask32)
        vb2 = rotateR (vb1 `xor` vc2) 63
     in (va2, vb2, vc2, vd2)

permutationP :: Block -> Int -> Block
permutationP v off =
    let step ary a b c d =
            let (va', vb', vc', vd') =
                    gMix (ary ! a) (ary ! b) (ary ! c) (ary ! d)
             in ary // [(a, va'), (b, vb'), (c, vc'), (d, vd')]
        v1 = step v  (off + 0) (off + 4) (off +  8) (off + 12)
        v2 = step v1 (off + 1) (off + 5) (off +  9) (off + 13)
        v3 = step v2 (off + 2) (off + 6) (off + 10) (off + 14)
        v4 = step v3 (off + 3) (off + 7) (off + 11) (off + 15)
        v5 = step v4 (off + 0) (off + 5) (off + 10) (off + 15)
        v6 = step v5 (off + 1) (off + 6) (off + 11) (off + 12)
        v7 = step v6 (off + 2) (off + 7) (off +  8) (off + 13)
        v8 = step v7 (off + 3) (off + 4) (off +  9) (off + 14)
     in v8

compress :: Block -> Block -> Block
compress x y =
    let r  = listArray (0, blockWords - 1)
                [ (x ! i) `xor` (y ! i) | i <- [0 .. blockWords - 1] ]
        q1 = foldl (\acc i -> permutationP acc (i * 16)) r [0 .. 7]
        q2 = foldl columnPass q1 [0 .. 7]
     in listArray (0, blockWords - 1)
            [ (r ! i) `xor` (q2 ! i) | i <- [0 .. blockWords - 1] ]

columnPass :: Block -> Int -> Block
columnPass q c =
    let col0 = listArray (0, 15)
                 [ q ! (rr * 16 + 2 * c + k)
                 | rr <- [0 .. 7], k <- [0, 1] ]
        col1 = permutationP col0 0
        writes =
            [ (rr * 16 + 2 * c + k, col1 ! (2 * rr + k))
            | rr <- [0 .. 7], k <- [0, 1] ]
     in q // writes

-- ---------------------------------------------------------------------
-- Byte helpers
-- ---------------------------------------------------------------------
blockToBytes :: Block -> [Word8]
blockToBytes b = concatMap word64ToLeBytes (elems b)

bytesToBlock :: [Word8] -> Block
bytesToBlock bs = listArray (0, blockWords - 1)
                           (map leBytesToWord64 (chunksOf 8 bs))

word64ToLeBytes :: Word64 -> [Word8]
word64ToLeBytes w =
    [ fromIntegral ((w `shiftR` (8 * i)) .&. 0xFF) | i <- [0 .. 7 :: Int] ]

leBytesToWord64 :: [Word8] -> Word64
leBytesToWord64 bs =
    foldr (\(i, b) acc -> acc .|. (fromIntegral b `shiftL` (8 * i))) 0
          (zip [0 :: Int ..] bs)

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf n xs = let (h, t) = splitAt n xs in h : chunksOf n t

le32 :: Int -> [Word8]
le32 n =
    let w = fromIntegral n :: Word64
     in [ fromIntegral (w .&. 0xFF)
        , fromIntegral ((w `shiftR` 8) .&. 0xFF)
        , fromIntegral ((w `shiftR` 16) .&. 0xFF)
        , fromIntegral ((w `shiftR` 24) .&. 0xFF)
        ]

-- ---------------------------------------------------------------------
-- H' (same as Argon2d)
-- ---------------------------------------------------------------------
blake2bLong :: Int -> [Word8] -> [Word8]
blake2bLong t x
    | t <= 0 = error "H' output length must be positive"
    | t <= 64 =
        blake2bWith defaultParams { digestSize = t } (le32 t ++ x)
    | otherwise =
        let r = (t + 31) `div` 32 - 2
            v0 = blake2bWith defaultParams { digestSize = 64 } (le32 t ++ x)
            go !i !v !accRev
                | i > r = (v, accRev)
                | otherwise =
                    let v' = blake2bWith defaultParams { digestSize = 64 } v
                     in go (i + 1) v' (take 32 v' : accRev)
            (vR, middleRev) = go 2 v0 [take 32 v0]
            finalSize = t - 32 * r
            vLast = blake2bWith defaultParams { digestSize = finalSize } vR
         in concat (reverse middleRev) ++ vLast

-- ---------------------------------------------------------------------
-- index_alpha
-- ---------------------------------------------------------------------
indexAlpha :: Word64 -> Int -> Int -> Int -> Bool -> Int -> Int -> Int
indexAlpha j1 r sl c sameLane q slLen =
    let (w, start)
          | r == 0 && sl == 0 = (c - 1, 0)
          | r == 0 =
                ( if sameLane then sl * slLen + c - 1
                  else if c == 0 then sl * slLen - 1
                  else sl * slLen
                , 0)
          | otherwise =
                ( if sameLane then q - slLen + c - 1
                  else if c == 0 then q - slLen - 1
                  else q - slLen
                , ((sl + 1) * slLen) `mod` q )
        wU  = fromIntegral w :: Word64
        xx  = (j1 * j1) `shiftR` 32
        yy  = (wU * xx) `shiftR` 32
        rel = fromIntegral (wU - 1 - yy) :: Int
     in (start + rel) `mod` q

-- ---------------------------------------------------------------------
-- Memory matrix
-- ---------------------------------------------------------------------
type Memory = A.Array (Int, Int) Block

xorBlocks :: Block -> Block -> Block
xorBlocks a b = listArray (0, blockWords - 1)
                  [ (a ! i) `xor` (b ! i) | i <- [0 .. blockWords - 1] ]

zeroBlock :: Block
zeroBlock = listArray (0, blockWords - 1) (replicate blockWords 0)

-- Derive the @i@-th 128-word address chunk: @double-G(0, compress(0, input))@.
addressBlock
    :: Int -> Int -> Int -> Int -> Int -> Int -> Block
addressBlock r lane sl mPrime tTotal counter =
    let inputList =
            [ fromIntegral r      :: Word64
            , fromIntegral lane   :: Word64
            , fromIntegral sl     :: Word64
            , fromIntegral mPrime :: Word64
            , fromIntegral tTotal :: Word64
            , fromIntegral typeI  :: Word64
            , fromIntegral counter:: Word64
            ] ++ replicate (blockWords - 7) 0
        inputBlk = listArray (0, blockWords - 1) inputList
        z        = compress zeroBlock inputBlk
     in compress zeroBlock z

-- Fill one (pass, slice, lane) segment using the Argon2i address stream.
fillSegment
    :: Memory -> Int -> Int -> Int -> Int -> Int -> Int -> Int -> Int -> Memory
fillSegment memory r lane sl q slLen p mPrime tTotal =
    foldl step memory [startingC .. slLen - 1]
  where
    startingC = if r == 0 && sl == 0 then 2 else 0

    -- Cache address blocks: addressChunk i returns the chunk for 128-aligned i.
    addressFor i =
        let chunkIdx = i `div` addressesPerBlock + 1
         in addressBlock r lane sl mPrime tTotal chunkIdx

    step mem i =
        let addr      = addressFor i
            pseudo    = addr ! (i `mod` addressesPerBlock)
            j1        = pseudo .&. mask32
            j2        = (pseudo `shiftR` 32) .&. mask32
            col       = sl * slLen + i
            prevCol   = if col == 0 then q - 1 else col - 1
            prevBlock = mem A.! (lane, prevCol)
            lPrime    = if r == 0 && sl == 0 then lane
                        else fromIntegral j2 `mod` p
            zPrime    = indexAlpha j1 r sl i (lPrime == lane) q slLen
            refBlock  = mem A.! (lPrime, zPrime)
            newBlock  = compress prevBlock refBlock
            finalBlock
              | r == 0    = newBlock
              | otherwise = xorBlocks (mem A.! (lane, col)) newBlock
         in mem A.// [((lane, col), finalBlock)]

-- ---------------------------------------------------------------------
-- Parameter validation
-- ---------------------------------------------------------------------
validate
    :: [Word8] -> [Word8] -> Int -> Int -> Int -> Int
    -> [Word8] -> [Word8] -> Int -> ()
validate password salt t m p tagLength key ad version
    | fromIntegral (length password) > (mask32 :: Word64) =
        error "password length must fit in 32 bits"
    | length salt < 8 = error "salt must be at least 8 bytes"
    | fromIntegral (length salt) > (mask32 :: Word64) =
        error "salt length must fit in 32 bits"
    | fromIntegral (length key) > (mask32 :: Word64) =
        error "key length must fit in 32 bits"
    | fromIntegral (length ad) > (mask32 :: Word64) =
        error "associated_data length must fit in 32 bits"
    | tagLength < 4 = error "tag_length must be >= 4"
    | fromIntegral tagLength > (mask32 :: Word64) =
        error "tag_length must fit in 32 bits"
    | p < 1 || p > 0xFFFFFF = error "parallelism must be in [1, 2^24-1]"
    | m < 8 * p = error "memory_cost must be >= 8*parallelism"
    | fromIntegral m > (mask32 :: Word64) =
        error "memory_cost must fit in 32 bits"
    | t < 1 = error "time_cost must be >= 1"
    | version /= argon2Version =
        error "only Argon2 v1.3 (0x13) is supported"
    | otherwise = ()

-- ---------------------------------------------------------------------
-- argon2i
-- ---------------------------------------------------------------------
argon2i
    :: [Word8] -> [Word8] -> Int -> Int -> Int -> Int
    -> [Word8] -> [Word8] -> Int -> [Word8]
argon2i password salt timeCost memoryCost parallelism tagLength
        key ad version =
    let !_ = validate password salt timeCost memoryCost parallelism
                      tagLength key ad version
        segmentLength = memoryCost `div` (syncPoints * parallelism)
        mPrime        = segmentLength * syncPoints * parallelism
        q             = mPrime `div` parallelism
        slLen         = segmentLength
        p             = parallelism
        t             = timeCost

        h0Input = concat
            [ le32 p, le32 tagLength, le32 memoryCost, le32 t
            , le32 version, le32 typeI
            , le32 (length password), password
            , le32 (length salt),     salt
            , le32 (length key),      key
            , le32 (length ad),       ad
            ]
        h0 = blake2bWith defaultParams { digestSize = 64 } h0Input

        seedPairs =
            [ ((i, 0), bytesToBlock
                           (blake2bLong blockSize (h0 ++ le32 0 ++ le32 i)))
            | i <- [0 .. p - 1] ]
         ++ [ ((i, 1), bytesToBlock
                           (blake2bLong blockSize (h0 ++ le32 1 ++ le32 i)))
            | i <- [0 .. p - 1] ]

        empty   = A.listArray ((0, 0), (p - 1, q - 1))
                              (replicate (p * q) zeroBlock)
        memory0 = empty A.// seedPairs

        memoryFinal = foldl
            (\mem (r, sl, lane) ->
                 fillSegment mem r lane sl q slLen p mPrime t)
            memory0
            [ (r, sl, lane)
            | r  <- [0 .. t - 1]
            , sl <- [0 .. syncPoints - 1]
            , lane <- [0 .. p - 1]
            ]

        finalBlock = foldl xorBlocks (memoryFinal A.! (0, q - 1))
                           [ memoryFinal A.! (lane, q - 1)
                           | lane <- [1 .. p - 1] ]
     in blake2bLong tagLength (blockToBytes finalBlock)

argon2iHex
    :: [Word8] -> [Word8] -> Int -> Int -> Int -> Int
    -> [Word8] -> [Word8] -> Int -> String
argon2iHex password salt t m p tagLength key ad version =
    concatMap renderByte $
        argon2i password salt t m p tagLength key ad version
  where
    renderByte b =
        let hi = fromIntegral (b `shiftR` 4) :: Int
            lo = fromIntegral (b .&. 0x0F)   :: Int
         in [ intToDigit hi, intToDigit lo ]
