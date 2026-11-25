-- | The deterministic, budgeted coverage-guided campaign (VER06 5 and 9.1,
-- VER06o 6).
--
-- A campaign is a function of @(corpus, options, exec)@: triage every seed, then
-- repeatedly pick a seed, give it an energy-scaled number of mutated children, run
-- each child and either admit it (new coverage), record it (crash, timeout, oom) or
-- drop it (redundant).  The engine itself reads no clock, file or environment; only
-- the executor forks threads.
--
-- @exec@ is a function @ByteString -> IO (Outcome, [Integer])@: the outcome and the
-- trace (the edges) of that execution.
--
-- Per iteration the stream draws: @bounded (len corpus)@ for the seed; per energy
-- slot @1 + bounded 4@ steps, one @nextU64@ sub-seed, then, if there is more than
-- one seed, @bounded 4@ and on 0 a donor index and two split draws.  That order is
-- the cross-lane contract.
module CodingAdventures.TestBootstrap.Fuzz.Engine
  ( Exec
  , EngineConfig (..)
  , defaultEngineConfig
  , minimize
  , sameCrash
  , minimizeCrashingInput
  , runOutcomes
  , runOutcomesWith
  ) where

import Control.Monad (foldM, forM_, when)
import qualified Data.ByteString as B
import qualified Data.IntMap.Strict as IM
import qualified Data.Map.Strict as M
import Data.Text (Text)

import CodingAdventures.TestBootstrap.Error (argError)
import CodingAdventures.TestBootstrap.Fuzz.Bitmap
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Fuzz.Mutate (mutateSequence, splice)
import CodingAdventures.TestBootstrap.Fuzz.Signature (crashSignature)
import CodingAdventures.TestBootstrap.Fuzz.Types
import CodingAdventures.TestBootstrap.Prng (Rng, bounded, newRng, nextU64)

type Exec = B.ByteString -> IO (Outcome, [Integer])

-- | Test seams: the retained-bytes cap and the coverage-tier labeller (given the
-- total number of covered edges).  Production code uses 'defaultEngineConfig'.
data EngineConfig = EngineConfig
  { ecRetainedCap :: !Int
  , ecTier :: !(Maybe (Int -> IO Text))
  }

defaultEngineConfig :: EngineConfig
defaultEngineConfig = EngineConfig {ecRetainedCap = maxRetainedBytes, ecTier = Nothing}

-- ---------------------------------------------------------------------------
-- Minimisation (VER06 5.3)
-- ---------------------------------------------------------------------------

sameCrash :: Outcome -> Text -> Bool
sameCrash (Crash kind frames) sig = crashSignature kind frames == sig
sameCrash _ _ = False

-- | Shrinks @dat@ while it keeps the EXACT signature.  Returns
-- @(bytes, executions_spent, finished)@.  EVERY candidate execution, accepted or
-- rejected, is counted and the budget is checked before each one, so no stream of
-- rejected candidates can run past it.
--
-- Pass 1 deletes blocks, halving the block size.  It starts at @max 1 (len div 2)@
-- (not @len div 2@), which is what makes the empty input reachable from a 1-byte
-- crasher; the chunk sizes tried for a 9-byte input are 4, 2, 1.  Pass 2 zeroes
-- individual non-essential bytes.  @probe@ runs one candidate.
minimize :: B.ByteString -> Text -> (B.ByteString -> IO Outcome) -> Int -> IO (B.ByteString, Int, Bool)
minimize dat sig probe budget
  | B.null dat = pure (dat, 0, True)
  | otherwise = deletePass dat (max 1 (B.length dat `div` 2)) 0 0
  where
    deletePass cur chunk offset spent
      | chunk == 0 = zeroPass cur 0 spent
      | offset >= size = deletePass cur (chunk `div` 2) 0 spent
      | spent >= budget = pure (cur, spent, False)
      | otherwise = do
          let stop = offset + min chunk (size - offset)
              candidate = B.take offset cur <> B.drop stop cur
          out <- probe candidate
          if sameCrash out sig
            then deletePass candidate chunk offset (spent + 1) -- same offset: the tail slid left
            else deletePass cur chunk (offset + chunk) (spent + 1)
      where
        size = B.length cur
    zeroPass cur i spent
      | i >= B.length cur = pure (cur, spent, True)
      | B.index cur i == 0 = zeroPass cur (i + 1) spent
      | spent >= budget = pure (cur, spent, False)
      | otherwise = do
          let candidate = B.take i cur <> B.singleton 0 <> B.drop (i + 1) cur
          out <- probe candidate
          if sameCrash out sig
            then zeroPass candidate (i + 1) (spent + 1)
            else zeroPass cur (i + 1) (spent + 1)

