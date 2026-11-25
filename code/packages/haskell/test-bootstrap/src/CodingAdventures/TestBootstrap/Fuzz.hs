-- | The Haskell coverage-guided fuzz engine (VER06o): the public facade.
--
-- > import CodingAdventures.TestBootstrap.Fuzz
-- > import qualified CodingAdventures.TestBootstrap.Fuzz.Handle as H
-- >
-- > main = do
-- >   report <- run [B.pack [0, 0]] defaultFuzzOptions {fzMaxIterations = 500, fzSeed = 7} $ \dat h -> do
-- >     H.edge h (H.edgeId "entered")
-- >     when (dat == B.pack [0xFF, 0xFF]) (throwIO (userError "boom"))
-- >   print (isClean report)
--
-- Every execution of the target runs in its own thread with a fresh 'Handle' (see
-- "CodingAdventures.TestBootstrap.Fuzz.Executor").  Coverage is what the target
-- reports through 'H.edge' (VER06o 4.2: HPC is not used in the hot path).
module CodingAdventures.TestBootstrap.Fuzz
  ( -- * Running a campaign
    run
  , runOutcomes
  , executor
  , verdict
  , verdictFor
  , verdictTracked
  , executorTracked
    -- * Re-exports
  , Target
  , Outcome (..)
  , FuzzOptions (..)
  , defaultFuzzOptions
  , FuzzReport (..)
  , CrashRecord (..)
  , FuzzVerdict (..)
  , CrashStatus (..)
  , Reverify (..)
  , isClean
  , uniqueCrashes
  , tierReported
  , tierBlackBox
  , corpusAdmit
  , calculateEnergy
  , tapFailures
  , formatTapFailure
  , crashSignature
  ) where

import Control.Monad (foldM)
import qualified Data.ByteString as B
import Data.IORef
import Data.Text (Text)

import CodingAdventures.TestBootstrap.Fuzz.Bitmap (calculateEnergy, corpusAdmit)
import CodingAdventures.TestBootstrap.Fuzz.Engine
import CodingAdventures.TestBootstrap.Fuzz.Executor
import CodingAdventures.TestBootstrap.Fuzz.Limits (maxCrashes, maxLeakedWorkers)
import CodingAdventures.TestBootstrap.Fuzz.Signature (crashSignature, formatTapFailure, tapFailures)
import CodingAdventures.TestBootstrap.Fuzz.Types
import CodingAdventures.TestBootstrap.Pbt.Types (CiMode (..))

-- | The outcome-level executor for @target@: @ByteString -> IO (Outcome, [Integer])@,
-- one worker thread per call, each with a PRIVATE leak counter.  It does NOT enforce
-- the leak cap across calls: to re-verify a report with the cap enforced use
-- 'verdictFor' (or 'verdictTracked' with 'executorTracked' on one shared counter).
executor :: Target -> FuzzOptions -> Exec
executor target o dat = do
  r <- runExecution target dat o
  pure (erOutcome r, erEdges r)

-- | 'executor' that counts leaked workers in a shared counter (see
-- 'verdictTracked').
executorTracked :: IORef Int -> Target -> FuzzOptions -> Exec
executorTracked leaks target o dat = do
  r <- runExecutionTracked leaks target dat o
  pure (erOutcome r, erEdges r)

-- | Runs a campaign against @target@.  Any synchronous exception escaping it is a
-- crash.  The coverage tier is @caller-reported@ if the target called @edge@
-- during the campaign and @black-box-uninstrumented@ otherwise.
run :: [B.ByteString] -> FuzzOptions -> Target -> IO FuzzReport
run initialCorpus o target = do
  calls <- newIORef (0 :: Int)
  leaks <- newIORef (0 :: Int)
  let exec dat = do
        r <- runExecutionTracked leaks target dat o
        modifyIORef' calls (+ erEdgeCalls r)
        pure (erOutcome r, erEdges r)
      tier _ = do
        n <- readIORef calls
        pure (if n > 0 then tierReported else tierBlackBox)
  runOutcomesWith
    defaultEngineConfig
      { ecTier = Just tier
      , ecAbort = Just ((>= maxLeakedWorkers) <$> readIORef leaks)
      , ecLeaked = Just (readIORef leaks)
      }
    initialCorpus
    o
    exec

