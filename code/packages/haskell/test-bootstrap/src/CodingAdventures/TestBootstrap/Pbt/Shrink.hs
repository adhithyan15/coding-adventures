-- | Integrated shrinking (VER05 4, VER05e 4, VER05n 1 and 5).
--
-- A property that fails on @[913, -4, 77, 0, 2]@ has told you nothing; one that
-- fails on @[0]@ has told you what is wrong.  Shrinking walks from the first
-- failing candidate toward the smallest one that still fails.  It is
-- "integrated" because the generator that made the value knows its own bounds,
-- so a shrunk @int 5 10@ never becomes 0.
--
-- The public shrinkers take a property that returns True when it HOLDS (the
-- corpus convention) and return @(minimal, steps)@, where @steps@ counts
-- ACCEPTED moves only.  Internally every shrinker works with a @falsifies@
-- function (True while the candidate STILL FAILS), which the entry points
-- have already wrapped so that each call is charged one attempt to the budget
-- tree; a shrinker never re-charges the evaluation, it only charges the COPY
-- of a candidate (work) before building it.
module CodingAdventures.TestBootstrap.Pbt.Shrink
  ( Falsifies
  , ElemShrink
  , inBounds
  , intWalk
  , shrinkIntCore
  , shrinkSequence
  , walkWord8
  , shrinkIntWith
  , shrinkInt
  , shrinkListWith
  , shrinkList
  , shrinkBytesWith
  , shrinkBytes
  , simulateReplayWith
  , simulateReplay
  ) where

import Control.Exception (throwIO)
import Control.Monad (unless, when)
import qualified Data.ByteString as B
import Data.Foldable (toList)
import qualified Data.Sequence as S
import Data.Word (Word64, Word8)

import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Pbt.Budget
import CodingAdventures.TestBootstrap.Pbt.Types
import CodingAdventures.TestBootstrap.Prng (bounded)

-- | True while the candidate still FALSIFIES the property.
type Falsifies a = a -> IO Bool

-- | Shrinks one element: @value -> falsifies -> max_steps -> (value, steps)@.
type ElemShrink a = a -> Falsifies a -> Int -> IO (a, Int)

-- | @lo <= v <= hi@ with either bound optional.
inBounds :: Maybe Integer -> Maybe Integer -> Integer -> Bool
inBounds lo hi v = maybe True (<= v) lo && maybe True (v <=) hi

-- | Bisects then unit-steps @current@ toward @target@.
--
-- Each turn tries, in order,
--
-- 1. the bisection candidate @current - floor(diff / 2)@ when @|diff| > 1@;
-- 2. the unit candidate @current - signum diff@.
--
-- A candidate is accepted only if inside @[lo, hi]@, different from @current@
-- and still falsifying.  No acceptable candidate ends the walk.
--
-- Worked example (corpus @shrink_int/54321_le_100@, property @x <= 100@):
-- 54321 -> 27161 -> 13581 -> ... -> 101 in 15 accepted moves; 100 would satisfy
-- the property, so neither candidate from 101 is accepted.
--
-- @div@ FLOORS on negatives (@-81 `div` 2 == -41@), so for @diff = -81@ the
-- bisection candidate is @current + 41@; truncation would give @current + 40@.
intWalk :: Config -> Integer -> Integer -> Maybe Integer -> Maybe Integer -> Falsifies Integer -> Int -> IO (Integer, Int)
intWalk cfg start target lo hi falsifies budget = go start 0
  where
    go cur steps
      | steps >= budget || cur == target = pure (cur, steps)
      | otherwise = do
          let diff = cur - target
          m1 <- if abs diff > 1 then attempt cur (cfgBisect cfg cur diff) else pure Nothing
          m2 <- case m1 of
            Just c -> pure (Just c)
            Nothing -> attempt cur (cfgUnit cfg cur diff)
          case m2 of
            Nothing -> pure (cur, steps)
            Just c -> go c (steps + 1)
    attempt cur cand
      | cand /= cur && inBounds lo hi cand = do
          ok <- falsifies cand
          pure (if ok then Just cand else Nothing)
      | otherwise = pure Nothing

-- | The target probe followed by the walk (VER05 4.2): the target itself failing
-- cannot be improved on and costs exactly one step.
shrinkIntCore :: Config -> Maybe Integer -> Maybe Integer -> Integer -> Integer -> Falsifies Integer -> Int -> IO (Integer, Int)
shrinkIntCore cfg lo hi target initial falsifies maxSteps
  | maxSteps == 0 = pure (initial, 0)
  | otherwise = do
      probe <-
        if inBounds lo hi target && target /= initial
          then falsifies target
          else pure False
      if probe then pure (target, 1) else intWalk cfg initial target lo hi falsifies maxSteps

-- | A byte shrunk toward 0 by the integer walk.
walkWord8 :: Config -> ElemShrink Word8
walkWord8 cfg w falsifies steps = do
  (v, used) <- intWalk cfg (toInteger w) 0 (Just 0) (Just 255) (falsifies . fromInteger) steps
  pure (fromInteger v, used)

