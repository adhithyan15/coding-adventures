-- | Deterministic randomness and hashing for test runs (TF01 5.1, VER05 2).
--
-- SplitMix64 and FNV-1a are defined on 64-bit words that WRAP.  'Word64'
-- wraps natively in Haskell (unlike Ruby's unbounded integers, which need a
-- mask after every operation), so none of the arithmetic below needs one.
--
-- The stream is a mutable 'IORef' (a generator draws thousands of words, and
-- threading a pure state through every sampler would buy nothing here): a
-- 'Rng' must not be shared between threads.
module CodingAdventures.TestBootstrap.Prng
  ( Rng
  , newRng
  , newRngWithThreshold
  , rngState
  , nextU64
  , bounded
  , boundedBig
  , defaultThreshold
  , fnv1a64
  , subSeed
  , goldenGamma
  , mix
  ) where

import Control.Monad (when)
import Data.Bits (shiftL, shiftR, xor)
import qualified Data.ByteString as B
import Data.IORef
import Data.Text (Text)
import qualified Data.Text.Encoding as TE
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Util (integerBitLength)

-- | A SplitMix64 stream.  The second field is the rejection-threshold
-- function of 'bounded'; production code always uses 'defaultThreshold', the
-- field exists so a negative control can install a wrong one.
data Rng = Rng !(IORef Word64) (Word64 -> Word64)

goldenGamma :: Word64
goldenGamma = 0x9e3779b97f4a7c15

-- | The SplitMix64 output finalizer (a bijection on 64-bit words):
--
-- > z1  = (z  xor (z  >> 30)) * 0xbf58476d1ce4e5b9
-- > z2  = (z1 xor (z1 >> 27)) * 0x94d049bb133111eb
-- > out =  z2 xor (z2 >> 31)
mix :: Word64 -> Word64
mix z0 =
  let z1 = (z0 `xor` (z0 `shiftR` 30)) * 0xbf58476d1ce4e5b9
      z2 = (z1 `xor` (z1 `shiftR` 27)) * 0x94d049bb133111eb
   in z2 `xor` (z2 `shiftR` 31)

newRng :: Word64 -> IO Rng
newRng = newRngWithThreshold defaultThreshold

newRngWithThreshold :: (Word64 -> Word64) -> Word64 -> IO Rng
newRngWithThreshold thr seed = do
  ref <- newIORef seed
  pure (Rng ref thr)

-- | The raw state (tests only; it IS the stream, never printed by the engine).
rngState :: Rng -> IO Word64
rngState (Rng ref _) = readIORef ref

-- | state += golden gamma; output = mix state.  Seed 0's first output is
-- 0xe220a8397b1dcdaf.
nextU64 :: Rng -> IO Word64
nextU64 (Rng ref _) = do
  s <- readIORef ref
  let !s' = s + goldenGamma
  writeIORef ref s'
  pure $! mix s'

-- | The rejection threshold @t = (2^64 - n) mod n@.
--
-- Unsigned negation wraps modulo 2^64, so @negate n@ IS @2^64 - n@ and the
-- spelling is exact (the same trick as C's @(0 - n) % n@).  Spelled on a
-- signed or unbounded integer it would be a negative number and give the
-- wrong answer for @n > 2^63@ (pinned by a test at @n = 2^63 + 1@).
--
-- Worked example, n = 3: 2^64 = 3 * 6148914691236517205 + 1, so t = 1 and the
-- single raw word 0 is thrown away.
defaultThreshold :: Word64 -> Word64
defaultThreshold n = negate n `mod` n

-- | Uniform integer in @[0, n)@ for @1 <= n <= 2^64@, by rejection sampling:
-- raws below the threshold are redrawn so the remaining @2^64 - t@ values
-- split into whole residue classes.  @bounded 1@ still consumes one word (the
-- VER05 2.2 loop run once): the stream position must be identical in every
-- lane.
bounded :: Rng -> Integer -> IO Integer
bounded rng@(Rng _ thr) n = do
  when (n < 1) (argError ("bounded(n) requires n >= 1, got " ++ show n))
  when (n > two64) (argError "bounded(n) requires n <= 2^64")
  if n == two64
    then toInteger <$> nextU64 rng
    else do
      let n' = fromInteger n :: Word64
          t = thr n'
          loop = do
            x <- nextU64 rng
            if x >= t then pure (toInteger (x `mod` n')) else loop
      loop

two64 :: Integer
two64 = 1 `shiftL` 64

-- | Uniform integer in @[0, span)@ for any @span >= 1@ (VER05 7.2):
--
-- > span <= 2^64   bounded span
-- > span  > 2^64   k-word rejection, k = max 2 (ceil (bitlen (span-1) / 64)):
-- >                raw = w1 << 64(k-1) | ... | wk   (first draw most significant)
-- >                t   = (2^(64k) - span) mod span; redraw while raw < t
boundedBig :: Rng -> Integer -> IO Integer
boundedBig rng sp = do
  when (sp < 1) (argError ("span must be >= 1, got " ++ show sp))
  if sp <= two64
    then bounded rng sp
    else do
      let k = max 2 ((integerBitLength (sp - 1) + 63) `div` 64)
          t = ((1 `shiftL` (64 * k)) - sp) `mod` sp
          draw :: Int -> Integer -> IO Integer
          draw 0 acc = pure acc
          draw i acc = do
            w <- nextU64 rng
            draw (i - 1) ((acc `shiftL` 64) + toInteger w)
          loop = do
            raw <- draw k 0
            if raw >= t then pure (raw `mod` sp) else loop
      loop

-- | 64-bit FNV-1a of the bytes (offset 0xcbf29ce484222325, prime 0x100000001b3).
fnv1a64 :: B.ByteString -> Word64
fnv1a64 = B.foldl' step 0xcbf29ce484222325
  where
    step h b = (h `xor` fromIntegral b) * 0x100000001b3

-- | @sub_seed(run_seed, id) = SplitMix64(run_seed xor fnv1a64(utf8 id)).next_u64@
-- (VER05 2.3): reordering or adding unrelated properties never changes the
-- candidate sequence of an unchanged id.
subSeed :: Word64 -> Text -> Word64
subSeed runSeed pid = mix ((runSeed `xor` fnv1a64 (TE.encodeUtf8 pid)) + goldenGamma)
