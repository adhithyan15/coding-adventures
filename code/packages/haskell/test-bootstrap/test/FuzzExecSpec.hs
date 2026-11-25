-- | Execution isolation and classification, and the campaign contracts
-- (VER06o 5, 6).  Every execution runs under the executor's wall clock and every
-- test under the harness's 60-second timeout, so nothing here can hang.
module FuzzExecSpec (tests) where

import Control.Concurrent
import Control.Exception
import Control.Monad (forM, forM_, void, when)
import qualified Data.ByteString as B
import qualified Data.ByteString.Internal as BI
import Foreign.ForeignPtr.Unsafe (unsafeForeignPtrToPtr)
import Foreign.Ptr (ptrToIntPtr)
import GHC.Clock (getMonotonicTime)
import Data.IORef
import qualified Data.Map.Strict as M
import qualified Data.Text as T
import System.Exit (ExitCode (..), exitWith)

import CodingAdventures.TestBootstrap.Error (PbtError (..))
import CodingAdventures.TestBootstrap.Fuzz
import CodingAdventures.TestBootstrap.Fuzz.Engine
import CodingAdventures.TestBootstrap.Fuzz.Executor
import CodingAdventures.TestBootstrap.Fuzz.Handle
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Pbt.Safe (tryRecoverable)
import CodingAdventures.TestBootstrap.Pbt.Types (CiMode (..))
import Harness

-- | Fast options: short wall clock, small budgets.
fast :: FuzzOptions
fast = defaultFuzzOptions {fzTimeoutMs = 400, fzMemBudget = 8 * 1048576, fzMaxIterations = 60, fzSeed = 7, fzCi = CiOff}

exec1 :: FuzzOptions -> Target -> IO ExecResult
exec1 o t = runExecution t (B.pack [1, 2, 3]) o

outcomeOf :: FuzzOptions -> Target -> IO Outcome
outcomeOf o t = erOutcome <$> exec1 o t

isArg :: PbtError -> Bool
isArg (ArgumentError _) = True
isArg _ = False

-- | A hostile exception: neither 'show' nor 'displayException' may be called.
data Evil = Evil
instance Show Evil where show _ = error "show was called on a target value"
instance Exception Evil where displayException _ = error "displayException was called"

data Plain = Plain deriving (Show)
instance Exception Plain

-- | True when @child@ lies inside @parent@'s buffer (a slice of it).  A copy never
-- does: it is a fresh allocation.
inside :: B.ByteString -> B.ByteString -> Bool
inside child parent =
  let (fc, lc) = (fst3 (BI.toForeignPtr child), B.length child)
      (fp, lp) = (fst3 (BI.toForeignPtr parent), B.length parent)
      ac = ptrToInt (unsafeForeignPtrToPtr fc)
      ap = ptrToInt (unsafeForeignPtrToPtr fp)
   in lc > 0 && ac >= ap && ac + lc <= ap + lp
  where
    fst3 (a, _, _) = a
    ptrToInt = (\p -> fromIntegral (ptrToIntPtr p) :: Int)

-- | A masked loop that ignores the kill until the test sets the stop flag, so the
-- leaked worker stops burning CPU for later tests.
maskedSpin :: IORef Bool -> IO ()
maskedSpin stop = mask_ loop
  where
    loop = do
      s <- readIORef stop
      if s then pure () else loop

