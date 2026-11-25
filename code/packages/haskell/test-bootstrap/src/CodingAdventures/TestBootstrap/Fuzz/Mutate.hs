-- | The mutator algebra (VER06 3, VER06o 3).
--
-- Every mutator takes a strict 'B.ByteString' and returns a NEW one (a
-- ByteString is immutable, so "never modifies its argument" is free).  Haskell
-- facts that shape the code:
--
-- * offsets, counts and deltas are 'Integer' (the Go oracle feeds 30-digit
--   values), and every wrap is a FLOORED 'mod' (Haskell's 'mod' floors, so
--   @(-1) \`mod\` 8 == 7@; 'rem' truncates and is never used for a wrap);
-- * a flip never builds an 'Integer' from the input bytes.  The Elixir lane
--   read the whole buffer as one integer and crashed on a 2^(8 x 1 MiB) value.
--   Here inversion is 'B.map' 'complement', a range flip masks the two edge
--   bytes and complements the middle: word-wise, O(len), for ANY count.
module CodingAdventures.TestBootstrap.Fuzz.Mutate
  ( Endian (..)
  , parseEndian
  , interest8
  , interest16
  , interest32
  , interest64
  , interestByWidth
  , deltas
  , bitFlip
  , byteFlip
  , byteArithmetic
  , boundaryInsert
  , blockDelete
  , blockInsert
  , blockDuplicate
  , splice
  , SeqResult (..)
  , mutateSequenceCounted
  , mutateSequence
  , mutateSequenceWithCap
  , WorkCapExceeded (..)
  ) where

import Control.Exception (Exception, throwIO)
import Control.Monad (when)
import Data.Bits (complement, shiftL, xor, (.&.))
import qualified Data.ByteString as B
import Data.Word (Word64, Word8)

import CodingAdventures.TestBootstrap.Error (argError)
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Prng (Rng, bounded, newRng)

data Endian = Little | Big deriving (Eq, Show)

-- | @"little"@ and @"big"@ only.
parseEndian :: String -> Maybe Endian
parseEndian "little" = Just Little
parseEndian "big" = Just Big
parseEndian _ = Nothing

interest8, interest16, interest32, interest64, deltas :: [Integer]
interest8 = [0, 1, 127, 128, 255]
interest16 = [0, 1, 255, 256, 32767, 32768, 65535]
interest32 = [0, 1, 65535, 65536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF]
interest64 = [0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF]
deltas = [-35, -8, -1, 1, 8, 35]

interestByWidth :: [(Int, [Integer])]
interestByWidth = [(1, interest8), (2, interest16), (4, interest32), (8, interest64)]

-- ---------------------------------------------------------------------------
-- Flips
-- ---------------------------------------------------------------------------

-- | Inverts @count@ consecutive units (bits when @unit == 1@, bytes when
-- @unit == 8@) from @offset mod units@, wrapping around the buffer.  Only the
-- PARITY of each unit's flips matters (XOR twice is a no-op): every unit is hit
-- @count div units@ times, and the first @count mod units@ units from the
-- wrapped start once more, so the work is O(len) for any count, where a loop of
-- @count@ single flips would never end.
--
-- Bit @i@ is bit @i mod 8@ of byte @i div 8@, LSB first (VER06 3.1).
--
-- Worked example, 2 bytes = 16 bits, @bitFlip 0 17@: 17 div 16 = 1 (every bit
-- once) and 17 mod 16 = 1 (bit 0 once more, so twice, even): bits 1..15 end up
-- inverted and bit 0 unchanged.
parityFlip :: Integer -> B.ByteString -> Integer -> Integer -> B.ByteString
parityFlip unit raw offset count'
  | size == 0 || count' <= 0 = raw
  | otherwise =
      let units = total `div` unit
          start = fromInteger ((offset `mod` units) * unit) :: Int
          full = count' `div` units
          len = fromInteger ((count' `mod` units) * unit) :: Int
          base = if odd full then B.map complement raw else raw
       in if len == 0
            then base
            else
              if start + len <= totalI
                then flipRange base start (start + len)
                else flipRange (flipRange base start totalI) 0 (start + len - totalI)
  where
    size = B.length raw
    totalI = size * 8
    total = toInteger totalI

-- | Inverts bits @[a, b)@ of the buffer (@b > a@): the two edge bytes get a
-- mask, every byte between is complemented whole.
flipRange :: B.ByteString -> Int -> Int -> B.ByteString
flipRange raw a b
  | first == lastB = B.concat [B.take first raw, B.singleton (B.index raw first `xor` (firstMask .&. lastMask)), B.drop (first + 1) raw]
  | otherwise =
      B.concat
        [ B.take first raw
        , B.singleton (B.index raw first `xor` firstMask)
        , B.map complement (B.take (lastB - first - 1) (B.drop (first + 1) raw))
        , B.singleton (B.index raw lastB `xor` lastMask)
        , B.drop (lastB + 1) raw
        ]
  where
    first = a `div` 8
    lastB = (b - 1) `div` 8
    firstMask = fromIntegral ((0xFF :: Int) `shiftL` (a `mod` 8)) :: Word8
    lastMask = fromIntegral (((1 :: Int) `shiftL` ((b - 1) `mod` 8 + 1)) - 1) :: Word8

-- | Invert @count@ consecutive bits from @bitOffset mod (8 * len)@:
-- @bitFlip "\\0\\0" 9 1@ is @"\\0\\2"@.
bitFlip :: B.ByteString -> Integer -> Integer -> B.ByteString
bitFlip = \d o c -> parityFlip 1 d o c

-- | XOR @count@ consecutive bytes with 0xFF from @byteOffset mod len@.
byteFlip :: B.ByteString -> Integer -> Integer -> B.ByteString
byteFlip = \d o c -> parityFlip 8 d o c

-- ---------------------------------------------------------------------------
-- Arithmetic and boundary values
-- ---------------------------------------------------------------------------

widthOk :: Integer -> Int -> Bool
widthOk w size = w `elem` [1, 2, 4, 8] && size >= fromInteger w

-- | Reads at most 8 bytes: a 'Word64' is wide enough, so no big integer is
-- ever built from input bytes.
readWord :: Endian -> B.ByteString -> Word64
readWord Little = B.foldr' (\b acc -> acc * 256 + fromIntegral b) 0
readWord Big = B.foldl' (\acc b -> acc * 256 + fromIntegral b) 0

writeWord :: Endian -> Int -> Integer -> B.ByteString
writeWord endian w v =
  let le = [fromInteger ((v `div` (256 ^ i)) `mod` 256) | i <- [0 .. w - 1]] :: [Word8]
   in B.pack (if endian == Little then le else reverse le)

-- | Add signed @delta@ modulo @2^(8 width)@ to the integer at the offset.  u16
-- little-endian: @ff 00@ is 0x00ff; +1 gives 0x0100, stored @00 01@.
byteArithmetic :: B.ByteString -> Integer -> Integer -> Integer -> Endian -> B.ByteString
byteArithmetic raw byteOffset delta width endian
  | widthOk width size =
      let w = fromInteger width :: Int
          at = fromInteger (byteOffset `mod` toInteger (size - w + 1)) :: Int
          cur = readWord endian (B.take w (B.drop at raw))
          new = (toInteger cur + delta) `mod` (2 ^ (8 * w) :: Integer)
       in B.concat [B.take at raw, writeWord endian w new, B.drop (at + w) raw]
  | otherwise = raw
  where
    size = B.length raw

-- | Overwrite @width@ bytes with @val mod 2^(8 width)@:
-- @boundaryInsert "\\0\\0\\0\\0" 0 4 0x80000000 Little@ is @"\\0\\0\\0\\x80"@.
boundaryInsert :: B.ByteString -> Integer -> Integer -> Integer -> Endian -> B.ByteString
boundaryInsert raw byteOffset width val endian
  | widthOk width size =
      let w = fromInteger width :: Int
          at = fromInteger (byteOffset `mod` toInteger (size - w + 1)) :: Int
       in B.concat [B.take at raw, writeWord endian w (val `mod` (2 ^ (8 * w) :: Integer)), B.drop (at + w) raw]
  | otherwise = raw
  where
    size = B.length raw

-- ---------------------------------------------------------------------------
-- Blocks and splice
-- ---------------------------------------------------------------------------

-- | Delete up to @len@ bytes at @offset mod size@; clamped at the end.
blockDelete :: B.ByteString -> Integer -> Integer -> B.ByteString
blockDelete raw offset len
  | size == 0 || len <= 0 = raw
  | otherwise =
      let at = fromInteger (offset `mod` toInteger size) :: Int
          cut = fromInteger (min len (toInteger (size - at))) :: Int
       in B.take at raw <> B.drop (at + cut) raw
  where
    size = B.length raw

-- | Insert at @offset mod (size + 1)@; an empty input becomes the insert.
blockInsert :: B.ByteString -> Integer -> B.ByteString -> B.ByteString
blockInsert raw offset ins
  | size == 0 = ins
  | otherwise =
      let at = fromInteger (offset `mod` toInteger (size + 1)) :: Int
       in B.take at raw <> ins <> B.drop at raw
  where
    size = B.length raw

-- | Copy up to @len@ bytes from @src mod size@ to @dst mod (size + 1)@ (the end
-- is a valid insertion point).
blockDuplicate :: B.ByteString -> Integer -> Integer -> Integer -> B.ByteString
blockDuplicate raw srcOffset len dstOffset
  | size == 0 || len <= 0 = raw
  | otherwise =
      let src = fromInteger (srcOffset `mod` toInteger size) :: Int
          chunk = B.take (fromInteger (min len (toInteger (size - src)))) (B.drop src raw)
          dst = fromInteger (dstOffset `mod` toInteger (size + 1)) :: Int
       in B.take dst raw <> chunk <> B.drop dst raw
  where
    size = B.length raw

-- | @target[0..ts) <> donor[ds..]@ with both splits clamped into their buffers.
splice :: B.ByteString -> B.ByteString -> Integer -> Integer -> B.ByteString
splice target donor targetSplit donorSplit =
  let t = clampTo (B.length target) targetSplit
      d = clampTo (B.length donor) donorSplit
   in B.take t target <> B.drop d donor
  where
    clampTo size n = fromInteger (max 0 (min (toInteger size) n)) :: Int

-- ---------------------------------------------------------------------------
-- Sequences
-- ---------------------------------------------------------------------------

-- | The result of a counted sequence: the bytes and the work, or the work done
-- when the cap would have been crossed.
data SeqResult = WorkOk !B.ByteString !Int | WorkExhausted !Int
  deriving (Eq, Show)

-- | Thrown by 'mutateSequence' when the per-call work cap is crossed.
data WorkCapExceeded = WorkCapExceeded {wceWork :: !Int, wceCap :: !Int}
  deriving (Eq, Show)

instance Exception WorkCapExceeded

-- | @steps@ seeded mutations, each chosen by one @bounded 7@ draw:
--
-- > 0 bit_flip, count 1            3 boundary_insert, width 1, interest8
-- > 1 byte_flip, count 1           4 block_delete (len 1..n/2), skipped if n <= 1
-- > 2 byte_arithmetic, deltas      5 block_insert of one random byte
-- >                                6 block_duplicate (len 1..n/2), skipped if n <= 1
--
-- An empty buffer is refilled to @00@ before every step.  The draw order is the
-- cross-lane contract.  @maxLen@ is VER06 3.5's length budget: an insert or
-- duplicate that would pass it is skipped AFTER its draws, so the stream stays
-- aligned and memory stays bounded.
--
-- Work is the bytes copied: the initial copy plus one buffer per step.  A call
-- stops AT the step that would pass @cap@ and reports what it had done (the
-- caller still charges a failed call).  @steps@ over 'maxIterations' and inputs
-- over 'maxInputBytes' are an 'ArgumentError', raised before anything is copied.
mutateSequenceCounted :: B.ByteString -> Integer -> Integer -> Int -> Int -> IO SeqResult
mutateSequenceCounted dat seed steps maxLen cap = do
  when (steps > toInteger maxIterations) (argError ("steps exceeds the maximum (" ++ show maxIterations ++ ")"))
  when (B.length dat > maxInputBytes) (argError ("input exceeds " ++ show maxInputBytes ++ " bytes"))
  let work0 = B.length dat
  if work0 > cap
    then pure (WorkExhausted 0)
    else do
      rng <- newRng (fromInteger seed)
      loop rng dat (max 0 steps) work0
  where
    loop :: Rng -> B.ByteString -> Integer -> Int -> IO SeqResult
    loop _ cur 0 work = pure (WorkOk cur work)
    loop rng cur left work
      | B.null cur = loop rng (B.singleton 0) left work
      | work + n > cap = pure (WorkExhausted work)
      | otherwise = do
          cur' <- stepOnce rng cur n
          loop rng cur' (left - 1) (work + n)
      where
        n = B.length cur
    stepOnce :: Rng -> B.ByteString -> Int -> IO B.ByteString
    stepOnce rng cur n = do
      kind <- bounded rng 7
      let nI = toInteger n
      case kind of
        0 -> do
          i <- bounded rng (nI * 8)
          pure (bitFlip cur i 1)
        1 -> do
          i <- bounded rng nI
          pure (byteFlip cur i 1)
        2 -> do
          di <- bounded rng (toInteger (length deltas))
          off <- bounded rng nI
          pure (byteArithmetic cur off (deltas !! fromInteger di) 1 Little)
        3 -> do
          vi <- bounded rng (toInteger (length interest8))
          off <- bounded rng nI
          pure (boundaryInsert cur off 1 (interest8 !! fromInteger vi) Little)
        4 | n > 1 -> do
          l <- bounded rng (max 1 (nI `div` 2))
          off <- bounded rng nI
          pure (blockDelete cur off (1 + l))
        5 -> do
          b <- bounded rng 256
          at <- bounded rng (nI + 1)
          pure (if n + 1 <= maxLen then blockInsert cur at (B.singleton (fromInteger b)) else cur)
        6 | n > 1 -> do
          l <- bounded rng (max 1 (nI `div` 2))
          src <- bounded rng nI
          dst <- bounded rng (nI + 1)
          let len = 1 + l
          pure (if toInteger n + min len (nI - src) <= toInteger maxLen then blockDuplicate cur src len dst else cur)
        _ -> pure cur

-- | 'mutateSequenceCounted' with the per-call cap, throwing 'WorkCapExceeded' on
-- a breach.
mutateSequence :: B.ByteString -> Integer -> Integer -> Int -> IO B.ByteString
mutateSequence = mutateSequenceWithCap maxSequenceWork

-- | 'mutateSequence' with an explicit per-call work cap (a test seam: the real
-- cap is 2^28, which a shrinking buffer rarely reaches).
mutateSequenceWithCap :: Int -> B.ByteString -> Integer -> Integer -> Int -> IO B.ByteString
mutateSequenceWithCap cap dat seed steps maxLen = do
  r <- mutateSequenceCounted dat seed steps maxLen cap
  case r of
    WorkOk out _ -> pure out
    WorkExhausted w -> throwIO (WorkCapExceeded w cap)
