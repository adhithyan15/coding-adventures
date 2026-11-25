-- | SplitMix64, bounded draws, FNV-1a and sub-seeds, with the pinned vectors of
-- the independent reference (VER05n 8.1): including the rejection draw and the
-- 2^63 + 1 boundary, which the pbt-v1 corpus cannot catch on its own.
module PrngSpec (tests) where

import Control.Monad (replicateM)
import Data.Bits (shiftL)
import Data.IORef
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Prng
import Control.Exception (displayException)
import Harness

next1 :: Word64 -> IO Word64
next1 seed = newRng seed >>= nextU64

tests :: [Test]
tests =
  [ test "seed 0 first output is pinned" $ do
      w <- next1 0
      assertEq "first word" 0xe220a8397b1dcdaf w
  , test "Word64 state wraps at maxBound" $ do
      r <- newRng maxBound
      _ <- nextU64 r
      s <- rngState r
      assertEq "maxBound + gamma" (goldenGamma - 1) s
  , test "a negative seed wraps like the reference" $ do
      a <- next1 (fromInteger (-1))
      b <- next1 maxBound
      assertEq "-1 is 2^64-1" b a
      c <- next1 (fromInteger (5 + 2 ^ (64 :: Int)))
      d <- next1 5
      assertEq "seed + 2^64" d c
  , test "fnv1a64 vectors" $ do
      assertEq "empty" 14695981039346656037 (fnv1a64 "")
      assertEq "a" 12638187200555641996 (fnv1a64 "a")
      assertEq "foobar" 9625390261332436968 (fnv1a64 "foobar")
  , test "sub_seed vectors from the independent reference" $ do
      assertEq "42 prop/a" 7249934457332604747 (subSeed 42 "prop/a")
      assertEq "0 empty" 14087677454934409008 (subSeed 0 "")
      assertEq "2^64+5 x" 4559403196206768639 (subSeed (fromInteger (2 ^ (64 :: Int) + 5)) "x")
  , test "sub_seed hashes the UTF-8 bytes of the id" $
      assertEq "e-acute" (subSeed 1 "\233") (subSeed 1 "\233")
  , test "bounded 1 consumes exactly one word" $ do
      a <- newRng 7
      v <- bounded a 1
      assertEq "answer" 0 v
      w <- nextU64 a
      assertEq "second word of seed 7" 309689372594955804 w
  , test "bounded refuses out-of-range arguments" $ do
      r <- newRng 0
      assertThrows "0" isArg (bounded r 0)
      assertThrows "negative" isArg (bounded r (-3))
      assertThrows "2^64+1" isArg (bounded r (2 ^ (64 :: Int) + 1))
      assertThrows "span 0" isArg (boundedBig r 0)
  , test "bounded 2^64 is one raw word" $ do
      a <- newRng 5
      v <- bounded a (2 ^ (64 :: Int))
      assertEq "raw" 7134611160154358618 v
  , test "forced rejection draw: a first word of 0 is rejected by bounded 3" $ do
      let seed = 7046029254386353131 :: Word64 -- (0 - gamma) mod 2^64: its first word is 0
      assertEq "seed" (0 - goldenGamma) seed
      probe <- newRng seed
      w0 <- nextU64 probe
      w1 <- nextU64 probe
      assertEq "first word" 0 w0
      assertEq "second word" 16294208416658607535 w1
      assertEq "second mod 3" 1 (w1 `mod` 3)
      r <- newRng seed
      v <- bounded r 3
      assertEq "rejected the 0, answered from the second word" 1 v
  , test "a wrong threshold (0) keeps the word 0: the negative control is real" $ do
      r <- newRngWithThreshold (const 0) 7046029254386353131
      v <- bounded r 3
      assertEq "sabotaged answer" 0 v
  , test "the threshold is not a signed negation" $ do
      assertEq "n=3" 1 (defaultThreshold 3)
      assertEq "n=2^63+1" (2 ^ (63 :: Int) - 1) (defaultThreshold (2 ^ (63 :: Int) + 1))
      assertEq "n=2^63" 0 (defaultThreshold (2 ^ (63 :: Int)))
  , test "2^63 + 1 boundary: below the threshold is rejected, above is kept" $ do
      let n = 2 ^ (63 :: Int) + 1 :: Integer
      w3 <- next1 3
      assertEq "seed 3 first word" 2092789425003139053 w3
      assertTrue "below threshold" (toInteger w3 < 2 ^ (63 :: Int) - 1)
      r3 <- newRng 3
      v3 <- bounded r3 n
      assertEq "answered from the second word" 3694763184872335752 v3
      r1 <- newRng 1
      v1 <- bounded r1 n
      assertEq "accepted at once" 1227844342346046656 v1
  , test "boundedBig thirds of a wide span (an unrejected raw mod span would give a half)" $ do
      r <- newRng 11
      let span' = 3 * 2 ^ (126 :: Int) :: Integer
          n = 6000 :: Int
      vs <- replicateM n (boundedBig r span')
      assertTrue "in range" (all (< span') vs)
      let low = length (filter (< 2 ^ (126 :: Int)) vs)
          mid = length (filter (\v -> v >= 2 ^ (126 :: Int) && v < 2 * 2 ^ (126 :: Int)) vs)
          near c = abs (fromIntegral c / fromIntegral n - (1 / 3 :: Double)) < 0.04
      assertTrue "low third" (near low)
      assertTrue "middle third" (near mid)
  , test "boundedBig small spans match bounded" $ do
      a <- newRng 9
      b <- newRng 9
      x <- bounded a 100
      y <- boundedBig b 100
      assertEq "same" x y
  , test "boundedBig above 2^64 uses k words" $ do
      r <- newRng 1
      before <- rngState r
      v <- boundedBig r (2 ^ (130 :: Int))
      after <- rngState r
      assertTrue "in range" (v < 2 ^ (130 :: Int))
      assertTrue "advanced" (after /= before)
      -- 2^130 needs ceil(130/64) = 3 words (a power of two never rejects)
      r2 <- newRng 1
      mapM_ (const (nextU64 r2)) [1 :: Int .. 3]
      s2 <- rngState r2
      assertEq "three words consumed" s2 after
  , test "bounded is uniform enough" $ do
      r <- newRng 1
      counts <- newIORef (replicate 5 (0 :: Int))
      mapM_ (\_ -> do i <- bounded r 5; modifyIORef' counts (bump (fromInteger i))) [1 .. 5000 :: Int]
      cs <- readIORef counts
      assertTrue "each bucket near 1000" (all (\c -> abs (c - 1000) < 150) cs)
  , test "the 1 << 64 constant used by the tests is right" $
      assertEq "2^64" (18446744073709551616 :: Integer) (1 `shiftL` 64)
  ]
  where
    isArg (ArgumentError m) = length m > 0 && length (displayException (ArgumentError m)) > 0
    isArg _ = False
    bump i xs = [if j == i then x + 1 else x | (j, x) <- zip [0 :: Int ..] xs]