-- | Shrinks @dat@ while it keeps the exact signature; @budget@ caps executions.
minimizeCrashingInput :: B.ByteString -> Text -> Int -> (B.ByteString -> IO Outcome) -> IO B.ByteString
minimizeCrashingInput dat sig budget probe = do
  when (budget < 0 || budget > maxMinimizeBudget) (argError ("budget must be in [0, " ++ show maxMinimizeBudget ++ "]"))
  when (B.length dat > maxInputBytes) (argError ("data exceeds " ++ show maxInputBytes ++ " bytes"))
  (out, _, _) <- minimize dat sig probe budget
  pure out

-- ---------------------------------------------------------------------------
-- The campaign
-- ---------------------------------------------------------------------------

data Entry = Entry {eData :: !B.ByteString, eNewEdges :: !Int, eGeneration :: !Int}

data St = St
  { sCum :: !Bitmap
  , sCorpus :: !(IM.IntMap Entry)
  , sSize :: !Int
  , sCrashes :: !(M.Map Text CrashRecord)
  , sOrder :: [Text] -- newest first
  , sTimeouts :: !Int
  , sOoms :: !Int
  , sDropped :: !Int
  , sAdmDropped :: !Int
  , sMinLeft :: !Int
  , sRetained :: !Int
  }

runOutcomes :: [B.ByteString] -> FuzzOptions -> Exec -> IO FuzzReport
runOutcomes = runOutcomesWith defaultEngineConfig

