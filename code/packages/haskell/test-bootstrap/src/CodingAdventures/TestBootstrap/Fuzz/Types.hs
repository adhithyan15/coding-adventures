-- | Options, outcomes, reports and verdicts of the fuzz engine (VER06o 5.2, 6).
--
-- Every type that can hold a seed or a crash input has a hand-written 'Show'
-- that prints counts only: an input is PRIVATE (VER06 6.3) and must never reach
-- a log through @show@.
module CodingAdventures.TestBootstrap.Fuzz.Types
  ( FuzzOptions (..)
  , defaultFuzzOptions
  , validateFuzzOptions
  , Outcome (..)
  , CrashRecord (..)
  , FuzzReport (..)
  , isClean
  , uniqueCrashes
  , tierReported
  , tierBlackBox
  , Reverify (..)
  , CrashStatus (..)
  , FuzzVerdict (..)
  ) where

import Control.Monad (when)
import qualified Data.ByteString as B
import Data.Text (Text)
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.Error (argError)
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Pbt.Types (CiMode (..))

-- | Bounds of a campaign; every field is validated (CWE-400).
--
-- 'fzMinimizeBudget' is the most target executions spent minimising ONE novel
-- crash; on top of it the whole campaign spends at most 'fzMaxIterations'
-- minimisation executions.  'fzStepBudget' and 'fzMemBudget' are the
-- per-execution budgets of @tick@ and @reserve@; 'fzMemBudget' is also the
-- worker's ALLOCATION limit and 'fzTimeoutMs' the wall-clock kill.  An unset CI
-- mode is treated as CI (fails closed).
data FuzzOptions = FuzzOptions
  { fzMaxIterations :: !Int
  , fzSeed :: !Word64
  , fzMaxInputLen :: !Int
  , fzBaseEnergy :: !Int
  , fzMinimizeBudget :: !Int
  , fzStepBudget :: !Int
  , fzMemBudget :: !Int
  , fzTimeoutMs :: !Int
  , fzCi :: !CiMode
  }
  deriving (Eq)

instance Show FuzzOptions where
  show o = "FuzzOptions<max_iterations=" ++ show (fzMaxIterations o) ++ ">"

defaultFuzzOptions :: FuzzOptions
defaultFuzzOptions =
  FuzzOptions
    { fzMaxIterations = 1000
    , fzSeed = 42
    , fzMaxInputLen = 4096
    , fzBaseEnergy = 10
    , fzMinimizeBudget = defaultMinimizeBudget
    , fzStepBudget = defaultStepBudget
    , fzMemBudget = defaultMemBudget
    , fzTimeoutMs = defaultTimeoutMs
    , fzCi = CiUnset
    }

-- | Out-of-range options throw 'ArgumentError' instead of producing a verdict.
validateFuzzOptions :: FuzzOptions -> IO ()
validateFuzzOptions o = do
  range "max_iterations" (fzMaxIterations o) 0 maxIterations
  range "max_input_len" (fzMaxInputLen o) 0 maxInputBytes
  range "base_energy" (fzBaseEnergy o) 0 maxIterations
  range "minimize_budget" (fzMinimizeBudget o) 0 maxMinimizeBudget
  range "step_budget" (fzStepBudget o) 0 maxStepBudget
  range "mem_budget" (fzMemBudget o) 0 maxMemBudget
  range "timeout_ms" (fzTimeoutMs o) 1 maxTimeoutMs
  where
    range name v lo hi =
      when (v < lo || v > hi) (argError (name ++ " must be in [" ++ show lo ++ ", " ++ show hi ++ "]"))

-- | What one execution came to.  A crash is its kind and frames (frames are
-- always empty in this lane: GHC exceptions carry no portable stack).
data Outcome = Pass | Crash !Text [Text] | Timeout | Oom
  deriving (Eq, Show)

