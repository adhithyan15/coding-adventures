-- | One target execution in its OWN thread (VER06o 5).
--
-- A GHC green thread is the only unit the RTS can bound and kill, so every
-- execution runs in a forked worker under the engine's wall-clock 'timeout':
--
-- * the worker is forked MASKED and unmasks only around the target, and it ALWAYS
--   fills its result 'MVar' (even when killed), so the engine's wait never depends
--   on the target behaving;
-- * on expiry the engine 'killThread's the worker from a HELPER thread (the kill
--   blocks until delivered) and waits a bounded 'killWaitMs'; a worker stuck in an
--   uninterruptible foreign call is LEAKED, never waited for;
-- * exceptions are caught only through 'tryRecoverable': every asynchronous
--   exception is rethrown, never swallowed.  The single asynchronous exception the
--   worker handles itself is its OWN 'AllocationLimitExceeded' (below);
-- * @-fno-omit-yields@ (on the shared stanza) makes a loop that never allocates
--   reach a safe point, so 'killThread' and 'timeout' can interrupt it.
--
-- MEMORY.  GHC has no per-thread heap limit.  The worker sets
-- @setAllocationCounter limit@ and @enableAllocationLimit@: when the thread's
-- allocation counter goes below zero the RTS throws 'AllocationLimitExceeded' to
-- it.  The limit counts TOTAL bytes allocated, not residency (a target that
-- allocates and drops 2 GiB hits a 1 GiB limit with a tiny live heap), to an
-- accuracy of about 4 KiB, and it does not charge another thread's allocation.
--
-- That exception is @oom@ ONLY when it is this worker's own limit tripping.  After
-- a real trip the RTS re-initialises the counter to a small allowance (100 KiB by
-- default), so a counter below @limit / 8@ means the thread consumed essentially
-- all of it.  A target that merely @throwIO AllocationLimitExceeded@ with a nearly
-- full counter is a crash of kind @AllocationLimitExceeded@.  A target that catches
-- the trip and carries on is judged at the end by the same counter, like the step
-- budget's sticky flag.
--
-- After the worker has finished (or been given up on) the engine (1) retires the
-- handle, (2) reads the sticky exhausted state and the counters, and only then
-- (3) looks at how the worker ended.  A target that swallowed the budget sentinel
-- and went on is still a timeout or an oom, never a pass and never a crash.
--
-- THE WALL CLOCK HOLDS ONLY IF THE TARGET IS BUILT WITH @-fno-omit-yields@ (VER06o
-- 5.4).  GHC interrupts a thread only at a safe point.  A non-allocating loop, or a
-- long non-yielding call (bignum @^@ with a huge exponent, an unsafe FFI call), in
-- code compiled WITHOUT that flag never reaches one: it hangs the whole process, and
-- the CI job timeout is the only backstop (fail closed, but no verdict).  Add
-- @ghc-options: -fno-omit-yields@ to the test stanza that builds the target and its
-- dependencies.  An out-of-process runner is future work.
--
-- The allocation limit is NOT delivered while the worker is masked, which honest
-- @modifyMVar@ / @bracket@ acquire code also is: such a region can overshoot the
-- memory budget until it unmasks.  A hostile target can also call
-- @setAllocationCounter@ / @disableAllocationLimit@ on its own thread to fake or hide
-- an oom (the trusted-target model).
--
-- LEAKS.  A worker that ignores the kill (masked loop) is counted; a campaign that
-- has leaked 'maxLeakedWorkers' is aborted with the @workers-leaked@ failure and is
-- never clean.
--
-- LIMITS (recorded in VER06o 5.4): a thread the target forks, a @forkOS@ thread or
-- a foreign call that never returns is outside the engine's reach; a target that
-- masks asynchronous exceptions around a non-interruptible loop defers the kill and
-- is reported as a timeout with its worker leaked.
module CodingAdventures.TestBootstrap.Fuzz.Executor
  ( Target
  , ExecResult (..)
  , runExecution
  , runExecutionTracked
  , staleKind
  ) where

