-- | Integrated shrinking: the pinned step counts on the edge cases that the
-- pbt-v1 corpus cannot catch (VER05n 8.1).
module ShrinkSpec (tests) where

import Control.Exception (ErrorCall (..), displayException)
import Data.IORef
import qualified Data.ByteString as B

import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Pbt.Shrink
import CodingAdventures.TestBootstrap.Pbt.Types
import Harness

isArg :: PbtError -> Bool
isArg (ArgumentError m) = length m > 0 && length (displayException (ArgumentError m)) > 0
isArg _ = False

isNotFalsifying :: PbtError -> Bool
isNotFalsifying NotFalsifying = True
isNotFalsifying _ = False

tests :: [Test]
tests =
  [ test "the corpus headline example" $ do
      r <- shrinkInt 54321 0 0 100000 (pure . (<= 100))
      assertEq "54321" (101, 15) r
  , test "the target probe is one step" $ do
      r <- shrinkInt 500 0 0 1000 (pure . (< (-5)))
      assertEq "everything fails" (0, 1) r
  , test "negative bisection floors" $ do
      -- diff = -81 => floor(-81/2) = -41, so the candidate is -81 + 41 = -40
      -- (truncation toward zero would give -41)
      seen <- newIORef []
      _ <- shrinkInt (-81) 0 (-200) 200 (\x -> modifyIORef seen (x :) >> pure (x >= -10))
      xs <- reverse <$> readIORef seen
      assertEq "initial, target probe, first bisection" [-81, 0, -40] (take 3 xs)
  , test "unit steps when bisection cannot progress" $ do
      r <- shrinkInt 100 0 (-100) 500 (pure . (\x -> x >= -10 && x <= 50))
      assertEq "unit walk" (51, 49) r
  , test "the result stays in bounds and is exactly the threshold for a monotone property" $
      mapM_
        ( \t -> mapM_
            ( \start -> if start < t && start >= -t
                then pure ()
                else do
                  (m, _) <- shrinkInt start 0 (-400) 400 (pure . (\x -> abs x < t))
                  assertEq ("threshold " ++ show t ++ " from " ++ show start) t (abs m)
            )
            [-300, 300, 77]
        )
        [1 .. 60]
  , test "a zero budget returns the start" $ do
      r <- shrinkIntWith defaultConfig Nothing 99 0 0 100 0 (pure . (< 5))
      assertEq "no steps" (99, 0) r
  , test "the budget limits accepted moves" $ do
      (v, s) <- shrinkIntWith defaultConfig Nothing 54321 0 0 100000 3 (pure . (<= 100))
      assertEq "exactly three steps" 3 s
      assertTrue "still above the minimum" (v > 101)
  , test "argument validation" $ do
      let prop = pure . (< (5 :: Integer))
      assertThrows "not falsifying" isNotFalsifying (shrinkInt 1 0 0 10 prop)
      assertThrows "min > max" isArg (shrinkInt 50 0 10 0 prop)
      assertThrows "outside bounds" isArg (shrinkInt 50 0 0 10 prop)
      assertThrows "negative budget" isArg (shrinkIntWith defaultConfig Nothing 5 0 0 10 (-1) prop)
      assertThrows "huge budget" isArg (shrinkIntWith defaultConfig Nothing 5 0 0 10 10001 prop)
  , test "a target outside the bounds is not probed" $ do
      r1 <- shrinkInt 99 10 10 500 (pure . (< 5))
      assertEq "refinement target" (10, 1) r1
      (m, _) <- shrinkInt 99 5 10 500 (pure . (< 50))
      assertEq "target below min" 50 m
  , test "bisection accepts the right half, then the left half, in exactly this order" $ do
      calls <- newIORef []
      let prop l = modifyIORef calls (l :) >> pure (5 `notElem` l)
      r <- shrinkList [0, 0, 0, 0, 5, 0, 0, 0] 0 prop
      assertEq "result" ([5], 3) r
      seen <- reverse <$> readIORef calls
      assertEq
        "evaluation order"
        [ [0, 0, 0, 0, 5, 0, 0, 0] -- the initial check
        , [] -- empty probe
        , [0, 0, 0, 0] -- left half holds: rejected
        , [5, 0, 0, 0] -- right half falsifies: ACCEPTED
        , [5, 0] -- left half falsifies: accepted
        , [5] -- left half falsifies: accepted
        , [3] -- element phase: bisection candidate, holds
        , [4] -- element phase: unit candidate, holds
        ]
        seen
  , test "bisection, only left halves" $ do
      (_, s) <- shrinkList [5, 0, 0, 0, 0, 0, 0, 0] 0 (pure . notElem 5)
      assertEq "steps" 3 s
  , test "bisection, only right halves" $ do
      r <- shrinkList [0, 0, 0, 0, 0, 0, 0, 5] 0 (pure . notElem 5)
      assertEq "result" ([5], 3) r
  , test "the empty probe wins in one step" $ do
      r <- shrinkList [1, 2, 3] 0 (const (pure False))
      assertEq "empty" ([], 1) r
  , test "deletion is right to left and 1-minimal" $ do
      (m, _) <- shrinkList [1, 2, 3, 3, 4, 5] 0 (pure . (\l -> and (zipWith (/=) l (drop 1 l))))
      assertEq "adjacent duplicates" [3, 3] m
  , test "the element walk does not go below the element target" $ do
      (m, _) <- shrinkList [9, 9, 9] 4 (pure . (\l -> sum l < 10))
      assertTrue "still falsifies" (sum m >= 10)
      assertTrue "elements stay at or above the target" (all (>= 4) m)
  , test "list validation" $ do
      assertThrows "not falsifying" isNotFalsifying (shrinkList [1] 0 (const (pure True)))
      assertThrows "bad budget" isArg (shrinkListWith defaultConfig Nothing [1] 0 (-1) (const (pure False)))
  , test "shrink_bytes zeroes and truncates" $ do
      (m, s) <- shrinkBytes (B.pack [1 .. 8]) (pure . (\b -> B.length b < 5))
      assertEq "minimal" (B.pack [0, 0, 0, 0, 0], 16) (m, s)
  , test "shrink_bytes validation" $ do
      assertThrows "not falsifying" isNotFalsifying (shrinkBytes "a" (const (pure True)))
  , test "simulate_replay is deterministic" $ do
      a <- simulateReplay 42 (pure . (<= 100))
      assertEq "pinned" (413, 101) a
      b <- simulateReplay 42 (pure . (<= 100))
      assertEq "same seed, same pair" a b
  , test "simulate_replay needs a falsifiable property" $
      assertThrows "never falsified" isArg (simulateReplay 1 (const (pure True)))
  , test "a zero step budget returns the input untouched" $ do
      let cfg = defaultConfig
      r <- shrinkIntWith cfg Nothing 5 0 0 10 0 (pure . (< 3))
      assertEq "no steps" (5, 0) r
  , test "a sabotaged deletion phase changes the answer" $ do
      let prop = pure . (\l -> and (zipWith (/=) l (drop 1 l)))
      (honest, _) <- shrinkListWith defaultConfig Nothing [1, 2, 3, 3, 4, 5] 0 1000 prop
      (sabotaged, _) <- shrinkListWith defaultConfig {cfgSkipDeletion = True} Nothing [1, 2, 3, 3, 4, 5] 0 1000 prop
      assertTrue "different" (honest /= sabotaged)
  , test "a throwing property propagates from a public shrinker" $
      assertThrows "boom" (\(ErrorCall _) -> True) (shrinkInt 5 0 0 10 (\_ -> error "boom"))
  ]
