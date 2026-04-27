{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
-- | Pure-Haskell Argon2d (RFC 9106) memory-hard password hashing.
--
-- Argon2d is the data-DEPENDENT variant of the Argon2 family: the index of
-- every reference block comes from the first 64 bits of the previously
-- computed block.  That correlation with the password maximises GPU/ASIC
-- resistance but leaks a timing side channel through memory-access pattern.
-- Use Argon2d only when side-channel attacks are NOT in the threat model
-- (proof-of-work schemes, etc.).  For general password hashing prefer
-- Argon2id.
--
-- Reference: <https://datatracker.ietf.org/doc/html/rfc9106>
--
-- == Haskell 64-bit notes
--
-- All 64-bit arithmetic uses 'Word64' with native wrap-on-overflow.  The
-- G-mixer's @2 * trunc32(a) * trunc32(b)@ cross-term is computed on
-- 'Word64' and wraps mod 2^64 automatically -- exactly what the spec wants.
--
-- The memory matrix is @Array (Int, Int) Block@ where 'Block' is a
-- 128-word @UArray Int Word64@.  RFC test-sized vectors fit in a few KiB,
-- so the O(n) cost of functional array updates is not a real concern.
module Argon2d
    ( -- * High-level API
      argon2d
    , argon2dHex
      -- * Constants
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
blockSize :: Int
blockSize = 1024             -- bytes per Argon2 memory block

blockWords :: Int
blockWords = 128             -- 64-bit words per block

syncPoints :: Int
syncPoints = 4               -- slices per pass

-- | The only approved Argon2 version.
argon2Version :: Int
argon2Version = 0x13

typeD :: Int
typeD = 0                    -- primitive type code for Argon2d

mask32 :: Word64
mask32 = 0xFFFFFFFF

type Block = UArray Int Word64

-- ---------------------------------------------------------------------
-- G-mixer (RFC 9106 §3.5).
--
-- BLAKE2's quarter-round PLUS the Argon2 @2 * trunc32(a) * trunc32(b)@
-- cross-term per add.  Word64 arithmetic wraps naturally.
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

-- One 8-round permutation P across a 16-word slice.  Four "column" rounds
-- on (0,4,8,12) etc. followed by four "diagonal" rounds on (0,5,10,15) etc.
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

-- | Compression function G: @r := x XOR y; row-pass; column-pass; r XOR q@.
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
-- Block <-> byte-string helpers.  Argon2 is little-endian everywhere.
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
-- H' -- Argon2 variable-length hash (RFC 9106 §3.3).
--
-- For @t <= 64@: single BLAKE2b call of size @t@.  Otherwise: chain
-- 64-byte BLAKE2b calls, take the first 32 bytes of each, and finish
-- with a BLAKE2b call sized to fit exactly.  The initial input is
-- @LE32(t) || x@.
-- ---------------------------------------------------------------------
blake2bLong :: Int -> [Word8] -> [Word8]
blake2bLong t x
    | t <= 0 = error "H' output length must be positive"
    | t <= 64 =
        blake2bWith defaultParams { digestSize = t } (le32 t ++ x)
    | otherwise =
        let r = (t + 31) `div` 32 - 2
            v0 = blake2bWith defaultParams { digestSize = 64 } (le32 t ++ x)
            -- Compute v_1 .. v_r (total r+1 blocks), taking the first 32
            -- bytes of each of the first r blocks.  The final call is
            -- sized to @finalSize = t - 32r@ and its FULL output is
            -- appended to the running prefix.
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
-- index_alpha (RFC 9106 §3.4.1.1).
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
-- Memory matrix: @Array (Int, Int) Block@ indexed by (lane, column).
-- ---------------------------------------------------------------------
type Memory = A.Array (Int, Int) Block

xorBlocks :: Block -> Block -> Block
xorBlocks a b = listArray (0, blockWords - 1)
                  [ (a ! i) `xor` (b ! i) | i <- [0 .. blockWords - 1] ]

-- Fill one (pass, slice, lane) segment.  Argon2d is entirely data-
-- dependent: the low / high 32 bits of the previous block's first word
-- supply J1 / J2.
fillSegment
    :: Memory -> Int -> Int -> Int -> Int -> Int -> Int -> Memory
fillSegment memory r lane sl q slLen p = foldl step memory [startingC .. slLen - 1]
  where
    startingC = if r == 0 && sl == 0 then 2 else 0
    step mem i =
        let col      = sl * slLen + i
            prevCol  = if col == 0 then q - 1 else col - 1
            prevBlock = mem A.! (lane, prevCol)
            pseudo   = prevBlock ! 0
            j1       = pseudo .&. mask32
            j2       = (pseudo `shiftR` 32) .&. mask32
            lPrime   = if r == 0 && sl == 0 then lane
                       else fromIntegral j2 `mod` p
            zPrime   = indexAlpha j1 r sl i (lPrime == lane) q slLen
            refBlock = mem A.! (lPrime, zPrime)
            newBlock = compress prevBlock refBlock
            finalBlock
              | r == 0    = newBlock
              | otherwise = xorBlocks (mem A.! (lane, col)) newBlock
         in mem A.// [((lane, col), finalBlock)]

-- ---------------------------------------------------------------------
-- Parameter validation (RFC 9106 §3.1).
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
-- argon2d -- compute the Argon2d tag (RFC 9106 §3).
--
-- Parameters mirror the sibling ports:
--
-- @
--   argon2d password salt timeCost memoryCost parallelism tagLength
--           key associatedData version
-- @
--
-- Use an empty list @[]@ for an absent key or associated-data input.
-- 'argon2Version' is the canonical version constant (@0x13@).
-- ---------------------------------------------------------------------
argon2d
    :: [Word8]   -- ^ password
    -> [Word8]   -- ^ salt (>= 8 bytes, 16+ recommended)
    -> Int       -- ^ timeCost (passes, >= 1)
    -> Int       -- ^ memoryCost (KiB, >= 8 * parallelism)
    -> Int       -- ^ parallelism (lanes, [1, 2^24-1])
    -> Int       -- ^ tagLength (output bytes, >= 4)
    -> [Word8]   -- ^ key (optional MAC secret)
    -> [Word8]   -- ^ associatedData (optional context)
    -> Int       -- ^ version (use 'argon2Version')
    -> [Word8]
argon2d password salt timeCost memoryCost parallelism tagLength
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
            , le32 version, le32 typeD
            , le32 (length password), password
            , le32 (length salt),     salt
            , le32 (length key),      key
            , le32 (length ad),       ad
            ]
        h0 = blake2bWith defaultParams { digestSize = 64 } h0Input

        -- Seed blocks (i, 0) and (i, 1) for each lane i.
        seedPairs =
            [ ((i, 0), bytesToBlock (blake2bLong blockSize (h0 ++ le32 0 ++ le32 i)))
            | i <- [0 .. p - 1] ]
         ++ [ ((i, 1), bytesToBlock (blake2bLong blockSize (h0 ++ le32 1 ++ le32 i)))
            | i <- [0 .. p - 1] ]

        emptyCell = listArray (0, blockWords - 1) (replicate blockWords 0)
        empty = A.listArray ((0, 0), (p - 1, q - 1))
                          (replicate (p * q) emptyCell)
        memory0 = empty A.// seedPairs

        -- Sequentially run t passes × syncPoints slices × p lanes.
        memoryFinal = foldl
            (\mem (r, sl, lane) -> fillSegment mem r lane sl q slLen p)
            memory0
            [ (r, sl, lane)
            | r  <- [0 .. t - 1]
            , sl <- [0 .. syncPoints - 1]
            , lane <- [0 .. p - 1]
            ]

        finalBlock = foldl xorBlocks (memoryFinal A.! (0, q - 1))
                           [ memoryFinal A.! (lane, q - 1) | lane <- [1 .. p - 1] ]
     in blake2bLong tagLength (blockToBytes finalBlock)

-- | 'argon2d' as a lowercase hex string.
argon2dHex
    :: [Word8] -> [Word8] -> Int -> Int -> Int -> Int
    -> [Word8] -> [Word8] -> Int -> String
argon2dHex password salt t m p tagLength key ad version =
    concatMap renderByte $
        argon2d password salt t m p tagLength key ad version
  where
    renderByte b =
        let hi = fromIntegral (b `shiftR` 4) :: Int
            lo = fromIntegral (b .&. 0x0F)   :: Int
         in [ intToDigit hi, intToDigit lo ]