import Control.Concurrent
import Control.Exception
import Control.Monad (void)
import Data.IORef
import Data.Maybe (isNothing)
import qualified Data.ByteString as B
import Data.Int (Int64)
import Data.Text (Text)
import GHC.Conc (disableAllocationLimit, enableAllocationLimit, getAllocationCounter, setAllocationCounter)
import System.Timeout (timeout)

import CodingAdventures.TestBootstrap.Fuzz.Handle
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Fuzz.Signature (kindOf)
import CodingAdventures.TestBootstrap.Fuzz.Types
import CodingAdventures.TestBootstrap.Pbt.Safe (tryRecoverable)

-- | A fuzz target: it gets the input and a fresh per-execution handle.  Any
-- synchronous exception escaping it is a crash.
type Target = B.ByteString -> Handle -> IO ()

-- | What one execution came to; the edges are those of THIS execution, read after
-- the handle was retired.
data ExecResult = ExecResult
  { erOutcome :: !Outcome
  , erEdges :: [Integer]
  , erEdgeCalls :: !Int
  , erLeaked :: !Bool
  -- ^ the worker never answered the kill (it is leaked, never waited for)
  }

-- | How the worker ended, with the allocation counter read after the limit was
-- disabled.
data Raw = RDone | RSync SomeException | RTrip | RKilled

data Result = Result Raw !Int64

-- | The crash kind of a sentinel that did not come from a breach of THIS execution.
staleKind :: Text
staleKind = "stale-budget-sentinel"

runExecution :: Target -> B.ByteString -> FuzzOptions -> IO ExecResult
runExecution target input o = do
  leaks <- newIORef 0
  runExecutionTracked leaks target input o

-- | 'runExecution' with a campaign-wide leak counter.  Once ONE worker has leaked
-- the engine waits only 'leakedKillWaitMs' for later ones, which stops the slowdown
-- cascade of a target that always masks and spins.
runExecutionTracked :: IORef Int -> Target -> B.ByteString -> FuzzOptions -> IO ExecResult
runExecutionTracked leaks target input o = do
  h <- newHandle (fzStepBudget o) (fzMemBudget o)
  done <- newEmptyMVar
  let limit = fromIntegral (max (fzMemBudget o) allocationFloor) :: Int64
      body unmask = do
        setAllocationCounter limit
        enableAllocationLimit
        r <- try (tryRecoverable (unmask (target input h)))
        disableAllocationLimit
        c <- getAllocationCounter
        pure $ Result (case r of
          Right (Right ()) -> RDone
          Right (Left e) -> RSync e
          Left AllocationLimitExceeded -> RTrip) c
  tid <- mask_ $ forkIOWithUnmask $ \unmask ->
    (body unmask >>= putMVar done)
      `catch` \(_ :: SomeException) -> putMVar done (Result RKilled limit)
  let reap = void (forkIO (killThread tid))
  waited <- timeout (fzTimeoutMs o * 1000) (takeMVar done) `onException` reap
  (res, wall) <- case waited of
    Just r -> pure (Just r, False)
    Nothing -> do
      reap
      before <- readIORef leaks
      r2 <- timeout ((if before > 0 then leakedKillWaitMs else killWaitMs) * 1000) (takeMVar done)
      case r2 of
        Nothing -> modifyIORef' leaks (+ 1)
        Just _ -> pure ()
      pure (r2, True)
  _ <- retire h
  ex <- exhausted h
  es <- edges h
  n <- edgeCalls h
  pure (ExecResult (classify ex wall res limit) es n (wall && isNothing res))

classify :: Exhaustion -> Bool -> Maybe Result -> Int64 -> Outcome
classify ex wall res limit
  | ex == ExSteps = Timeout
  | ex == ExMem = Oom
  | wall = Timeout
  | otherwise = case res of
      Nothing -> Timeout
      Just (Result raw c)
        | c < limit `div` 8 -> Oom
        | otherwise -> case raw of
            RDone -> Pass
            RTrip -> Crash "AllocationLimitExceeded" []
            RKilled -> Crash "worker-killed" []
            RSync e -> case fromException e of
              Just (BudgetSentinel _ _) -> Crash staleKind []
              Nothing -> Crash (kindOf e) []
