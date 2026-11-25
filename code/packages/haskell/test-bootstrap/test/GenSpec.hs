-- | The generator algebra: draw order, ranges, shrinking attached to
-- generators, and fail-closed invalid constructors.
module GenSpec (tests) where

import Control.Exception (SomeException, displayException, try)
import Control.Monad (replicateM)
import Data.IORef
import qualified Data.ByteString as B
import Data.List (nub, sort)
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Pbt.Gen
import CodingAdventures.TestBootstrap.Pbt.Types
import CodingAdventures.TestBootstrap.Prng
import Harness

draws :: Gen a -> Int -> Word64 -> IO [a]
draws g n seed = do
  r <- newRng seed
  replicateM n (sample Nothing g r)

shr :: Gen a -> a -> (a -> Bool) -> Int -> IO (a, Int)
shr g v falsifies = shrinkGen defaultConfig Nothing g v (pure . falsifies)

isArg :: PbtError -> Bool
isArg (ArgumentError m) = length m > 0 && length (displayException (ArgumentError m)) > 0
isArg _ = False

isExh :: PbtError -> Bool
isExh e@(ExhaustedError m) = length m > 0 && length (displayException e) > 0
isExh _ = False

ints :: [Val] -> [Integer]
ints vs = [i | VInt i <- vs]