-- | Shrinks a falsifying sequence (VER05 4.3).
--
-- * /probe/ -- @min_len == 0@: does the empty list fail?  Then it is the answer,
--   one step.  (@min_len > 0@: probe the length-@min_len@ prefix and carry on
--   from it.)
-- * /bisection/ -- keep the left half, else the right half, while one fails.
-- * /deletion/ -- single elements, right to left.
-- * /element-wise/ -- left to right through the element shrinker, if any.
--
-- Worked example (corpus @list_sum_le_20@, @[10 5 8 4]@): the empty list and
-- both halves satisfy the property; deleting index 3 leaves @[10 5 8]@ (sum
-- 23, still failing); element-wise 10 -> 8 gives the 1-minimal @[8 5 8]@.
--
-- Every candidate COPY is charged to the budget tree BEFORE it is built; once
-- refused, every loop ends.  The working value is a 'S.Seq' so that indexing,
-- updating and deleting stay logarithmic however long the list is.  When
-- @max_steps == 0@ or the budget is already spent the input is returned as is.
shrinkSequence :: Config -> Budget -> Int -> Maybe (ElemShrink a) -> Falsifies [a] -> Int -> [a] -> IO ([a], Int)
shrinkSequence cfg b minLen mElem falsifies maxSteps initial
  | maxSteps == 0 = pure (initial, 0)
  | otherwise = do
      let start = S.fromList initial
          n0 = S.length start
      okCopy <- chargeCopy b n0
      if not okCopy
        then pure (initial, 0)
        else do
          probe <- tryCandidate (n0 > minLen) minLen (S.take minLen start)
          if probe && minLen == 0
            then pure ([], 1)
            else do
              let (cur0, steps0) = if probe then (S.take minLen start, 1) else (start, 0)
              (cur1, steps1) <- bisect cur0 steps0
              (cur2, steps2) <- if cfgSkipDeletion cfg then pure (cur1, steps1) else deletion cur1 steps1 (S.length cur1 - 1)
              (cur3, steps3) <- elementwise cur2 steps2
              pure (toList cur3, steps3)
  where
    floorLen = max 1 minLen

    -- charge the copy, then evaluate; a refused charge is "does not falsify"
    tryCandidate cond len cand
      | not cond = pure False
      | otherwise = do
          ok <- chargeCopy b len
          if ok then falsifies (toList cand) else pure False

    -- Phase 1: bisection, left half first, then the right half.
    bisect cur steps
      | S.length cur > floorLen && steps < maxSteps = do
          let n = S.length cur
              mid = n `div` 2
              left = S.take mid cur
              right = S.drop mid cur
          l <- tryCandidate (mid >= minLen) mid left
          if l
            then bisect left (steps + 1)
            else do
              r <- tryCandidate (n - mid >= minLen) (n - mid) right
              if r then bisect right (steps + 1) else pure (cur, steps)
      | otherwise = pure (cur, steps)

    -- Phase 2: single-element deletion, right to left.
    deletion cur steps i
      | i >= 0 && S.length cur > floorLen && steps < maxSteps = do
          hasRoom <- room b
          if not hasRoom
            then pure (cur, steps)
            else do
              ok <- chargeCopy b (S.length cur - 1)
              if not ok
                then pure (cur, steps)
                else do
                  let cand = S.deleteAt i cur
                  f <- falsifies (toList cand)
                  if f
                    then deletion cand (steps + 1) (min i (S.length cand - 1))
                    else deletion cur steps (i - 1)
      | otherwise = pure (cur, steps)

    -- Phase 3: element-wise, left to right.
    elementwise cur steps = case mElem of
      Nothing -> pure (cur, steps)
      Just shrinkElem -> goElem shrinkElem (S.length cur) cur steps 0
    goElem shrinkElem size cur steps idx
      | idx >= size = pure (cur, steps)
      | otherwise = do
          let remaining = maxSteps - steps
          hasRoom <- room b
          if remaining <= 0 || not hasRoom
            then pure (cur, steps)
            else do
              let withElement cand = do
                    ok <- chargeCopy b (S.length cur)
                    if ok then falsifies (toList (S.update idx cand cur)) else pure False
              (v, used) <- shrinkElem (S.index cur idx) withElement remaining
              if used > 0
                then goElem shrinkElem size (S.update idx v cur) (steps + used) (idx + 1)
                else goElem shrinkElem size cur steps (idx + 1)

-- | The user property of a corpus-style shrinker: True when it HOLDS.
type Holds a = a -> IO Bool

notFalsifying :: IO a
notFalsifying = throwIO NotFalsifying

