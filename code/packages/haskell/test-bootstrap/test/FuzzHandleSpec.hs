-- | The per-execution handle (VER06o 5.2, 5.3): a fresh handle per execution,
-- sticky exhaustion in BOTH directions, retired handles inert, forged handles inert.
module FuzzHandleSpec (tests) where

import Control.Concurrent (forkIO, threadDelay)
import Control.Concurrent.MVar
import Control.Exception
import Control.Monad (forM_, replicateM_)
import Data.IORef

import CodingAdventures.TestBootstrap.Error (PbtError (..))
import CodingAdventures.TestBootstrap.Fuzz.Handle
import CodingAdventures.TestBootstrap.Fuzz.Handle.Internal
import Harness

isSentinel :: Which -> BudgetSentinel -> Bool
isSentinel w (BudgetSentinel _ w') = w == w'

isArg :: PbtError -> Bool
isArg (ArgumentError _) = True
isArg _ = False

tests :: [Test]
tests =
  [ test "edges are recorded, reduced mod 65536 with a floored wrap" $ do
      h <- newHandle 10 10
      mapM_ (edge h) [3, 3 + 65536, -1, 64, 63, 0]
      es <- edges h
      assertEq "edges" [0, 3, 63, 64, 65535] es
      n <- edgeCalls h
      assertEq "calls" 6 n
  , test "edgeId is a stable FNV-1a id" $ do
      assertEq "stable" (edgeId "abc") (edgeId "abc")
      assertEq "fnv1a abc" 16654208175385433931 (edgeId "abc")
      assertTrue "differs" (edgeId "a" /= edgeId "b")
  , test "tick passes the budget exactly, then throws, and keeps throwing (sticky)" $ do
      h <- newHandle 3 100
      replicateM_ 3 (tick h 1)
      assertThrows "4th tick" (isSentinel WhichSteps) (tick h 1)
      assertThrows "5th tick still" (isSentinel WhichSteps) (tick h 1)
      assertThrows "zero tick still" (isSentinel WhichSteps) (tick h 0)
      ex <- exhausted h
      assertEq "flag" ExSteps ex
  , test "a steps breach also makes a LATER reserve throw (and vice versa)" $ do
      h <- newHandle 1 1000
      assertThrows "steps breach" (isSentinel WhichSteps) (tick h 2)
      assertThrows "reserve after steps" (isSentinel WhichSteps) (reserve h 1)
      h2 <- newHandle 1000 10
      assertThrows "mem breach" (isSentinel WhichMem) (reserve h2 11)
      assertThrows "tick after mem" (isSentinel WhichMem) (tick h2 1)
      assertThrows "reserve again" (isSentinel WhichMem) (reserve h2 0)
      ex <- exhausted h2
      assertEq "mem flag" ExMem ex
  , test "the FIRST breach wins" $ do
      h <- newHandle 1 1
      assertThrows "steps first" (isSentinel WhichSteps) (tick h 5)
      assertThrows "mem later still reports steps" (isSentinel WhichSteps) (reserve h 5)
      ex <- exhausted h
      assertEq "first wins" ExSteps ex
  , test "negative and huge charges are clamped, never wrap" $ do
      h <- newHandle 5 5
      tick h (-100)
      u0 <- stepsUsed h
      assertEq "negative charges nothing" 0 u0
      assertThrows "huge" (isSentinel WhichSteps) (tick h maxBound)
      u <- stepsUsed h
      assertEq "clamped to budget + 1" 6 u
      h2 <- newHandle 5 5
      assertThrows "huge reserve" (isSentinel WhichMem) (reserve h2 maxBound)
      m <- memUsed h2
      assertEq "clamped" 6 m
  , test "a retired handle is inert: edge is a no-op, tick and reserve throw the retired sentinel" $ do
      h <- newHandle 100 100
      edge h 7
      wasLive <- retire h
      assertTrue "was live" wasLive
      again <- retire h
      assertFalse "second retire" again
      r <- isRetired h
      assertTrue "retired" r
      edge h 9
      es <- edges h
      assertEq "no new edge" [7] es
      n <- edgeCalls h
      assertEq "no new call" 1 n
      assertThrows "tick" (isSentinel WhichRetired) (tick h 1)
      assertThrows "reserve" (isSentinel WhichRetired) (reserve h 1)
      s <- stepsUsed h
      assertEq "no step charged" 0 s
  , test "a handle forwarded to another thread and kept after the execution is inert" $ do
      h <- newHandle 100 100
      stash <- newIORef h
      done <- newEmptyMVar
      _ <- forkIO (retire h >>= \_ -> threadDelay 1000 >> readIORef stash >>= \k -> edge k 5 >> putMVar done ())
      _ <- within 10 (takeMVar done)
      es <- edges h
      assertEq "no edge from the late thread" [] es
      assertThrows "late tick" (isSentinel WhichRetired) (tick h 1)
  , test "a fresh handle per execution: budgets and edges never leak between handles" $ do
      a <- newHandle 1 1
      b <- newHandle 1 1
      assertThrows "a breaches" (isSentinel WhichSteps) (tick a 5)
      tick b 1
      edge a 1
      es <- edges b
      assertEq "b untouched" [] es
      exb <- exhausted b
      assertEq "b not exhausted" ExNone exb
  , test "a handle forged with another token is inert" $ do
      h <- newHandle 100 100
      let forged = h {hToken = hToken h + 1}
      edge forged 4
      es <- edges h
      assertEq "forged edge ignored" [] es
      assertThrows "forged tick" (isSentinel WhichRetired) (tick forged 1)
      assertThrows "forged reserve" (isSentinel WhichRetired) (reserve forged 1)
  , test "the sentinel carries the handle's token" $ do
      h <- newHandle 0 0
      r <- try (tick h 1)
      case r of
        Left (BudgetSentinel tok w) -> do
          assertEq "token" (hToken h) tok
          assertEq "which" WhichSteps w
        Right () -> failTest "expected a sentinel"
  , test "exhausted cross-checks the counters: a cleared flag is still a breach" $ do
      h <- newHandle 2 2
      assertThrows "breach" (isSentinel WhichSteps) (tick h 5)
      atomicModifyIORef' (hRef h) (\s -> (s {hsExhausted = ExNone}, ()))
      ex <- exhausted h
      assertEq "steps counter > budget" ExSteps ex
      m <- newHandle 2 2
      assertThrows "mem breach" (isSentinel WhichMem) (reserve m 5)
      atomicModifyIORef' (hRef m) (\s -> (s {hsExhausted = ExNone}, ()))
      exm <- exhausted m
      assertEq "mem counter > budget" ExMem exm
  , test "newHandle validates its budgets" $ do
      assertThrows "steps" isArg (newHandle (-1) 1)
      assertThrows "mem" isArg (newHandle 1 (-1))
      assertThrows "steps high" isArg (newHandle 1000000000 1)
      assertThrows "mem high" isArg (newHandle 1 2000000000)
      assertEq "show" "Handle<>" . show =<< newHandle 1 1
  , test "many threads edging one handle never lose an update" $ do
      h <- newHandle 10 10
      dones <- mapM (\t -> do d <- newEmptyMVar; _ <- forkIO (forM_ [t * 100 .. t * 100 + 99 :: Integer] (edge h) >> putMVar d ()); pure d) [0 .. 7 :: Integer]
      forM_ dones (within 10 . takeMVar)
      es <- edges h
      assertEq "800 distinct edges" 800 (length es)
      n <- edgeCalls h
      assertEq "800 calls" 800 n
  ]
