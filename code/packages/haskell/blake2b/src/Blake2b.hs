-- | Pure Haskell BLAKE2b (RFC 7693) cryptographic hash function.
--
-- BLAKE2b is the 64-bit variant of the BLAKE2 family: faster than MD5 on
-- modern hardware and at least as secure as SHA-3 against every known
-- attack.  Designed in 2012 as a drop-in replacement for SHA-2 in
-- performance-sensitive contexts; used internally by Argon2, libsodium,
-- WireGuard, Noise Protocol, and IPFS.
--
-- Why this package exists:
--   BLAKE2b is a hard prerequisite for Argon2 (the memory-hard password
--   hashing function).  The larger HF06 spec stands up BLAKE2b in ten
--   languages; this Haskell port mirrors the Python, Go, TypeScript,
--   Rust, Ruby, Elixir, Swift, Lua, and Perl siblings using the same
--   cross-language KAT table.
--
-- The algorithm in one diagram:
--
-- >   Input bytes (any length)          Key (optional, 0..64 bytes)
-- >          |                                 |
-- >          |  (if keyed, prepend key block)  |
-- >          +----------------<----------------+
-- >          v
-- >   +----------+----------+----------+
-- >   |  block_0 |  block_1 |   ...    |  (each 128 bytes)
-- >   +----------+----------+----------+
-- >          |
-- >          v
-- >      [h[0..7]] -> F -> F -> ... -> F(final=true) -> digest[:nn]
--
-- The state @h@ is eight 'Word64' words initialized from SHA-512's IVs
-- XOR-ed with a parameter block encoding output length, key length,
-- salt, and personalization.  The compression function @F@ mixes one
-- 128-byte block into the state over 12 ARX rounds.  The final block
-- is compressed with @final=true@ (v[14] inversion), which differentiates
-- it from intermediate calls and prevents length-extension attacks.
--
-- Scope: sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp,
-- BLAKE2sp, BLAKE2Xb, and BLAKE3 are out of scope.
module Blake2b
    ( -- * Description
      description
      -- * One-shot API
    , blake2b
    , blake2bHex
      -- * Parameterized API
    , Params (..)
    , defaultParams
    , blake2bWith
    , blake2bHexWith
    ) where