-- | Shrinks a falsifying integer toward @target@ inside @[lo, hi]@ (the corpus
-- calls these min and max).  Throws 'NotFalsifying' when @initial@ satisfies
-- the property and 'ArgumentError' for @lo > hi@, an @initial@ outside the
-- bounds or a budget outside @[0, 10000]@.  A PUBLIC ENTRY POINT: opens a child
-- of @parent@ (a fresh root for 'Nothing') and charges every evaluation, the
-- initial one included.
shrinkIntWith :: Config -> Maybe Budget -> Integer -> Integer -> Integer -> Integer -> Int -> Holds Integer -> IO (Integer, Int)
shrinkIntWith cfg parent initial target lo hi maxSteps prop = do
  checkSteps maxSteps
  when (lo > hi) (argError "min exceeds max")
  unless (inBounds (Just lo) (Just hi) initial) (argError "initial value must lie within [min, max]")
  withChild cfg parent maxSteps $ \b -> do
    ok <- charge b 1
    if not ok
      then pure (initial, 0) -- starved by an enclosing shrink
      else do
        holds <- prop initial
        when holds notFalsifying
        let falsifies = chargedFalsifies b (fmap not . prop)
        shrinkIntCore (budgetConfig b) (Just lo) (Just hi) target initial falsifies maxSteps

-- | 'shrinkIntWith' with the default configuration, a fresh root and a 1000-step budget.
shrinkInt :: Integer -> Integer -> Integer -> Integer -> Holds Integer -> IO (Integer, Int)
shrinkInt initial target lo hi = shrinkIntWith defaultConfig Nothing initial target lo hi 1000

-- | Shrinks a falsifying list of integers: empty probe, halves, single
-- deletion, then each element toward @elemTarget@ (unbounded).
shrinkListWith :: Config -> Maybe Budget -> [Integer] -> Integer -> Int -> Holds [Integer] -> IO ([Integer], Int)
shrinkListWith cfg parent initial elemTarget maxSteps prop = do
  checkSteps maxSteps
  withChild cfg parent maxSteps $ \b -> do
    -- charged before the first copy and for the initial evaluation
    ok1 <- chargeCopy b (2 * length initial)
    ok2 <- if ok1 then charge b 1 else pure False
    if not ok2
      then pure (initial, 0)
      else do
        holds <- prop initial
        when holds notFalsifying
        let falsifies = chargedFalsifies b (fmap not . prop)
            walk v f steps = intWalk (budgetConfig b) v elemTarget Nothing Nothing f steps
        shrinkSequence (budgetConfig b) b 0 (Just walk) falsifies maxSteps initial

shrinkList :: [Integer] -> Integer -> Holds [Integer] -> IO ([Integer], Int)
shrinkList initial elemTarget = shrinkListWith defaultConfig Nothing initial elemTarget 1000

-- | Shrinks falsifying bytes: sequence shrinking over byte values toward 0x00.
shrinkBytesWith :: Config -> Maybe Budget -> B.ByteString -> Int -> Holds B.ByteString -> IO (B.ByteString, Int)
shrinkBytesWith cfg parent initial maxSteps prop = do
  checkSteps maxSteps
  withChild cfg parent maxSteps $ \b -> do
    -- the copy, the list conversion and the string conversion back
    ok1 <- chargeCopy b (3 * B.length initial)
    ok2 <- if ok1 then charge b 1 else pure False
    if not ok2
      then pure (initial, 0)
      else do
        holds <- prop initial
        when holds notFalsifying
        -- each attempt also converts its candidate back: charge that copy too
        let falsifies ws = do
              ok <- chargeCopy b (length ws)
              if ok then chargedFalsifies b (fmap not . prop) (B.pack ws) else pure False
        (ws, steps) <- shrinkSequence (budgetConfig b) b 0 (Just (walkWord8 (budgetConfig b))) falsifies maxSteps (B.unpack initial)
        pure (B.pack ws, steps)

shrinkBytes :: B.ByteString -> Holds B.ByteString -> IO (B.ByteString, Int)
shrinkBytes initial = shrinkBytesWith defaultConfig Nothing initial 1000

-- | Replays a failure from nothing but @seed@: draw up to 1000 values
-- @bounded(1000)@; the first that falsifies is the initial failure, then
-- shrunk toward 0 over @[0, 1000]@.  Same seed, same pair, forever.  Returns
-- @(initial_fail, minimal)@.
simulateReplayWith :: Config -> Maybe Budget -> Word64 -> Holds Integer -> IO (Integer, Integer)
simulateReplayWith cfg parent seed prop = withChild cfg parent 1000 $ \b -> do
  rng <- cfgNewRng (budgetConfig b) seed
  let draw :: Int -> IO (Maybe Integer)
      draw 0 = pure Nothing
      draw k = do
        ok <- charge b 1
        if not ok
          then pure Nothing
          else do
            v <- bounded rng 1000
            holds <- prop v
            if holds then draw (k - 1) else pure (Just v)
  first <- draw 1000
  case first of
    Nothing -> argError "property was not falsified within 1000 draws"
    Just v -> do
      (minimal, _) <- shrinkIntWith (budgetConfig b) (Just b) v 0 0 1000 1000 prop
      pure (v, minimal)

simulateReplay :: Word64 -> Holds Integer -> IO (Integer, Integer)
simulateReplay = simulateReplayWith defaultConfig Nothing