tests :: [Test]
tests =
  [ test "sampling is deterministic per seed" $ do
      let g = list (int (-50) 50) 0 10
      a <- draws g 20 9
      b <- draws g 20 9
      c <- draws g 20 10
      assertEq "same seed" a b
      assertTrue "different seed" (a /= c)
  , test "primitives stay in range" $ do
      xs <- draws u8 200 1
      assertTrue "u8" (all (\v -> v >= 0 && v <= 255) xs)
      ys <- draws u64 200 1
      assertTrue "u64" (all (\v -> v >= 0 && v < 2 ^ (64 :: Int)) ys)
      zs <- draws i128 200 1
      assertTrue "i128" (all (\v -> v >= negate (2 ^ (127 :: Int)) && v < 2 ^ (127 :: Int)) zs)
      ws <- draws (int (-3) 3) 200 1
      assertEq "int range" [-3 .. 3] (sort (nub ws))
      bs <- draws bool 200 1
      assertEq "both bools" [False, True] (sort (nub bs))
      assertEq "single" [4] . nub =<< draws (int 4 4) 5 1
      assertEq "constant" [7 :: Int] . nub =<< draws (constant 7) 5 1
  , test "i128 reads two words, high first, as two's complement" $ do
      r <- newRng 42
      hi <- nextU64 r
      lo <- nextU64 r
      let v = toInteger hi * 2 ^ (64 :: Int) + toInteger lo
          expected = if v >= 2 ^ (127 :: Int) then v - 2 ^ (128 :: Int) else v
      got <- newRng 42 >>= sample Nothing i128
      assertEq "i128" expected got
      vs <- genPrimitive defaultConfig 42 "i128" 4
      assertEq "four" 4 (length vs)
  , test "wide integer ranges" $ do
      xs <- draws (int (negate (2 ^ (200 :: Int))) (2 ^ (200 :: Int))) 20 1
      assertTrue "within" (all (\v -> abs v <= 2 ^ (200 :: Int)) xs)
  , test "bytes length and bounds" $ do
      xs <- draws (bytes 2 5) 200 1
      assertTrue "lengths" (all (\b -> B.length b >= 2 && B.length b <= 5) xs)
      es <- draws (bytes 0 0) 3 1
      assertEq "empty" [B.empty] (nub es)
  , test "a fixed-length bytes still consumes the length word" $ do
      -- bounded(1) consumes a word, so bytes(4,4) draws 1 + 4 words
      r <- newRng 5
      _ <- sample Nothing (bytes 4 4) r
      expected <- newRng 5
      mapM_ (const (nextU64 expected)) [1 :: Int .. 5]
      a <- rngState r
      b <- rngState expected
      assertEq "stream position" b a
  , test "list lengths are inclusive" $ do
      xs <- draws (list u8 1 3) 300 1
      assertEq "lengths" [1, 2, 3] (sort (nub (map length xs)))
  , test "invalid constructors fail closed: the sampler throws, never a pass" $ do
      let boom :: Gen Integer -> IO ()
          boom g = do
            r <- newRng 1
            assertThrows "invalid" isArg (sample Nothing g r)
      boom (int 5 4)
      boom (int 0 (2 ^ (20000 :: Int)))
      boom (frequency [])
      boom (frequency [(0, u8)])
      boom (frequency [(-1, u8), (2, u8)])
      boom (frequency [(2 ^ (32 :: Int), u8), (1, u8)])
      boom (oneOf [])
      boom (filterG 0 (const True) u8)
      boom (filterG 10001 (const True) u8)
      r <- newRng 1
      assertThrows "bytes min>max" isArg (sample Nothing (bytes 3 2) r)
      assertThrows "bytes too long" isArg (sample Nothing (bytes 0 (maxInputBytes + 1)) r)
      assertThrows "bytes negative" isArg (sample Nothing (bytes (-1) 2) r)
      assertThrows "list min>max" isArg (sample Nothing (list u8 2 1) r)
      assertThrows "list too long" isArg (sample Nothing (list u8 0 (maxListLen + 1)) r)
  , test "map transforms and drops shrinking" $ do
      let g = mapG (* 2) (int 0 9)
      xs <- draws g 100 1
      assertTrue "even" (all even xs)
      r <- shr g 50 (const True) 100
      assertEq "no shrinker" (50, 0) r
  , test "flat_map samples dependently" $ do
      let g = flatMap (\n -> list (constant n) (fromInteger n) (fromInteger n)) (int 1 3)
      xs <- draws g 100 1
      assertTrue "sizes" (all (\l -> not (null l) && length l == fromInteger (head l)) xs)
  , test "filter keeps only satisfying values and shrinks within them" $ do
      let g = filterG 100 even (int 0 1000)
      xs <- draws g 100 1
      assertTrue "even" (all even xs)
      (v, _) <- shr g 900 (>= 11) 1000
      assertTrue "an odd candidate is never accepted" (even v)
      assertTrue "still falsifies" (v >= 11)
  , test "filter exhaustion is raised after exactly max_retries samples" $ do
      calls <- newIORef (0 :: Int)
      let g = filterM 7 (\_ -> modifyIORef' calls (+ 1) >> pure False) (int 0 9)
      r <- newRng 1
      assertThrows "exhausted" isExh (sample Nothing g r)
      n <- readIORef calls
      assertEq "calls" 7 n
  , test "one_of is uniform" $ do
      xs <- draws (oneOf [constant 'a', constant 'b', constant 'c']) 900 1
      let count c = length (filter (== c) xs)
      assertTrue "balanced" (all (\c -> abs (count c - 300) < 70) ['a', 'b', 'c'])
  , test "frequency follows weights and never picks a zero-weight arm" $ do
      xs <- draws (frequency [(1, constant 'a'), (3, constant 'b'), (0, constant 'n')]) 2000 1
      assertFalse "never" ('n' `elem` xs)
      let count c = length (filter (== c) xs)
      assertTrue "a" (abs (count 'a' - 500) < 100)
      assertTrue "b" (abs (count 'b' - 1500) < 100)
      -- the maximum total is accepted
      _ <- draws (frequency [(2 ^ (32 :: Int), constant 'x')]) 3 1
      pure ()
  , test "tuple2 and tuple3 sample in order and shrink each coordinate" $ do
      let g = tuple2 (int 0 1000) (int 0 1000)
      ((a, b), steps) <- shr g (900, 800) (\(x, y) -> x + y >= 100) 1000
      assertEq "sum" 100 (a + b)
      assertTrue "steps" (steps > 0)
      let g3 = tuple3 (int 0 100) (int 0 100) bool
      ((x, y, z), _) <- shr g3 (90, 90, True) (\(p, q, s) -> p * q >= 100 || s) 1000
      assertTrue "still falsifies" (x * y >= 100 || z)
      ts <- draws g3 10 1
      assertEq "ten" 10 (length ts)
  , test "bool shrinks to false only when false fails" $ do
      r1 <- shr bool True (const True) 10
      assertEq "false fails" (False, 1) r1
      r2 <- shr bool True id 10
      assertEq "false holds" (True, 0) r2
      r3 <- shr bool False (const True) 10
      assertEq "already false" (False, 0) r3
      r4 <- shr bool True (const True) 0
      assertEq "no budget" (True, 0) r4
  , test "integers shrink toward the in-range point nearest zero" $ do
      r1 <- shr (int 10 100) 80 (const True) 100
      assertEq "positive range" (10, 1) r1
      r2 <- shr (int (-100) (-10)) (-80) (const True) 100
      assertEq "negative range" (-10, 1) r2
      r3 <- shr (int (-5) 5) 4 (const True) 100
      assertEq "straddling zero" (0, 1) r3
  , test "list shrinking respects min_len and uses the element shrinker" $ do
      (m, _) <- shr (list (int 0 100) 2 6) [50, 60, 70, 80] ((>= 100) . sum) 1000
      assertEq "sum" 100 (sum m)
      assertTrue "min_len" (length m >= 2)
  , test "bytes shrinking respects min_len" $ do
      (m, _) <- shr (bytes 3 8) (B.pack [5, 6, 7, 8, 9]) (const True) 1000
      assertEq "zeros of length min" (B.pack [0, 0, 0]) m
  , test "a generator without a shrinker returns its value" $ do
      let g = customGen (\_ _ -> pure (1 :: Integer)) Nothing
      r <- shr g 5 (const True) 10
      assertEq "unshrunk" (5, 0) r
      assertThrows "bad budget" isArg (shr g 5 (const True) (-1))
  , test "corpus entry points validate" $ do
      assertThrows "unknown type" isArg (genPrimitive defaultConfig 1 "float" 1)
      assertThrows "huge count" isArg (genPrimitive defaultConfig 1 "bool" (maxGenCount + 1))
      assertThrows "negative count" isArg (genPrimitive defaultConfig 1 "bool" (-1))
      assertThrows "lo > hi" isArg (genBounded defaultConfig 1 5 4 1)
      assertThrows "wide span x count" isArg (genBounded defaultConfig 1 0 (2 ^ (4000 :: Int)) 1000000)
      assertThrows "count even with max_len 0" isArg (genBytes defaultConfig 1 0 0 (maxGenCount + 1))
      assertThrows "count x max_len" isArg (genBytes defaultConfig 1 0 100000 100)
      vs <- genBytes defaultConfig 1 0 0 3
      assertEq "three empties" 3 (length vs)
  , test "gen_bounded uses bounded draws" $ do
      vs <- genBounded defaultConfig 7 10 20 8
      assertEq "pinned" [12, 10, 10, 10, 17, 17, 11, 19] (ints vs)
      wide <- genBounded defaultConfig 1 0 (2 ^ (100 :: Int)) 3
      assertTrue "wide" (all (\v -> v >= 0 && v <= 2 ^ (100 :: Int)) (ints wide))
  , test "an exception in the sampler is visible to the caller" $ do
      let g = customGen (\_ _ -> ioError (userError "gen boom")) Nothing :: Gen Integer
      r <- newRng 1
      res <- try (sample Nothing g r)
      case res of
        Left (_ :: SomeException) -> pure ()
        Right _ -> failTest "expected an exception"
  ]