-- | One deduplicated crash.  'crMinimalInput' is PRIVATE: never in 'show'.
data CrashRecord = CrashRecord
  { crSignature :: !Text
  , crKind :: !Text
  , crFrames :: [Text]
  , crMinimalInput :: !B.ByteString
  , crHits :: !Int
  , crMinimized :: !Bool
  , crInputDropped :: !Bool
  }

instance Show CrashRecord where
  show r = "CrashRecord<" ++ show (crSignature r) ++ " " ++ show (crKind r) ++ " hits=" ++ show (crHits r) ++ ">"

tierReported, tierBlackBox :: Text
tierReported = "caller-reported"
tierBlackBox = "black-box-uninstrumented"

-- | What a campaign found.  Every execution lands in exactly one bucket:
-- 'rIterations' counts MUTATION executions; crashes, timeouts and ooms count
-- triage and mutation executions; anything that passed is in the corpus or was
-- redundant.  The seed and corpus never appear in 'show'.
data FuzzReport = FuzzReport
  { rIterations :: !Int
  , rAdmittedSeeds :: !Int
  , rTotalEdges :: !Int
  , rCrashes :: [CrashRecord]
  , rTimeouts :: !Int
  , rOoms :: !Int
  , rCrashesDropped :: !Int
  , rAdmissionsDropped :: !Int
  , rCoverageTier :: !Text
  , rSeed :: !Word64
  , rCorpus :: [B.ByteString]
  , rLeakedWorkers :: !Int
  -- ^ workers that never answered the kill (masked loops); a campaign that leaks
  -- 'maxLeakedWorkers' of them is aborted and is never clean
  }

instance Show FuzzReport where
  show r =
    "FuzzReport<iterations=" ++ show (rIterations r) ++ " seeds=" ++ show (rAdmittedSeeds r)
      ++ " edges=" ++ show (rTotalEdges r) ++ " crashes=" ++ show (uniqueCrashes r)
      ++ " timeouts=" ++ show (rTimeouts r) ++ " ooms=" ++ show (rOoms r) ++ ">"

-- | Can this be read as "the campaign found nothing"?  Only if at least one
-- mutation iteration ran and nothing crashed, timed out, ran out of memory or was
-- dropped.  ZERO EXECUTIONS IS NEVER A PASS.
isClean :: FuzzReport -> Bool
isClean r =
  rIterations r > 0 && null (rCrashes r) && rTimeouts r == 0 && rOoms r == 0
    && rCrashesDropped r == 0
    && rAdmissionsDropped r == 0
    && rLeakedWorkers r == 0

uniqueCrashes :: FuzzReport -> Int
uniqueCrashes r = length (rCrashes r) + rCrashesDropped r

-- | The re-verification of a crash witness: the signature came back
-- ('Reproduced'), did not ('Flaky'; still failing, fail closed) or no input was
-- kept ('Unverified').  Never "passed".
data Reverify = Reproduced | Flaky | Unverified
  deriving (Eq, Show)

data CrashStatus = CrashStatus
  { csSignature :: !Text
  , csKind :: !Text
  , csFrames :: [Text]
  , csHits :: !Int
  , csStatus :: !Reverify
  }
  deriving (Eq, Show)

-- | The honest, CI-safe summary of a report.  In CI mode ('CiOn' or 'CiUnset')
-- 'fvSeed' and 'fvWitnesses' are 'Nothing' and 'fvLines' carry only the closed
-- failure id and the 8-hex signature.
data FuzzVerdict = FuzzVerdict
  { fvPassed :: !Bool
  , fvReason :: !(Maybe Text)
  , fvLines :: [Text]
  , fvCrashes :: [CrashStatus]
  , fvIterations :: !Int
  , fvTimeouts :: !Int
  , fvOoms :: !Int
  , fvSeed :: !(Maybe Word64)
  , fvWitnesses :: !(Maybe [(Text, B.ByteString)])
  }

instance Show FuzzVerdict where
  show v = "FuzzVerdict<passed=" ++ show (fvPassed v) ++ " reason=" ++ show (fvReason v) ++ " lines=" ++ show (length (fvLines v)) ++ ">"
