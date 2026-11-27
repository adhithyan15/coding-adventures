-- | The generator algebra (VER05 3, VER05e 3, VER05n 3).
--
-- DRAW ORDER IS THE COMPATIBILITY CONTRACT between language lanes: the pbt-v1
-- vectors pin exact outputs per seed, so reordering a draw breaks conformance.
-- Nested generators are length-bounded per level, not in total
-- (@list (list g 0 n) 0 n@ can produce n x n elements); shrinking, by contrast,
-- shares one work budget across all levels.
--
-- Values: integers are 'Integer' (u8, u64 and i128 included), bytes are
-- 'B.ByteString', lists are lists, tuples are Haskell pairs and triples.
--
-- /Invalid generators fail closed./  A constructor given invalid arguments
-- returns a generator whose sampler throws 'ArgumentError': a pure constructor
-- must not be a thunk bomb, and the runner reports it as @property-exhausted@,
-- so an invalid generator can never pass.
module CodingAdventures.TestBootstrap.Pbt.Gen
  ( Gen (..)
  , Shrinker
  , sample
  , shrinkGen
  , shrinkNested
  , constant
  , bool
  , u8
  , u64
  , i128
  , int
  , bytes
  , mapG
  , filterG
  , filterM
  , flatMap
  , oneOf
  , frequency
  , list
  , tuple2
  , tuple3
  , customGen
  , genPrimitive
  , genBounded
  , genBytes
  ) where

import Control.Exception (throwIO)
import Control.Monad (replicateM, unless, when)
import Data.Bits (shiftL)
import qualified Data.ByteString as B
import Data.Text (Text)
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Pbt.Budget
import CodingAdventures.TestBootstrap.Pbt.Shrink
import CodingAdventures.TestBootstrap.Pbt.Types
import CodingAdventures.TestBootstrap.Prng
import CodingAdventures.TestBootstrap.Util (integerBitLength)

-- | A shrinker JOINS the budget node it is handed (it never opens a scope).
type Shrinker a = Budget -> a -> Falsifies a -> Int -> IO (a, Int)

-- | A generator: a sampler and an optional integrated shrinker.  The sampler
-- gets the enclosing budget scope (if any) so that a @filter@ can charge its
-- retries; it runs in 'IO' because the 'Rng' is a mutable stream.
data Gen a = Gen
  { genSample :: Maybe Budget -> Rng -> IO a
  , genShrink :: Maybe (Shrinker a)
  }

-- | Draws one value (outside any budget scope unless one is passed).
sample :: Maybe Budget -> Gen a -> Rng -> IO a
sample ctx g = genSample g ctx

invalid :: String -> Gen a
invalid msg = Gen (\_ _ -> argError msg) Nothing

-- | For combinators shrinking their components: JOINS the enclosing budget.
shrinkNested :: Budget -> Gen a -> a -> Falsifies a -> Int -> IO (a, Int)
shrinkNested b g v f maxSteps = do
  checkSteps maxSteps
  case genShrink g of
    Nothing -> pure (v, 0)
    Just s -> s b v f maxSteps

-- | Shrinks @value@ while @falsifies@ holds; @(value, 0)@ if there is no
-- shrinker.  A PUBLIC ENTRY POINT: it opens a child of @parent@ (a fresh root
-- for 'Nothing') with its own local cap, and charges every evaluation of
-- @falsifies@, so however a property or a custom shrinker nests calls, the
-- total work stays inside the outermost budget.
shrinkGen :: Config -> Maybe Budget -> Gen a -> a -> Falsifies a -> Int -> IO (a, Int)
shrinkGen cfg parent g v falsifies maxSteps = do
  checkSteps maxSteps
  withChild cfg parent maxSteps $ \b -> shrinkNested b g v (chargedFalsifies b falsifies) maxSteps

-- | Always generates @v@ and draws nothing.
constant :: a -> Gen a
constant v = Gen (\_ _ -> pure v) Nothing

-- | The in-range point nearest zero: the integer shrink target.
clampZero :: Integer -> Integer -> Integer
clampZero lo hi
  | lo > 0 = lo
  | hi < 0 = hi
  | otherwise = 0

intShrinker :: Integer -> Integer -> Shrinker Integer
intShrinker lo hi b v f maxSteps = shrinkIntCore (budgetConfig b) (Just lo) (Just hi) (clampZero lo hi) v f maxSteps