-- | 'verdict' for a target, with the re-verification leak cap ENFORCED: the executor
-- and the verdict share ONE leak counter ('executorTracked' + 'verdictTracked').
-- This is the call to use.
verdictFor :: Target -> FuzzReport -> FuzzOptions -> Text -> IO FuzzVerdict
verdictFor target report o targetId = do
  leaks <- newIORef 0
  verdictTracked leaks report o (executorTracked leaks target o) targetId

-- | The honest verdict of a report (VER06o 6).  With an ARBITRARY @exec@ (including
-- 'executor') only the REPORT's leak count is enforced (a report that already leaked
-- the cap re-verifies nothing); leaks during re-verification are not counted.  Only
-- 'verdictFor', or 'verdictTracked' with 'executorTracked', enforces the cap while
-- re-verifying.
--
-- Before a crash is reported its minimal input is run once more through @exec@:
-- the crash is 'Reproduced' if the signature comes back and 'Flaky' if it does not
-- (it still fails the verdict, fail closed); 'Unverified' when no input was kept.
-- @passed@ is true only for a clean report (so zero executions never pass).  With
-- 'CiOn' or 'CiUnset' the verdict carries no seed and no witness.
verdict :: FuzzReport -> FuzzOptions -> Exec -> Text -> IO FuzzVerdict
verdict report o exec targetId = do
  leaks <- newIORef 0
  verdictTracked leaks report o exec targetId

-- | 'verdict' whose re-verification shares ONE leak counter with the executor
-- ('executorTracked' on the same 'IORef'), with the campaign's cap: once the report's
-- leaked workers plus those leaked while re-verifying reach 'maxLeakedWorkers', the
-- remaining witnesses are 'Unverified' and nothing more is executed.
verdictTracked :: IORef Int -> FuzzReport -> FuzzOptions -> Exec -> Text -> IO FuzzVerdict
verdictTracked leaks report o exec targetId = do
  let redact = fzCi o /= CiOff
      clean = isClean report
  statuses <- foldM (\acc r -> (\s -> acc ++ [s]) <$> reverify r) [] (take maxCrashes (rCrashes report))
  ls <- tapFailures report targetId 1
  pure
    FuzzVerdict
      { fvPassed = clean
      , fvReason = if clean then Nothing else Just (reason report)
      , fvLines = ls
      , fvCrashes = statuses
      , fvIterations = rIterations report
      , fvTimeouts = rTimeouts report
      , fvOoms = rOoms report
      , fvSeed = if redact then Nothing else Just (rSeed report)
      , fvWitnesses = if redact then Nothing else Just [(crSignature r, crMinimalInput r) | r <- rCrashes report, not (crInputDropped r)]
      }
  where
    reverify r = do
      now <- readIORef leaks
      status <-
        if crInputDropped r || rLeakedWorkers report + now >= maxLeakedWorkers
          then pure Unverified
          else do
            (out, _) <- exec (crMinimalInput r)
            pure (if sameCrash out (crSignature r) then Reproduced else Flaky)
      pure CrashStatus {csSignature = crSignature r, csKind = crKind r, csFrames = crFrames r, csHits = crHits r, csStatus = status}

reason :: FuzzReport -> Text
reason r
  | rLeakedWorkers r >= maxLeakedWorkers = "workers-leaked"
  | not (null (rCrashes r)) = "crash"
  | rTimeouts r > 0 = "timeout"
  | rOoms r > 0 = "oom"
  | rCrashesDropped r > 0 = "crashes_dropped"
  | rAdmissionsDropped r > 0 = "admissions_dropped"
  | rLeakedWorkers r > 0 = "workers-leaked"
  | otherwise = "no_iterations"