-- | Runs a bounded coverage-guided campaign over @exec@.  Contracts (VER06 9.1),
-- each pinned by a test:
--
-- * a function of (corpus, options, exec): same inputs, same report;
-- * with a live corpus @iterations == max_iterations@ exactly, and no mutated
--   candidate exceeds @max_input_len@ (initial seeds run as given);
-- * an empty corpus is replaced by one @00@ seed; seeds that crash, time out or
--   exhaust memory are never mutated, so a corpus of crashers fuzzes nothing;
-- * every execution lands in crashes, timeouts, ooms or the corpus; 'isClean'
--   needs at least one iteration and no failure at all;
-- * one 'CrashRecord' per signature; a novel crash is minimised.
runOutcomesWith :: EngineConfig -> [B.ByteString] -> FuzzOptions -> Exec -> IO FuzzReport
runOutcomesWith cfg initialCorpus o exec = do
  validateFuzzOptions o
  when (length (take (maxInitialCorpus + 1) initialCorpus) > maxInitialCorpus) (argError ("initial corpus exceeds " ++ show maxInitialCorpus ++ " seeds"))
  forM_ initialCorpus $ \s ->
    when (B.length s > maxInputBytes) (argError ("an initial corpus seed exceeds " ++ show maxInputBytes ++ " bytes"))
  rng <- newRng (fzSeed o)
  let seeds = if null initialCorpus then [B.singleton 0] else initialCorpus
      st0 =
        St
          { sCum = emptyBitmap
          , sCorpus = IM.empty
          , sSize = 0
          , sCrashes = M.empty
          , sOrder = []
          , sTimeouts = 0
          , sOoms = 0
          , sDropped = 0
          , sAdmDropped = 0
          , sMinLeft = fzMaxIterations o
          , sRetained = 0
          }
  -- 1. Triage: run every seed once.
  st1 <- foldM (\st dat -> triage st dat) st0 seeds
  -- 2. The mutation loop.
  (iterations, st2) <- mutateLoop rng st1 0
  let total = count (sCum st2)
  tier <- case ecTier cfg of
    Just f -> f total
    Nothing -> pure (if total > 0 then tierReported else tierBlackBox)
  pure
    FuzzReport
      { rIterations = iterations
      , rAdmittedSeeds = sSize st2
      , rTotalEdges = total
      , rCrashes = [r | sig <- reverse (sOrder st2), Just r <- [M.lookup sig (sCrashes st2)]]
      , rTimeouts = sTimeouts st2
      , rOoms = sOoms st2
      , rCrashesDropped = sDropped st2
      , rAdmissionsDropped = sAdmDropped st2
      , rCoverageTier = tier
      , rSeed = fzSeed o
      , rCorpus = [eData e | e <- IM.elems (sCorpus st2)]
      }
  where
    triage st dat = do
      (out, es) <- exec dat
      case out of
        Pass -> pure (admitSeed st dat 0 es)
        _ -> fileOutcome st out dat

    -- Puts a passing input in the corpus iff it covered a new edge.  The first
    -- passing seed is always admitted, even with no coverage, so there is
    -- something to mutate in black-box mode.
    admitSeed st dat generation es =
      let (hasNew, countNew, cum) = admit (sCum st) es
          st' = st {sCum = cum}
          firstSeed = generation == 0 && sSize st == 0
       in if not (hasNew || firstSeed)
            then st'
            else
              if sRetained st' + B.length dat > ecRetainedCap cfg
                then st' {sAdmDropped = sAdmDropped st' + 1}
                else
                  st'
                    { sCorpus = IM.insert (sSize st') (Entry dat countNew generation) (sCorpus st')
                    , sSize = sSize st' + 1
                    , sRetained = sRetained st' + B.length dat
                    }

    fileOutcome st Timeout _ = pure st {sTimeouts = sTimeouts st + 1}
    fileOutcome st Oom _ = pure st {sOoms = sOoms st + 1}
    fileOutcome st (Crash kind frames) dat = recordCrash st kind frames dat
    fileOutcome st Pass _ = pure st

    recordCrash st kind frames dat = do
      let sig = crashSignature kind frames
      case M.lookup sig (sCrashes st) of
        Just r -> pure (repeatCrash st sig r dat)
        Nothing -> novelCrash st sig kind frames dat

    repeatCrash st sig r dat
      | not (crInputDropped r) && B.length dat < B.length (crMinimalInput r) =
          let saved = B.length (crMinimalInput r) - B.length dat
              r' = r {crHits = crHits r + 1, crMinimalInput = dat}
           in st {sCrashes = M.insert sig r' (sCrashes st), sRetained = sRetained st - saved}
      | otherwise = st {sCrashes = M.insert sig r {crHits = crHits r + 1} (sCrashes st)}

    novelCrash st sig kind frames dat
      | M.size (sCrashes st) >= maxCrashes = pure st {sDropped = sDropped st + 1}
      | otherwise = do
          let allowance = min (fzMinimizeBudget o) (sMinLeft st)
              -- The minimiser's re-run: coverage is discarded and nothing is counted.
              probe d = fst <$> exec d
          (kept, spent, finished) <- minimize dat sig probe allowance
          let over = sRetained st + B.length kept > ecRetainedCap cfg
              (kept', dropped, retained) =
                if over then (B.empty, True, sRetained st) else (kept, False, sRetained st + B.length kept)
              r =
                CrashRecord
                  { crSignature = sig
                  , crKind = kind
                  , crFrames = frames
                  , crMinimalInput = kept'
                  , crHits = 1
                  , crMinimized = finished && not dropped
                  , crInputDropped = dropped
                  }
          pure
            st
              { sCrashes = M.insert sig r (sCrashes st)
              , sOrder = sig : sOrder st
              , sMinLeft = sMinLeft st - spent
              , sRetained = retained
              }

    mutateLoop :: Rng -> St -> Int -> IO (Int, St)
    mutateLoop rng st iterations
      | iterations >= fzMaxIterations o || sSize st == 0 = pure (iterations, st)
      | otherwise = do
          idx <- fromInteger <$> bounded rng (toInteger (sSize st))
          let entry = sCorpus st IM.! idx
              energy = calculateEnergy (fzBaseEnergy o) (B.length (eData entry)) (eNewEdges entry) (count (sCum st))
          (iterations', st') <- slots rng st entry energy 0 iterations
          mutateLoop rng st' iterations'

    slots :: Rng -> St -> Entry -> Int -> Int -> Int -> IO (Int, St)
    slots rng st entry energy slot iterations
      | slot >= energy || iterations >= fzMaxIterations o = pure (iterations, st)
      | otherwise = do
          n <- bounded rng 4
          sub <- nextU64 rng
          candidate0 <- mutateSequence (eData entry) (toInteger sub) (1 + n) maxInputBytes
          candidate1 <- maybeSplice rng st candidate0
          let candidate = B.take (fzMaxInputLen o) candidate1
          (out, es) <- exec candidate
          st' <- case out of
            Pass -> pure (admitSeed st candidate (eGeneration entry + 1) es)
            _ -> fileOutcome st out candidate
          slots rng st' entry energy (slot + 1) (iterations + 1)

    maybeSplice rng st candidate
      | sSize st > 1 = do
          pick <- bounded rng 4
          if pick == 0
            then do
              di <- fromInteger <$> bounded rng (toInteger (sSize st))
              let donor = eData (sCorpus st IM.! di)
              tSplit <- bounded rng (toInteger (B.length candidate) + 1)
              dSplit <- bounded rng (toInteger (B.length donor) + 1)
              pure (splice candidate donor tSplit dSplit)
            else pure candidate
      | otherwise = pure candidate