-- | @bounded(2) == 1@.  Shrinks True to False.
bool :: Gen Bool
bool = Gen (\_ r -> (== 1) <$> bounded r 2) (Just shrinker)
  where
    shrinker _ v f maxSteps
      | v && maxSteps > 0 = do
          ok <- f False
          pure (if ok then (False, 1) else (v, 0))
      | otherwise = pure (v, 0)

-- | @[0, 255]@: @bounded(256)@.
u8 :: Gen Integer
u8 = Gen (\_ r -> bounded r 256) (Just (intShrinker 0 255))

-- | A full 64-bit word: @next_u64@.
u64 :: Gen Integer
u64 = Gen (\_ r -> toInteger <$> nextU64 r) (Just (intShrinker 0 (2 ^ (64 :: Int) - 1)))

-- | Two stream words, high first, read as two's complement:
--
-- > words 0x8000...0 : 0   -> -2^127  (top bit set => subtract 2^128)
-- > words 0x7fff...f : f.. ->  2^127 - 1
sampleI128 :: Rng -> IO Integer
sampleI128 r = do
  hi <- nextU64 r
  lo <- nextU64 r
  let v = (toInteger hi `shiftL` 64) + toInteger lo
  pure (if v >= 2 ^ (127 :: Int) then v - 2 ^ (128 :: Int) else v)

i128 :: Gen Integer
i128 = Gen (\_ r -> sampleI128 r) (Just (intShrinker (negate (2 ^ (127 :: Int))) (2 ^ (127 :: Int) - 1)))

-- | A uniform integer over the inclusive range @[lo, hi]@ (VER05 7.2): the
-- offset from @lo@ is drawn by 'boundedBig'.  Shrinks toward the in-range
-- point nearest zero and never leaves the range.  A span wider than 16,384
-- bits is refused (CWE-400 on the k-word rejection loop).
int :: Integer -> Integer -> Gen Integer
int lo hi
  | lo > hi = invalid "int needs min <= max"
  | integerBitLength (hi - lo + 1) > maxIntBits = invalid "int span exceeds the width cap"
  | otherwise = Gen (\_ r -> boundedInt r lo hi) (Just (intShrinker lo hi))

boundedInt :: Rng -> Integer -> Integer -> IO Integer
boundedInt r lo hi = (lo +) <$> boundedBig r (hi - lo + 1)

checkLengths :: Int -> Int -> Int -> Maybe String
checkLengths minLen maxLen limit
  | minLen < 0 || maxLen < minLen = Just "needs 0 <= min_len <= max_len"
  | maxLen > limit = Just ("max_len " ++ show maxLen ++ " exceeds " ++ show limit)
  | otherwise = Nothing

-- | A byte string: @L = min_len + bounded(span)@, then @L@ draws of
-- @bounded(256)@.  Shrinks by sequence shrinking, respecting @min_len@.
bytes :: Int -> Int -> Gen B.ByteString
bytes minLen maxLen = case checkLengths minLen maxLen maxInputBytes of
  Just msg -> invalid ("bytes " ++ msg)
  Nothing -> Gen sampler (Just shrinker)
  where
    span' = toInteger (maxLen - minLen + 1)
    sampler _ r = do
      n <- (minLen +) . fromInteger <$> bounded r span'
      ws <- replicateM n (fromInteger <$> bounded r 256)
      pure (B.pack ws)
    shrinker b v f maxSteps = do
      -- the list conversion (and the string one on the way out) are copies too:
      -- charge them first, and do nothing once the budget is spent
      ok <- if maxSteps == 0 then pure False else chargeCopy b (2 * B.length v)
      if not ok
        then pure (v, 0)
        else do
          let cfg = budgetConfig b
              wrapped ws = do
                okc <- chargeCopy b (length ws)
                if okc then f (B.pack ws) else pure False
          (ws, steps) <- shrinkSequence cfg b minLen (Just (walkWord8 cfg)) wrapped maxSteps (B.unpack v)
          pure (B.pack ws, steps)

-- | Transforms generated values.  Drops shrinking: the function has no inverse.
mapG :: (a -> b) -> Gen a -> Gen b
mapG fn g = Gen (\c r -> fn <$> genSample g c r) Nothing

