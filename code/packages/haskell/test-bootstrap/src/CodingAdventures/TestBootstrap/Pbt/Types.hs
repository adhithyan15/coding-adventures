-- | Limits, the closed failure vocabulary, options, verdicts and the
-- sabotage-hook configuration (VER05n 3, 6, 8).
module CodingAdventures.TestBootstrap.Pbt.Types
  ( -- * Limits (CWE-400: every loop and allocation is finite)
    maxCases
  , maxShrinkStepsLimit
  , maxInputBytes
  , maxListLen
  , maxGenCount
  , maxFrequencyTotal
  , maxShrinkWork
  , maxRetriesLimit
  , defaultMaxRetries
  , maxBudgetDepth
  , maxCheckNesting
  , maxIntBits
  , attemptBudget
  , checkSteps
    -- * Failure vocabulary
  , failureCounterexample
  , failureExhausted
  , failureTimeout
  , failurePanic
  , failureIds
  , isFailureId
    -- * Options and verdicts
  , CiMode (..)
  , ciFromLookup
  , Options (..)
  , defaultOptions
  , validateOptions
  , Verdict (..)
    -- * Configuration (sabotage seams, passed down, never global)
  , Config (..)
  , defaultConfig
  ) where

import Control.Monad (when)
import Data.Char (isSpace, toLower)
import Data.Text (Text)
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Prng

maxCases, maxShrinkStepsLimit, maxInputBytes, maxListLen, maxGenCount :: Int
maxCases = 100000
maxShrinkStepsLimit = 10000
maxInputBytes = 1048576
maxListLen = 100000
maxGenCount = 1000000

maxFrequencyTotal :: Integer
maxFrequencyTotal = 4294967296

-- | Most elements one whole shrink may copy, summed over every attempt of
-- every nested shrinker.
maxShrinkWork :: Int
maxShrinkWork = 64 * 1024 * 1024

maxRetriesLimit, defaultMaxRetries, maxBudgetDepth, maxCheckNesting, maxIntBits :: Int
maxRetriesLimit = 10000
defaultMaxRetries = 100
maxBudgetDepth = 64
maxCheckNesting = 64
-- | Widest integer span a generator will draw (CWE-400 on the k-word loop).
maxIntBits = 16384

-- | The attempt budget of one shrink: @16 * max_steps + 64@.  A rejected
-- attempt copies its candidate but is not an accepted step, so @max_steps@
-- alone does not bound the work.  A negative step count is clamped so it cannot
-- turn into a tiny or wrapped cap.
attemptBudget :: Int -> Int
attemptBudget maxSteps = 16 * max 0 maxSteps + 64

-- | @0 <= max_steps <= 10,000@ or an 'ArgumentError'.
checkSteps :: Int -> IO ()
checkSteps n = when (n < 0 || n > maxShrinkStepsLimit) (argError ("max_steps " ++ show n ++ " outside [0, " ++ show maxShrinkStepsLimit ++ "]"))

failureCounterexample, failureExhausted, failureTimeout, failurePanic :: Text
failureCounterexample = "property-counterexample"
failureExhausted = "property-exhausted"
failureTimeout = "property-timeout"
failurePanic = "property-panic"

failureIds :: [Text]
failureIds = [failureCounterexample, failureExhausted, failureTimeout, failurePanic]

-- | True only for the four registered ids.
isFailureId :: Text -> Bool
isFailureId = (`elem` failureIds)

-- | @CiUnset@ is treated as CI: the engine fails closed and redacts.
data CiMode = CiOn | CiOff | CiUnset deriving (Eq, Show)

-- | Pure interpretation of the @CI@ environment variable (the caller reads
-- it): unset, empty, @0@ or @false@ (trimmed, any case) is off, anything else on.
ciFromLookup :: Maybe String -> CiMode
ciFromLookup Nothing = CiOff
ciFromLookup (Just v) =
  let t = map toLower (trim v)
   in if null t || t == "0" || t == "false" then CiOff else CiOn
  where
    trim = reverse . dropWhile isSpace . reverse . dropWhile isSpace

data Options = Options
  { optNumTests :: !Int
  , optMaxShrinkSteps :: !Int
  , optCi :: !CiMode
  , optConfig :: Config
  }

defaultOptions :: Options
defaultOptions = Options {optNumTests = 100, optMaxShrinkSteps = 1000, optCi = CiUnset, optConfig = defaultConfig}

-- | Invalid options throw 'ArgumentError' instead of producing a verdict.
validateOptions :: Options -> IO ()
validateOptions o = do
  when (optNumTests o < 0 || optNumTests o > maxCases) (argError ("num_tests " ++ show (optNumTests o) ++ " outside [0, " ++ show maxCases ++ "]"))
  checkSteps (optMaxShrinkSteps o)

-- | The outcome of a check.  Redacted (CI) verdicts carry no seed, no
-- counterexample and no minimal value.  Every field is built from already
-- rendered text, so reading one can never throw.
data Verdict = Verdict
  { vPassed :: !Bool
  , vFailureId :: !(Maybe Text)
  , vMessage :: !(Maybe Text)
  , vCasesRun :: !Int
  , vIteration :: !Int
  , vShrinkSteps :: !Int
  , vSeed :: !(Maybe Word64)
  , vCounterexample :: !(Maybe Text)
  , vMinimal :: !(Maybe Text)
  , vDigest :: !(Maybe Text)
  }
  deriving (Eq, Show)

-- | The test seams.  A value, passed down through the budget tree (never a
-- global), so a negative control swaps one field, runs the corpus, and expects
-- problems.  Production code uses 'defaultConfig'.
data Config = Config
  { cfgBisect :: Integer -> Integer -> Integer
  -- ^ candidate from @(current, diff)@: @current - floor(diff / 2)@
  , cfgUnit :: Integer -> Integer -> Integer
  -- ^ candidate from @(current, diff)@: @current - signum diff@
  , cfgSkipDeletion :: !Bool
  , cfgNewRng :: Word64 -> IO Rng
  , cfgMaxWork :: !Int
  -- ^ work cap of a ROOT scope
  }

defaultConfig :: Config
defaultConfig =
  Config
    { cfgBisect = \cur diff -> cur - (diff `div` 2) -- div FLOORS on negatives
    , cfgUnit = \cur diff -> cur - signum diff
    , cfgSkipDeletion = False
    , cfgNewRng = newRng
    , cfgMaxWork = maxShrinkWork
    }