import Data.Bits ((.&.), (.|.), rotateR, shiftL, shiftR, xor)
import Data.List (foldl')
import Data.Word (Word8, Word64)
import Numeric (showHex)

description :: String
description =
    "BLAKE2b (RFC 7693) cryptographic hash function implemented from scratch"

-- | Parameters for a BLAKE2b invocation.  Use 'defaultParams' and the
-- record-update syntax to customise individual fields:
--
-- > blake2bWith defaultParams { digestSize = 32, key = myKey } input
data Params = Params
    { digestSize :: !Int
    -- ^ Output length in bytes, in @[1, 64]@.  Default 64.
    , key :: ![Word8]
    -- ^ Optional MAC key, @0@ to @64@ bytes.  Default empty.
    , salt :: ![Word8]
    -- ^ Optional salt, either exactly 16 bytes or empty.  Default empty.
    , personal :: ![Word8]
    -- ^ Optional personalization string, either exactly 16 bytes or
    -- empty.  Default empty.
    }
    deriving (Eq, Show)

defaultParams :: Params
defaultParams =
    Params
        { digestSize = 64
        , key = []
        , salt = []
        , personal = []
        }

-- | One-shot BLAKE2b with default 64-byte output and no key/salt/personal.
blake2b :: [Word8] -> [Word8]
blake2b = blake2bWith defaultParams

-- | One-shot BLAKE2b as a lowercase hex string.
blake2bHex :: [Word8] -> String
blake2bHex = blake2bHexWith defaultParams

-- | BLAKE2b with arbitrary parameters.  Validates the parameters and
-- raises 'error' on out-of-range inputs (mirroring the exception-
-- raising behaviour of the sibling ports).
blake2bWith :: Params -> [Word8] -> [Word8]
blake2bWith params input =
    let Params {digestSize = ds} = validate params
        (h0, byteCountStart, firstBuffer) = initHashing params
        -- Fold over all but the last block at final=False.  The last
        -- block (i.e. the buffer's final contents) is compressed in
        -- one final step with final=True.
        allBytes = firstBuffer ++ input
        (hAfterIntermediate, remaining, byteCount) =
            absorb h0 byteCountStart allBytes
        finalBlock = pad128 remaining
        finalByteCount = byteCount + length remaining
        hFinal = compressBlock hAfterIntermediate finalBlock finalByteCount True
     in take ds (concatMap word64ToLeBytes hFinal)

-- | BLAKE2b with arbitrary parameters, returning a lowercase hex string.
blake2bHexWith :: Params -> [Word8] -> String
blake2bHexWith params = concatMap renderByte . blake2bWith params
  where
    renderByte byteValue =
        let rendered = showHex byteValue ""
         in if length rendered == 1 then '0' : rendered else rendered

-- ---------------------------------------------------------------------
-- Parameter validation
--
-- Mirrors the validation logic of every sibling BLAKE2b port.  'error'
-- is used rather than returning an 'Either' because the test suite
-- expects to see an exception, and because in practice invalid
-- parameters indicate a programmer error rather than a runtime
-- condition.
-- ---------------------------------------------------------------------
validate :: Params -> Params
validate p
    | digestSize p < 1 || digestSize p > 64 =
        error $ "digest_size must be in [1, 64], got " ++ show (digestSize p)
    | length (key p) > 64 =
        error $
            "key length must be in [0, 64], got " ++ show (length (key p))
    | not (length (salt p) `elem` [0, 16]) =
        error $
            "salt must be exactly 16 bytes (or empty), got "
                ++ show (length (salt p))
    | not (length (personal p) `elem` [0, 16]) =
        error $
            "personal must be exactly 16 bytes (or empty), got "
                ++ show (length (personal p))
    | otherwise = p

-- ---------------------------------------------------------------------
-- Initial state: IVs XORed with the 64-byte parameter block.
--
-- The parameter block layout (RFC 7693 section 2.5):
--
--   byte offset   field             size (bytes)
--   0             digest_length     1
--   1             key_length        1
--   2             fanout            1   (sequential: 1)
--   3             depth             1   (sequential: 1)
--   4-7           leaf_length       4   (sequential: 0)
--   8-15          node_offset       8   (sequential: 0)
--   16            node_depth        1   (sequential: 0)
--   17            inner_length      1   (sequential: 0)
--   18-31         reserved          14
--   32-47         salt              16
--   48-63         personal          16
--
-- If keyed, the key (zero-padded to 128 bytes) is absorbed as the first
-- input block.  We return it here as @firstBuffer@ so the caller can
-- prepend it to the user's message.
-- ---------------------------------------------------------------------
initHashing :: Params -> ([Word64], Int, [Word8])
initHashing Params {digestSize = ds, key = k, salt = s, personal = per} =
    (h0, 0, firstBuffer)
  where
    parameterBlock =
        [ fromIntegral ds
        , fromIntegral (length k)
        , 1 -- fanout
        , 1 -- depth
        ]
            ++ replicate 28 0 -- leaf_length, node_offset, node_depth, inner_length, reserved
            ++ pad16 s
            ++ pad16 per
    -- Normalise empty salt/personal to 16 zero bytes.
    pad16 x
        | length x == 16 = x
        | otherwise = replicate 16 0
    paramWords = map leBytesToWord64 (chunksOf 8 parameterBlock)
    h0 = zipWith xor iv paramWords

    firstBuffer
        | null k = []
        | otherwise = k ++ replicate (128 - length k) 0

-- ---------------------------------------------------------------------
-- absorb h byteCount allBytes -> (h', leftover, byteCount')
--
-- Compress every full 128-byte block EXCEPT potentially the very last
-- one -- we must keep at least one byte around so the caller can run a
-- final-flagged compression.  Equivalently: flush only when the remaining
-- buffer STRICTLY exceeds BLOCK_SIZE.  Mirrors the canonical BLAKE2
-- off-by-one rule applied in every sibling port.
-- ---------------------------------------------------------------------
absorb :: [Word64] -> Int -> [Word8] -> ([Word64], [Word8], Int)
absorb h byteCount bytes
    | length bytes <= 128 = (h, bytes, byteCount)
    | otherwise =
        let (block, rest) = splitAt 128 bytes
            byteCount' = byteCount + 128
            h' = compressBlock h block byteCount' False
         in absorb h' byteCount' rest

pad128 :: [Word8] -> [Word8]
pad128 block = block ++ replicate (128 - length block) 0

-- ---------------------------------------------------------------------
-- IV: first 64 bits of the fractional parts of sqrt(first 8 primes).
-- Identical to SHA-512 -- a deliberate "nothing up my sleeve" choice.
-- ---------------------------------------------------------------------
iv :: [Word64]
iv =
    [ 0x6A09E667F3BCC908
    , 0xBB67AE8584CAA73B
    , 0x3C6EF372FE94F82B
    , 0xA54FF53A5F1D36F1
    , 0x510E527FADE682D1
    , 0x9B05688C2B3E6C1F
    , 0x1F83D9ABFB41BD6B
    , 0x5BE0CD19137E2179
    ]

-- ---------------------------------------------------------------------
-- Message-schedule permutations SIGMA[0..9].  Round i uses SIGMA[i mod 10].
-- Twelve rounds total, so rounds 10 and 11 reuse rows 0 and 1.
-- ---------------------------------------------------------------------
sigma :: [[Int]]
sigma =
    [ [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    , [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3]
    , [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4]
    , [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8]
    , [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13]
    , [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9]
    , [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11]
    , [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10]
    , [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5]
    , [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0]
    ]

-- ---------------------------------------------------------------------
-- compressBlock h block counter final -> h'
--
-- BLAKE2b's F function: absorb one 128-byte block into the 8-word state.
-- @counter@ is the total byte count through (and including) this block;
-- @final@ is 'True' only on the last call.
-- ---------------------------------------------------------------------
compressBlock :: [Word64] -> [Word8] -> Int -> Bool -> [Word64]
compressBlock h block counter final =
    -- Davies-Meyer feed-forward: XOR both halves of v back into h.
    zipWith3 xor3 h (take 8 v12) (drop 8 v12)
  where
    xor3 a b c = a `xor` b `xor` c

    -- Parse 16 LE 64-bit words from the 128-byte block.
    m = map leBytesToWord64 (chunksOf 8 block)

    -- Initial working vector: state || IV, with counter folded into
    -- v[12..13] and v[14] inverted on final blocks.  We represent the
    -- 128-bit counter as a single 'Word64'; the spec's reserved high
    -- 64 bits are always zero for any practical message (< 2^64 bytes
    -- = 16 EB).  XOR-ing 0 into v[13] / v[15] is a no-op but keeps the
    -- positional layout symmetric.
    counterXor =
        replicate 12 0
            ++ [fromIntegral counter .&. maxBound64, 0] -- v[12], v[13]
            ++ [if final then maxBound64 else 0, 0] -- v[14], v[15]
    vInitial = zipWith xor (h ++ iv) counterXor

    maxBound64 :: Word64
    maxBound64 = 0xFFFFFFFFFFFFFFFF

    -- Run twelve rounds over vInitial.
    v12 = foldl' doRound vInitial [0 .. 11]

    doRound vec i =
        let s = sigma !! (i `mod` 10)
            -- Columns
            v1 = gStep 0 4 8 12 (s !! 0) (s !! 1) vec
            v2 = gStep 1 5 9 13 (s !! 2) (s !! 3) v1
            v3 = gStep 2 6 10 14 (s !! 4) (s !! 5) v2
            v4 = gStep 3 7 11 15 (s !! 6) (s !! 7) v3
            -- Diagonals
            v5 = gStep 0 5 10 15 (s !! 8) (s !! 9) v4
            v6 = gStep 1 6 11 12 (s !! 10) (s !! 11) v5
            v7 = gStep 2 7 8 13 (s !! 12) (s !! 13) v6
            v8 = gStep 3 4 9 14 (s !! 14) (s !! 15) v7
         in v8

    gStep a b c d mx my vec =
        let (va', vb', vc', vd') =
                g (vec !! a) (vec !! b) (vec !! c) (vec !! d) (m !! mx) (m !! my)
         in replaceMany [(a, va'), (b, vb'), (c, vc'), (d, vd')] vec

-- ---------------------------------------------------------------------
-- The BLAKE2b quarter-round.  Rotation constants (32, 24, 16, 63) are
-- from RFC 7693 Appendix D; changing any one breaks compatibility with
-- every BLAKE2b implementation on earth.
-- ---------------------------------------------------------------------
g :: Word64 -> Word64 -> Word64 -> Word64 -> Word64 -> Word64 -> (Word64, Word64, Word64, Word64)
g va vb vc vd mx my =
    let va1 = va + vb + mx
        vd1 = rotateR (vd `xor` va1) 32
        vc1 = vc + vd1
        vb1 = rotateR (vb `xor` vc1) 24
        va2 = va1 + vb1 + my
        vd2 = rotateR (vd1 `xor` va2) 16
        vc2 = vc1 + vd2
        vb2 = rotateR (vb1 `xor` vc2) 63
     in (va2, vb2, vc2, vd2)

-- ---------------------------------------------------------------------
-- Small helpers: replace one or many list positions, chunk a list, and
-- little-endian Word64 marshalling.  None of these are performance-
-- critical because the KAT inputs top out at 10 KiB.
-- ---------------------------------------------------------------------
replaceMany :: [(Int, a)] -> [a] -> [a]
replaceMany updates values =
    [ maybe x id (lookup i updates)
    | (i, x) <- zip [0 ..] values
    ]

chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf n xs =
    let (h, t) = splitAt n xs
     in h : chunksOf n t

leBytesToWord64 :: [Word8] -> Word64
leBytesToWord64 [b0, b1, b2, b3, b4, b5, b6, b7] =
    fromIntegral b0
        .|. (fromIntegral b1 `shiftL` 8)
        .|. (fromIntegral b2 `shiftL` 16)
        .|. (fromIntegral b3 `shiftL` 24)
        .|. (fromIntegral b4 `shiftL` 32)
        .|. (fromIntegral b5 `shiftL` 40)
        .|. (fromIntegral b6 `shiftL` 48)
        .|. (fromIntegral b7 `shiftL` 56)
leBytesToWord64 _ = 0

word64ToLeBytes :: Word64 -> [Word8]
word64ToLeBytes w =
    [ fromIntegral (w .&. 0xFF)
    , fromIntegral ((w `shiftR` 8) .&. 0xFF)
    , fromIntegral ((w `shiftR` 16) .&. 0xFF)
    , fromIntegral ((w `shiftR` 24) .&. 0xFF)
    , fromIntegral ((w `shiftR` 32) .&. 0xFF)
    , fromIntegral ((w `shiftR` 40) .&. 0xFF)
    , fromIntegral ((w `shiftR` 48) .&. 0xFF)
    , fromIntegral ((w `shiftR` 56) .&. 0xFF)
    ]