-- | Keeps only values satisfying the predicate.  After exactly @max_retries@
-- rejected samples sampling throws 'ExhaustedError'.  Shrinking is preserved: a
-- candidate failing the predicate is simply not a valid input.  Every retry is
-- CHARGED whenever a budget scope exists, so stacked filters cannot multiply
-- uncharged work: in a NESTED check a retry is an attempt charged up the chain;
-- at a top-level check root it is one element of work.  The predicate is
-- charged BEFORE it runs: it is property code and may be costly.
filterG :: Int -> (a -> Bool) -> Gen a -> Gen a
filterG n p = filterM n (pure . p)

filterM :: Int -> (a -> IO Bool) -> Gen a -> Gen a
filterM maxRetries p g
  | maxRetries < 1 || maxRetries > maxRetriesLimit =
      invalid ("max_retries " ++ show maxRetries ++ " outside [1, " ++ show maxRetriesLimit ++ "]")
  | otherwise = Gen sampler (Just shrinker)
  where
    sampler ctx r = go maxRetries
      where
        go 0 = throwIO (ExhaustedError ("filter rejected " ++ show maxRetries ++ " samples"))
        go k = do
          case ctx of
            Nothing -> pure ()
            Just b -> do
              ok <- if isNested b then charge b 1 else chargeCopy b 1
              unless ok (throwIO (ExhaustedError "the enclosing shrink budget was spent while filtering"))
          v <- genSample g ctx r
          keep <- p v
          if keep then pure v else go (k - 1 :: Int)
    shrinker b v f maxSteps =
      -- charge BEFORE the user predicate runs: it is property code and may be costly
      shrinkNested b g v (\c -> do ok <- chargeCopy b 1; if ok then p c >>= \keep -> if keep then f c else pure False else pure False) maxSteps

-- | Dependent generation: sample, then sample from the generator the function
-- returns.  No shrinking.
flatMap :: (a -> Gen b) -> Gen a -> Gen b
flatMap fn g = Gen (\c r -> genSample g c r >>= \a -> genSample (fn a) c r) Nothing

