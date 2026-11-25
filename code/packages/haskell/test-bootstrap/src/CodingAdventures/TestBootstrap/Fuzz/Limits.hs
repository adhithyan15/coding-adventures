-- | The CWE-400 ceilings of the fuzz engine (VER06o 3 and 6).
--
-- A fuzz campaign is attacker-reachable work (a CI job fed a hostile
-- configuration or fixture), so every count and length that drives a loop or an
-- allocation has a hard ceiling, and each one is checked BEFORE the allocation
-- it bounds.
module CodingAdventures.TestBootstrap.Fuzz.Limits
  ( maxIterations
  , maxInputBytes
  , maxCrashes
  , maxInitialCorpus
  , maxStepBudget
  , maxMemBudget
  , maxMinimizeBudget
  , defaultMinimizeBudget
  , defaultStepBudget
  , defaultMemBudget
  , maxTimeoutMs
  , defaultTimeoutMs
  , killWaitMs
  , leakedKillWaitMs
  , maxLeakedWorkers
  , maxRetainedBytes
  , maxSequenceWork
  , corpusSequenceWork
  , maxFrames
  , maxKindLen
  , maxTargetLen
  , coverageMapSize
  , allocationFloor
  ) where

-- | Most mutation iterations one campaign may run, and most steps one
-- @mutateSequence@ call may take.
maxIterations :: Int
maxIterations = 1000000

-- | Largest input (a seed, a candidate, a sequence input): 1 MiB.
maxInputBytes :: Int
maxInputBytes = 1048576

-- | Distinct crash signatures kept; more are counted in @crashesDropped@.
maxCrashes :: Int
maxCrashes = 1000

maxInitialCorpus :: Int
maxInitialCorpus = 10000

maxStepBudget, maxMemBudget, maxMinimizeBudget :: Int
maxStepBudget = 100000000
maxMemBudget = 1073741824
maxMinimizeBudget = 100000

defaultMinimizeBudget, defaultStepBudget, defaultMemBudget :: Int
defaultMinimizeBudget = 2000
defaultStepBudget = 200000
-- | 256 MiB of ALLOCATION per execution.  GHC's per-thread limit counts total
-- bytes allocated, not residency (VER06o 5.1), so the default is larger than
-- the Elixir lane's 64 MiB of heap.
defaultMemBudget = 268435456

maxTimeoutMs, defaultTimeoutMs, killWaitMs :: Int
maxTimeoutMs = 60000
defaultTimeoutMs = 5000
-- | How long the engine waits for a killed worker to finish before it leaks it.
killWaitMs = 2000

-- | After the FIRST leaked worker the engine waits only this long (500 ms) for later
-- ones (a masked spinner never answers, so every full wait is wasted).  "Leaked"
-- means "did not answer the kill within the wait": a legitimate target whose cleanup
-- takes longer and that times out 'maxLeakedWorkers' times is aborted, fail closed.
-- A campaign that has leaked 'maxLeakedWorkers' workers is aborted with the honest
-- @workers-leaked@ failure (VER06o 5.4).
leakedKillWaitMs, maxLeakedWorkers :: Int
leakedKillWaitMs = 500
maxLeakedWorkers = 4

-- | Corpus seeds plus crash inputs kept by one campaign: 256 MiB.
maxRetainedBytes :: Int
maxRetainedBytes = 268435456

-- | Bytes one @mutateSequence@ call may copy (@2^28@), and the aggregate the
-- corpus runner allows over all its cases (@2^24@).
maxSequenceWork, corpusSequenceWork :: Int
maxSequenceWork = 268435456
corpusSequenceWork = 16777216

maxFrames, maxKindLen, maxTargetLen, coverageMapSize :: Int
maxFrames = 64
maxKindLen = 64
-- | The longest target token, in BYTES (VER06 9.3).
maxTargetLen = 128
coverageMapSize = 65536

-- | The smallest allocation limit a worker is given (1 MiB), whatever the
-- memory budget: the allocation counter is accurate to only about 4 KiB.
allocationFloor :: Int
allocationFloor = 1048576