-- | A loop that never allocates and never returns (an Int cycles through 2^64
-- values): only @-fno-omit-yields@ lets the wall clock interrupt it.
spin :: Int -> Int
spin n = if n == 5 then n else spin (n + 1)
{-# NOINLINE spin #-}

spinForever :: Int -> Int
spinForever n = if n == -42 && n /= n then n else spinForever (n + 1)
{-# NOINLINE spinForever #-}

-- | Allocates garbage without bound (total allocation, tiny residency).
allocBomb :: IO ()
allocBomb = go (0 :: Int)
  where
    go n = do
      _ <- evaluate (M.size (M.fromList [(i, i) | i <- [n .. n + 300]]))
      go (n + 1)

-- | A target that crashes on inputs of 7 or more bytes and reports edges.
crashy :: Target
crashy dat h = do
  edge h (edgeId "entered")
  when (B.length dat > 3) (edge h (edgeId "long"))
  when (B.length dat >= 7) (throwIO Plain)

tests :: [Test]
tests =
  [ test "a passing target is a pass, and its edges are read after retirement" $ do
      r <- exec1 fast (\_ h -> edge h 5 >> edge h 6)
      assertEq "outcome" Pass (erOutcome r)
      assertEq "edges" [5, 6] (erEdges r)
      assertEq "calls" 2 (erEdgeCalls r)
  , test "any synchronous exception is a crash classified by TYPE (never message)" $ do
      o1 <- outcomeOf fast (\_ _ -> error "boom")
      assertEq "error" (Crash "ErrorCall" []) o1
      o2 <- outcomeOf fast (\_ _ -> throwIO Plain)
      assertEq "plain" (Crash "Plain" []) o2
      o3 <- outcomeOf fast (\_ _ -> void (evaluate (1 `div` (0 :: Int))))
      assertEq "arith" (Crash "ArithException" []) o3
      o4 <- outcomeOf fast (\_ _ -> exitWith (ExitFailure 3))
      assertEq "exit code" (Crash "ExitCode" []) o4
      o5 <- outcomeOf fast (\_ _ -> void (evaluate (head ([] :: [Int]))))
      assertEq "head []" (Crash "ErrorCall" []) o5
  , test "a hostile exception (show and displayException throw) is classified without either being called" $ do
      o <- outcomeOf fast (\_ _ -> throwIO Evil)
      assertEq "evil" (Crash "Evil" []) o
  , test "the wall clock kills a target that sleeps forever" $ do
      o <- outcomeOf fast {fzTimeoutMs = 100} (\_ _ -> threadDelay 30000000)
      assertEq "timeout" Timeout o
  , test "a loop that never allocates is interrupted (-fno-omit-yields)" $ do
      o <- outcomeOf fast {fzTimeoutMs = 200} (\_ _ -> void (evaluate (spinForever 0)))
      assertEq "timeout" Timeout o
      p <- outcomeOf fast (\_ _ -> void (evaluate (spin 0)))
      assertEq "finite loop is fine" Pass p
  , test "an allocation bomb is an oom through the worker's OWN allocation limit" $ do
      o <- outcomeOf fast {fzTimeoutMs = 20000} (\_ _ -> allocBomb)
      assertEq "oom" Oom o
  , test "a target that catches the allocation trip and returns is STILL an oom (counter evidence)" $ do
      o <- outcomeOf fast {fzTimeoutMs = 20000} (\_ _ -> allocBomb `catch` \AllocationLimitExceeded -> pure ())
      assertEq "oom" Oom o
  , test "a forged AllocationLimitExceeded is a crash, never an oom" $ do
      o <- outcomeOf fast (\_ _ -> throwIO AllocationLimitExceeded)
      assertEq "crash" (Crash "AllocationLimitExceeded" []) o
  , test "async exceptions are rethrown by tryRecoverable, never swallowed" $ do
      r <- try (tryRecoverable (throwIO ThreadKilled)) :: IO (Either AsyncException (Either SomeException ()))
      case r of
        Left ThreadKilled -> pure ()
        _ -> failTest "ThreadKilled was swallowed"
      -- a target that kills itself is a failure (fail closed), never a pass
      o <- outcomeOf fast (\_ _ -> myThreadId >>= killThread)
      assertEq "self-kill" (Crash "worker-killed" []) o
  , test "a swallowed step breach is a timeout, whatever the target does next" $ do
      let swallow h = tick h 100 `catch` \(_ :: BudgetSentinel) -> pure ()
          o = fast {fzStepBudget = 5}
      a <- outcomeOf o (\_ h -> swallow h)
      assertEq "return" Timeout a
      b <- outcomeOf o (\_ h -> swallow h >> throwIO Plain)
      assertEq "raise" Timeout b
      c <- outcomeOf o (\_ h -> swallow h >> exitWith ExitSuccess)
      assertEq "exit" Timeout c
      d <- outcomeOf o (\_ h -> tick h 100)
      assertEq "unswallowed" Timeout d
  , test "a swallowed memory breach is an oom, whatever the target does next" $ do
      let swallow h = reserve h (1 * 1048576 * 1024) `catch` \(_ :: BudgetSentinel) -> pure ()
      a <- outcomeOf fast (\_ h -> swallow h)
      assertEq "return" Oom a
      b <- outcomeOf fast (\_ h -> swallow h >> throwIO Plain)
      assertEq "raise" Oom b
      c <- outcomeOf fast (\_ h -> reserve h maxBound)
      assertEq "unswallowed" Oom c
  , test "sticky across kinds inside one execution: steps breach, then reserve still throws" $ do
      o <- outcomeOf fast {fzStepBudget = 1} $ \_ h -> do
        r1 <- try (tick h 10)
        r2 <- try (reserve h 1)
        case (r1, r2) of
          (Left (BudgetSentinel _ WhichSteps), Left (BudgetSentinel _ WhichSteps)) -> pure ()
          _ -> throwIO Plain
      assertEq "timeout, and the target saw both throws" Timeout o
  , test "a forged sentinel is a crash, never a timeout" $ do
      o <- outcomeOf fast (\_ _ -> throwIO (BudgetSentinel 1234 WhichSteps))
      assertEq "forged" (Crash "stale-budget-sentinel" []) o
      -- a retained handle used in a LATER execution throws the retired sentinel: a crash
      stash <- newIORef Nothing
      _ <- outcomeOf fast (\_ h -> writeIORef stash (Just h))
      o2 <- outcomeOf fast $ \_ _ -> readIORef stash >>= maybe (pure ()) (\old -> tick old 1)
      assertEq "stale" (Crash "stale-budget-sentinel" []) o2
  , test "a retained or forwarded handle cannot touch a later execution" $ do
      stash <- newIORef Nothing
      _ <- outcomeOf fast (\_ h -> writeIORef stash (Just h) >> edge h 1)
      late <- newEmptyMVar
      r <- exec1 fast $ \_ h -> do
        edge h 2
        old <- readIORef stash
        _ <- forkIO (forM_ old (\o -> edge o 99 >> tick o 1 `catch` \(_ :: BudgetSentinel) -> pure ()) >> putMVar late ())
        within 10 (takeMVar late)
      assertEq "pass" Pass (erOutcome r)
      assertEq "only this execution's edge" [2] (erEdges r)
  , test "a thread the target forks cannot affect the engine, only its own retired handle" $ do
      gate <- newEmptyMVar
      r <- exec1 fast $ \_ h -> void (forkIO (takeMVar gate >> edge h 77))
      assertEq "outcome" Pass (erOutcome r)
      putMVar gate ()
      threadDelay 20000
      assertEq "edges fixed at retirement" [] (erEdges r)
  , test "a masked non-allocating loop defers the kill: reported as a timeout, worker leaked, then released" $ do
      stop <- newIORef False
      r <- exec1 fast {fzTimeoutMs = 100} (\_ _ -> maskedSpin stop)
      -- the kill is deferred (masked); the engine still returns, bounded
      assertEq "timeout" Timeout (erOutcome r)
      assertTrue "leaked" (erLeaked r)
      writeIORef stop True
  , test "a campaign whose target always masks and spins aborts with workers-leaked, bounded, never a pass" $ do
      stop <- newIORef False
      t0 <- getMonotonicTime
      r <- run [B.pack [i] | i <- [1 .. 8]] fast {fzTimeoutMs = 50} (\_ _ -> maskedSpin stop)
      t1 <- getMonotonicTime
      writeIORef stop True
      assertEq "aborted at the leak limit" maxLeakedWorkers (rLeakedWorkers r)
      assertTrue "bounded time (first leak waits, the rest are short)" (t1 - t0 < 30)
      assertFalse "never clean" (isClean r)
      v <- verdict r fast {fzCi = CiOff} (\_ -> pure (Timeout, [])) "t"
      assertFalse "never passed" (fvPassed v)
      assertEq "reason" (Just "workers-leaked") (fvReason v)
      assertTrue "public line is a registered id" (any ("[fuzz-timeout:" `T.isInfixOf`) (fvLines v))
  , test "a target that crashes once and masks-and-spins on minimisation candidates leaks at most the cap" $ do
      stop <- newIORef False
      let seedIn = B.pack [1, 2, 3, 4, 5, 6, 7, 8]
          target dat _ = if dat == seedIn then throwIO Plain else maskedSpin stop
      t0 <- getMonotonicTime
      r <- run [seedIn] fast {fzTimeoutMs = 50} target
      t1 <- getMonotonicTime
      writeIORef stop True
      assertEq "exactly the cap, not more" maxLeakedWorkers (rLeakedWorkers r)
      assertTrue "bounded" (t1 - t0 < 30)
      assertFalse "never clean" (isClean r)
      assertEq "the crash is still reported" 1 (length (rCrashes r))
  , test "re-verification shares the leak cap: a leaked-out report re-verifies nothing" $ do
      calls <- newIORef (0 :: Int)
      let crash = CrashRecord "4eca7ae8" "panic" [] "xy" 1 True False
          report = FuzzReport 5 1 0 [crash, crash] 0 0 0 0 tierBlackBox 7 [] maxLeakedWorkers
          exec _ = modifyIORef' calls (+ 1) >> pure (Crash "panic" [], [])
      v <- verdict report fast exec "t"
      n <- readIORef calls
      assertEq "nothing executed" 0 n
      assertEq "all unverified" [Unverified, Unverified] (map csStatus (fvCrashes v))
      assertFalse "not passed" (fvPassed v)
  , test "re-verification stops at the cap when it leaks workers itself" $ do
      stop <- newIORef False
      leaks <- newIORef 0
      let crash = CrashRecord "4eca7ae8" "panic" [] "xy" 1 True False
          report = FuzzReport 5 1 0 (replicate 7 crash) 0 0 0 0 tierBlackBox 7 [] 0
          o = fast {fzTimeoutMs = 50}
      t0 <- getMonotonicTime
      v <- verdictTracked leaks report o (executorTracked leaks (\_ _ -> maskedSpin stop) o) "t"
      t1 <- getMonotonicTime
      writeIORef stop True
      n <- readIORef leaks
      assertEq "leaked exactly the cap" maxLeakedWorkers n
      assertEq "the rest are unverified" (replicate (7 - maxLeakedWorkers) Unverified) (drop maxLeakedWorkers (map csStatus (fvCrashes v)))
      assertTrue "bounded" (t1 - t0 < 30)
      assertFalse "not passed" (fvPassed v)
  , test "retained inputs never share a buffer with what the target was handed (each copy is load-bearing)" $ do
      let big = B.replicate 1048576 7
          slice = B.take 10 big
      assertTrue "the test input really is a slice" (inside slice big)
      -- admission copy
      r <- runOutcomes [slice] fast {fzMaxIterations = 0} (\_ -> pure (Pass, []))
      assertEq "one seed" 1 (length (rCorpus r))
      assertFalse "corpus entry is a copy" (any (`inside` big) (rCorpus r))
      -- admitted MUTATED candidates, cut to max_input_len, are copies of what the target saw
      -- (ONE iteration per run, so no later candidate can alias an admitted entry)
      admittedRuns <- forM [0 .. 40] $ \sd -> do
        seen <- newIORef []
        r2 <- runOutcomes [B.replicate 64 1] fast {fzMaxIterations = 1, fzMaxInputLen = 8, fzSeed = sd} $ \d -> do
          modifyIORef' seen (d :)
          pure (Pass, [toInteger (B.foldl' (\a x -> a * 31 + fromIntegral x) (7 :: Int) d `mod` 65536)])
        ds <- readIORef seen
        forM_ (drop 1 (rCorpus r2)) $ \e -> do
          assertTrue "admitted entry within max_input_len" (B.length e <= 8)
          assertFalse "admitted entry shares no buffer with the executed candidate" (any (inside e) ds)
        pure (length (rCorpus r2) - 1)
      assertTrue "some mutated candidate was admitted" (sum admittedRuns > 0)
      -- novelCrash copy: the minimised witness is a slice of the executed input unless copied
      firstIn <- newIORef Nothing
      let allSevens d = B.length d >= 10 && B.all (== 7) d
          probe d = do
            modifyIORef' firstIn (maybe (Just d) Just)
            pure (if allSevens d then Crash "K" [] else Pass, [])
      c <- runOutcomes [big] fast {fzMaxIterations = 1000} probe
      parent <- readIORef firstIn
      let kept = crMinimalInput (head (rCrashes c))
      assertTrue "it was minimised" (B.length kept >= 10 && B.length kept < 100)
      assertFalse "minimal witness is a copy" (maybe False (inside kept) parent)
      -- repeatCrash copy: a shorter repeat of the same signature replaces the witness
      let bigB = B.replicate 1048576 9
      c2 <- runOutcomes [B.replicate 8 1, B.take 3 bigB] fast {fzMaxIterations = 0, fzMinimizeBudget = 0} (\_ -> pure (Crash "K" [], []))
      assertEq "one signature" 1 (length (rCrashes c2))
      assertEq "shorter repeat kept" 3 (B.length (crMinimalInput (head (rCrashes c2))))
      assertFalse "repeat witness is a copy" (inside (crMinimalInput (head (rCrashes c2))) bigB)
  , test "a campaign is deterministic and finds the crash, minimised" $ do
      r1 <- run [B.pack [1, 2, 3]] fast crashy
      r2 <- run [B.pack [1, 2, 3]] fast crashy
      assertEq "iterations" 60 (rIterations r1)
      assertEq "deterministic corpus" (rCorpus r1) (rCorpus r2)
      assertEq "deterministic crashes" (map crSignature (rCrashes r1)) (map crSignature (rCrashes r2))
      assertEq "one crash" 1 (length (rCrashes r1))
      let c = head (rCrashes r1)
      assertEq "kind" "Plain" (crKind c)
      assertEq "minimal length" 7 (B.length (crMinimalInput c))
      assertEq "tier" tierReported (rCoverageTier r1)
      assertTrue "edges seen" (rTotalEdges r1 >= 2)
      assertFalse "not clean" (isClean r1)
  , test "a clean campaign without edges is black-box and clean" $ do
      r <- run [B.pack [9]] fast (\_ _ -> pure ())
      assertEq "tier" tierBlackBox (rCoverageTier r)
      assertEq "seeds" 1 (rAdmittedSeeds r)
      assertTrue "clean" (isClean r)
  , test "zero executions is never a pass" $ do
      r <- run [B.pack [9]] fast {fzMaxIterations = 0} (\_ _ -> pure ())
      assertEq "iterations" 0 (rIterations r)
      assertFalse "not clean" (isClean r)
      v <- verdict r fast (executor (\_ _ -> pure ()) fast) "t"
      assertFalse "verdict not passed" (fvPassed v)
      assertEq "reason" (Just "no_iterations") (fvReason v)
      -- a corpus of crashers is never mutated: also zero iterations
      c <- run [B.pack [1]] fast (\_ _ -> throwIO Plain)
      assertEq "crasher iterations" 0 (rIterations c)
      assertFalse "crasher corpus not clean" (isClean c)
      -- an empty corpus is replaced by one 00 seed
      e <- run [] fast (\_ _ -> pure ())
      assertEq "empty corpus seeded" [B.singleton 0] (take 1 (rCorpus e))
  , test "timeouts and ooms are counted, filed and fail the verdict" $ do
      r <- run [B.pack [1]] fast {fzMaxIterations = 3, fzStepBudget = 0} (\_ h -> tick h 1)
      assertEq "timeouts" 1 (rTimeouts r)
      assertEq "no mutation of a timing-out seed" 0 (rIterations r)
      v <- verdict r fast (executor (\_ h -> tick h 1) fast {fzStepBudget = 0}) "t"
      assertEq "reason" (Just "timeout") (fvReason v)
      assertEq "lines" ["not ok 1 - t [fuzz-timeout:4e61ea76]"] (fvLines v)
      o <- run [B.pack [1]] fast (\_ h -> reserve h maxBound)
      assertEq "ooms" 1 (rOoms o)
      vo <- verdict o fast (executor (\_ h -> reserve h maxBound) fast) "t"
      assertEq "oom reason" (Just "oom") (fvReason vo)
      assertEq "oom line" ["not ok 1 - t [fuzz-oom:7146a410]"] (fvLines vo)
  , test "verdict: crashes are re-verified as reproduced, flaky or unverified, never passed" $ do
      r <- run [B.pack [1, 2, 3]] fast crashy
      v <- verdict r fast (executor crashy fast) "fuzz_t"
      assertFalse "passed" (fvPassed v)
      assertEq "statuses" [Reproduced] (map csStatus (fvCrashes v))
      assertEq "reason" (Just "crash") (fvReason v)
      assertEq "line count" 1 (length (fvLines v))
      -- the crash no longer reproduces: still failing, labelled flaky
      f <- verdict r fast (executor (\_ _ -> pure ()) fast) "fuzz_t"
      assertEq "flaky" [Flaky] (map csStatus (fvCrashes f))
      assertFalse "flaky still fails" (fvPassed f)
      -- an input dropped by the retained cap cannot be re-verified
      let crashNonEmpty x = pure (if B.null x then Pass else Crash "K" [], [])
      d <- runOutcomesWith defaultEngineConfig {ecRetainedCap = 0} [B.pack [1]] fast crashNonEmpty
      vd <- verdict d fast crashNonEmpty "t"
      assertEq "unverified" [Unverified] (map csStatus (fvCrashes vd))
      assertEq "no witness kept" (Just []) (fvWitnesses vd)
  , test "CI mode (an unset mode counts as CI) carries no seed and no witness" $ do
      r <- run [B.pack [1, 2, 3]] fast crashy
      forM_ [CiOn, CiUnset] $ \m -> do
        v <- verdict r fast {fzCi = m} (executor crashy fast) "t"
        assertEq "seed" Nothing (fvSeed v)
        assertEq "witnesses" Nothing (fvWitnesses v)
        assertFalse "no input bytes in a line" (any (T.isInfixOf "0102") (fvLines v))
      v <- verdict r fast {fzCi = CiOff} (executor crashy fast) "t"
      assertEq "seed shown off CI" (Just 7) (fvSeed v)
      assertEq "one witness" 1 (maybe 0 length (fvWitnesses v))
      assertEq "show hides everything" "FuzzVerdict<passed=False reason=Just \"crash\" lines=1>" (show v)
  , test "caps: retained bytes, seeds, the input size and options are enforced before running" $ do
      -- a retained cap of 0: the first seed cannot even be admitted
      r <- runOutcomesWith defaultEngineConfig {ecRetainedCap = 0} [B.pack [1]] fast (\_ -> pure (Pass, [1]))
      assertEq "admission dropped" 1 (rAdmissionsDropped r)
      assertEq "no seeds" 0 (rAdmittedSeeds r)
      assertFalse "dropped is not clean" (isClean r)
      -- a cap that fits one seed: later admissions are dropped, retained never passes the cap
      r2 <- runOutcomesWith defaultEngineConfig {ecRetainedCap = 8} [B.pack [1, 2, 3]] fast {fzMaxIterations = 30} (\d -> pure (Pass, [toInteger (B.length d)]))
      assertTrue "bounded corpus" (sum (map B.length (rCorpus r2)) <= 8)
      assertThrows "oversized seed" isArg (runOutcomes [B.replicate (maxInputBytes + 1) 0] fast (\_ -> pure (Pass, [])))
      assertThrows "too many seeds" isArg (runOutcomes (replicate (maxInitialCorpus + 1) B.empty) fast (\_ -> pure (Pass, [])))
      assertThrows "bad options" isArg (runOutcomes [] fast {fzMaxIterations = -1} (\_ -> pure (Pass, [])))
      -- candidates never exceed max_input_len
      seen <- newIORef (0 :: Int)
      _ <- runOutcomes [B.replicate 40 1] fast {fzMaxInputLen = 16, fzMaxIterations = 200} $ \d -> do
        when (B.length d > 40) (modifyIORef' seen (+ 1))
        pure (Pass, [toInteger (B.length d)])
      big <- readIORef seen
      assertEq "no oversized candidate" 0 big
  , test "the minimisation budget is campaign-wide and counts rejected candidates" $ do
      calls <- newIORef (0 :: Int)
      -- crashes only while 60 or more bytes remain, so almost every candidate is REJECTED
      let crashLong d = modifyIORef' calls (+ 1) >> pure (if B.length d >= 60 then Crash "K" [] else Pass, [toInteger (B.length d)])
      r <- runOutcomes [B.replicate 64 9] fast {fzMaxIterations = 5, fzMinimizeBudget = 1000} crashLong
      n <- readIORef calls
      -- 1 triage execution + at most max_iterations (5) minimisation executions
      assertEq "triage plus allowance" 6 n
      assertEq "one crash" 1 (length (rCrashes r))
      assertFalse "cut short, not minimised" (crMinimized (head (rCrashes r)))
      -- rejected candidates are charged too: a probe that never reproduces
      calls2 <- newIORef (0 :: Int)
      let flaky d = do
            k <- atomicModifyIORef' calls2 (\x -> (x + 1, x))
            pure (if k == 0 then Crash "K" [] else Pass, [toInteger (B.length d)])
      _ <- runOutcomes [B.replicate 64 9] fast {fzMaxIterations = 4, fzMinimizeBudget = 1000} flaky
      n2 <- readIORef calls2
      assertEq "triage plus 4 rejected candidates" 5 n2
  , test "distinct crash signatures are capped and the rest counted as dropped" $ do
      let kinds d = Crash (T.pack ("K" ++ show (B.length d))) []
      r <- runOutcomes [B.replicate n 1 | n <- [1 .. maxCrashes + 5]] fast {fzMaxIterations = 0, fzMinimizeBudget = 0} (\d -> pure (kinds d, []))
      assertEq "kept" maxCrashes (length (rCrashes r))
      assertEq "dropped" 5 (rCrashesDropped r)
      assertEq "unique" (maxCrashes + 5) (uniqueCrashes r)
  , test "repeat crashes keep the smallest witness and count hits" $ do
      r <- runOutcomes [B.pack [1, 1, 1], B.pack [2, 2]] fast {fzMaxIterations = 0, fzMinimizeBudget = 0} (\_ -> pure (Crash "K" [], []))
      assertEq "one signature" 1 (length (rCrashes r))
      let c = head (rCrashes r)
      assertEq "hits" 2 (crHits c)
      assertEq "smallest kept" (B.pack [2, 2]) (crMinimalInput c)
  , test "every wait is bounded: the executor returns even for a target that blocks forever" $ do
      forever' <- newEmptyMVar :: IO (MVar ())
      o <- outcomeOf fast {fzTimeoutMs = 100} (\_ _ -> takeMVar forever')
      assertEq "timeout" Timeout o
      putMVar forever' () -- keeps the MVar reachable, so the RTS cannot report a deadlock
  , test "verdict reasons name the first failure class; derived instances behave" $ do
      let base = FuzzReport 5 1 0 [] 0 0 0 0 tierBlackBox 7 [] 0
          reasonOf r = fvReason <$> verdict r fast (\_ -> pure (Pass, [])) "t"
      a <- reasonOf base {rCrashesDropped = 2}
      assertEq "crashes dropped" (Just "crashes_dropped") a
      b <- reasonOf base {rAdmissionsDropped = 1}
      assertEq "admissions dropped" (Just "admissions_dropped") b
      c <- reasonOf base
      assertEq "clean" Nothing c
      assertEq "outcome show" "Crash \"K\" []" (show (Crash "K" []))
      assertEq "outcome eq" True (Timeout == Timeout && Oom /= Pass)
      assertEq "which" "WhichSteps" (show WhichSteps)
      assertEq "sentinel" "BudgetSentinel 1 WhichMem" (show (BudgetSentinel 1 WhichMem))
      assertEq "exhaustion" "ExMem" (show ExMem)
      assertEq "reverify" "Flaky" (show Flaky)
      assertEq "status eq" True (CrashStatus "a" "k" [] 1 Reproduced == CrashStatus "a" "k" [] 1 Reproduced)
      assertEq "status show" True (not (null (show (CrashStatus "a" "k" [] 1 Unverified))))
      assertEq "options eq" True (fast == fast && fast /= fast {fzSeed = 8})
  ]