-- | Picks one generator uniformly (@bounded(len)@, then that arm's draws).
oneOf :: [Gen a] -> Gen a
oneOf [] = invalid "one_of needs at least one generator"
oneOf arms = Gen sampler Nothing
  where
    n = toInteger (length arms)
    sampler c r = do
      i <- bounded r n
      genSample (arms !! fromInteger i) c r

-- | A weighted choice among @(weight, gen)@ pairs: @pick = bounded(total)@; arms
-- are walked in order subtracting each weight, so a zero-weight arm can never be
-- selected.  Empty, a negative weight, a zero total or a total above 2^32 is
-- refused.
frequency :: [(Int, Gen a)] -> Gen a
frequency [] = invalid "frequency needs at least one arm"
frequency arms
  | any ((< 0) . fst) arms = invalid "frequency weights must be >= 0"
  | total == 0 = invalid "frequency total weight must be > 0"
  | total > maxFrequencyTotal = invalid "frequency total weight exceeds 2^32"
  | otherwise = Gen sampler Nothing
  where
    total = sum (map (toInteger . fst) arms)
    sampler c r = do
      pick <- bounded r total
      walk c r pick arms
    walk _ _ _ [] = argError "unreachable: pick < total"
    walk c r pick ((w, g) : rest)
      | pick < toInteger w = genSample g c r
      | otherwise = walk c r (pick - toInteger w) rest

-- | A list: @L = min_len + bounded(span)@, then @L@ elements in order.  Shrinks by
-- sequence shrinking with the element generator's shrinker for the element-wise
-- phase.
list :: Gen a -> Int -> Int -> Gen [a]
list el minLen maxLen = case checkLengths minLen maxLen maxListLen of
  Just msg -> invalid ("list " ++ msg)
  Nothing -> Gen sampler (Just shrinker)
  where
    span' = toInteger (maxLen - minLen + 1)
    sampler c r = do
      n <- (minLen +) . fromInteger <$> bounded r span'
      replicateM n (genSample el c r)
    shrinker b v f maxSteps =
      shrinkSequence (budgetConfig b) b minLen (Just (shrinkNested b el)) f maxSteps v

-- | Samples each generator left to right and shrinks one coordinate at a time,
-- in repeated passes until a pass makes no progress.  All passes share the one
-- budget scope, so the repeat-until-no-progress loop cannot multiply the work.
tuple2 :: Gen a -> Gen b -> Gen (a, b)
tuple2 ga gb = Gen sampler (Just shrinker)
  where
    sampler c r = do
      x <- genSample ga c r
      y <- genSample gb c r
      pure (x, y)
    shrinker b (x0, y0) f maxSteps = pass x0 y0 0
      where
        pass x y steps
          | steps >= maxSteps = pure ((x, y), steps)
          | otherwise = do
              (x', u1) <- shrinkNested b ga x (\c -> f (c, y)) (maxSteps - steps)
              let steps1 = steps + u1
              (y', u2) <-
                if maxSteps - steps1 > 0
                  then shrinkNested b gb y (\c -> f (x', c)) (maxSteps - steps1)
                  else pure (y, 0)
              if u1 + u2 > 0 then pass x' y' (steps1 + u2) else pure ((x', y'), steps1)

tuple3 :: Gen a -> Gen b -> Gen c -> Gen (a, b, c)
tuple3 ga gb gc = Gen sampler (Just shrinker)
  where
    sampler c r = do
      x <- genSample ga c r
      y <- genSample gb c r
      z <- genSample gc c r
      pure (x, y, z)
    shrinker b (x0, y0, z0) f maxSteps = pass x0 y0 z0 0
      where
        pass x y z steps
          | steps >= maxSteps = pure ((x, y, z), steps)
          | otherwise = do
              (x', u1) <- shrinkNested b ga x (\c -> f (c, y, z)) (maxSteps - steps)
              let s1 = steps + u1
              (y', u2) <-
                if maxSteps - s1 > 0
                  then shrinkNested b gb y (\c -> f (x', c, z)) (maxSteps - s1)
                  else pure (y, 0)
              let s2 = s1 + u2
              (z', u3) <-
                if maxSteps - s2 > 0
                  then shrinkNested b gc z (\c -> f (x', y', c)) (maxSteps - s2)
                  else pure (z, 0)
              if u1 + u2 + u3 > 0 then pass x' y' z' (s2 + u3) else pure ((x', y', z'), s2)

-- | A user sampler/shrinker pair.
customGen :: (Maybe Budget -> Rng -> IO a) -> Maybe (Shrinker a) -> Gen a
customGen = Gen

-- ---------------------------------------------------------------------------
-- Corpus entry points (pbt-v1)
-- ---------------------------------------------------------------------------

-- | @count@ primitives of @type@ (@bool@, @u8@, @u64@, @i128@) from @seed@.
genPrimitive :: Config -> Word64 -> Text -> Int -> IO [Val]
genPrimitive cfg seed ty count = do
  when (count < 0 || count > maxGenCount) (argError "count outside the generation limit")
  rng <- cfgNewRng cfg seed
  case ty of
    "bool" -> replicateM count ((VBool . (== 1)) <$> bounded rng 2)
    "u8" -> replicateM count (VInt <$> bounded rng 256)
    "u64" -> replicateM count (VInt . toInteger <$> nextU64 rng)
    "i128" -> replicateM count (VInt <$> sampleI128 rng)
    _ -> argError "unknown primitive type"

-- | @count@ uniform integers over @[lo, hi]@ from @seed@.  Each draw is about as
-- wide as the span, so @count x width@ bounds what the returned list can hold.
genBounded :: Config -> Word64 -> Integer -> Integer -> Int -> IO [Val]
genBounded cfg seed lo hi count = do
  when (lo > hi) (argError "need lo <= hi")
  when (count < 0 || count > maxGenCount) (argError "count outside the generation limit")
  let width = max 1 ((integerBitLength (hi - lo) + 7) `div` 8)
  when (toInteger count * toInteger width > 1048576) (argError "count x span exceeds the generation budget")
  rng <- cfgNewRng cfg seed
  replicateM count (VInt <$> boundedInt rng lo hi)

-- | @count@ byte strings with length in @[min_len, max_len]@ from @seed@.
genBytes :: Config -> Word64 -> Int -> Int -> Int -> IO [Val]
genBytes cfg seed minLen maxLen count = do
  maybe (pure ()) argError (checkLengths minLen maxLen maxInputBytes)
  when (count < 0 || count > maxGenCount) (argError "count outside the generation limit") -- applies even when max_len == 0
  when (maxLen > 0 && count > 1048576 `div` maxLen) (argError "count x max_len exceeds the generation budget")
  rng <- cfgNewRng cfg seed
  replicateM count (VBytes <$> genSample (bytes minLen maxLen) Nothing rng)
