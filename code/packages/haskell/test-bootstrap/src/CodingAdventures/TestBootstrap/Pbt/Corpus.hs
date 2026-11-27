-- | The production runner for the shared pbt-v1 corpus
-- (@code/specs/fixtures/pbt-v1/cases.json@, VER05 section 6, VER05n 8).
--
-- It takes the parsed JSON, strictly decodes every case, evaluates it with THIS
-- package's own public engine functions and returns human-readable problems.  An
-- empty list means "conforms".  It does no file I/O (the package is
-- capability-free); the test that reads the committed fixture lives in the test
-- tree.
--
-- Why a runner that can fail matters: a conformance check that cannot report a
-- problem is worse than none.  So the runner is strict:
--
-- * a case must have exactly the fields of its op, with canonical encodings;
-- * an unknown op, unknown property name or malformed value is a PROBLEM, never
--   a silent skip;
-- * every op and all four failure ids must appear, an unregistered id must be
--   rejected, and the number of cases must equal 25, so deleting cases is itself
--   a problem;
-- * every problem about a case starts with that case's (escaped, clipped) id.
module CodingAdventures.TestBootstrap.Pbt.Corpus
  ( Report (..)
  , parseCorpus
  , evaluate
  , evaluateWith
  , pbtV1CaseCount
  , maxCorpusCases
  , maxProblems
  ) where

import Control.Exception (throwIO)
import Control.Monad (foldM, when)
import qualified Data.ByteString as B
import Data.IORef
import Data.List (sort)
import qualified Data.Map.Strict as M
import qualified Data.Set as Set
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.CorpusJson.Canon (sameVal)
import CodingAdventures.TestBootstrap.CorpusJson.Escape (clipText, escapeText)
import CodingAdventures.TestBootstrap.CorpusJson.StrictJson
import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.CorpusJson.Values
import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Pbt.Gen
import CodingAdventures.TestBootstrap.Pbt.Runner
import CodingAdventures.TestBootstrap.Pbt.Safe
import CodingAdventures.TestBootstrap.Pbt.Shrink
import CodingAdventures.TestBootstrap.Pbt.Types
import qualified CodingAdventures.TestBootstrap.Tfv as Tfv

data Report = Report {reportEvaluated :: !Int, reportProblems :: [Text]}
  deriving (Eq, Show)

pbtV1CaseCount, maxCorpusCases, maxProblems, maxGenBytes :: Int
pbtV1CaseCount = 25
maxCorpusCases = 10000
maxProblems = 1000
-- | Bounds the total bytes one gen case may generate and hold (CWE-400).
maxGenBytes = 1048576

-- | Strictly decodes a pbt-v1 document into its (undecoded) case list: exactly
-- @{"schema_version": 1, "cases": [...]}@.
parseCorpus :: B.ByteString -> Either String [Json]
parseCorpus raw = case parse raw of
  Left (JsonError e) -> Left e
  Right (JObject fields)
    | length fields == 2 -> case (lookup "schema_version" fields, lookup "cases" fields) of
        (Just (JNumber "1"), Just (JArray cases)) -> Right cases
        (Just (JNumber "1"), _) -> Left "pbt corpus: \"cases\" must be an array"
        _ -> Left "pbt corpus: schema_version must be 1"
  Right _ -> Left "pbt corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}"

-- | The closed field set of each op (besides id and op).
shapes :: [(Text, [Text])]
shapes =
  [ ("gen_primitive", ["type", "seed", "count", "outputs"])
  , ("gen_bounded", ["min", "max", "seed", "count", "outputs"])
  , ("gen_bytes", ["min_len", "max_len", "seed", "count", "outputs"])
  , ("shrink_int", ["initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"])
  , ("shrink_refinement", ["initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"])
  , ("shrink_list", ["initial", "elem_target", "property", "expected_minimal", "expected_steps"])
  , ("shrink_bytes", ["initial", "property", "expected_minimal", "expected_steps"])
  , ("replay_consistency", ["seed", "property", "initial_fail", "expected_minimal"])
  , ("filter_exhausted", ["max_retries", "verdict", "failure_id"])
  , ("failure_id_valid", ["input", "valid"])
  , ("corpus_reject", ["text"])
  ]

intProps :: [(Text, Integer -> Bool)]
intProps =
  [ ("int_le_100", (<= 100))
  , ("int_in_range_neg10_50", \x -> x >= -10 && x <= 50)
  , ("int_is_even", even)
  , ("int_abs_le_10", \x -> abs x <= 10)
  , ("refinement_x_le_50", (<= 50))
  ]

listProps :: [(Text, [Integer] -> Bool)]
listProps =
  [ ("list_no_consecutive_duplicates", \l -> and (zipWith (/=) l (drop 1 l)))
  , ("list_sum_le_20", \l -> sum l <= 20)
  ]

bytesProps :: [(Text, B.ByteString -> Bool)]
bytesProps =
  [ ("bytes_len_lt_5", \b -> B.length b < 5)
  , ("bytes_no_zero_byte", B.notElem 0)
  ]

-- | One case's outcome.
data CaseResult = CaseResult
  { crProblems :: [Text]
  , crRanOk :: Bool
  , crCandidate :: Text
  , crRegistered :: Bool
  }

-- | Evaluates a case list with the default configuration and 25 expected cases.
evaluate :: [Json] -> IO Report
evaluate = evaluateWith defaultConfig (Just pbtV1CaseCount)

-- | Evaluates every case.  @Nothing@ skips the case-count check (ad-hoc case
-- lists in tests).  The configuration carries the negative-control sabotage seams.
evaluateWith :: Config -> Maybe Int -> [Json] -> IO Report
evaluateWith cfg expected casesIn = do
  let countProblem = case expected of
        Just n | length casesIn /= n ->
          ["pbt-v1: expected " <> tshow n <> " cases, got " <> tshow (length casesIn)]
        _ -> []
      tooMany = length (take (maxCorpusCases + 1) casesIn) > maxCorpusCases
      capProblem = ["pbt-v1: more than " <> tshow maxCorpusCases <> " cases exceeds the cap of " <> tshow maxCorpusCases | tooMany]
      cases = take maxCorpusCases casesIn
      idProblem = checkIds cases
  (ops, regs, rejected, evaluated, problems) <- foldM step (Set.empty, Set.empty, False, 0 :: Int, []) cases
  let missingOps = [op | (op, _) <- shapes, not (Set.member op ops)]
      missingIds = [i | i <- failureIds, not (Set.member i regs)]
      final =
        countProblem
          ++ capProblem
          ++ idProblem
          ++ reverse problems
          ++ ["pbt-v1: operations never exercised: " <> T.intercalate ", " (sort missingOps) | not (null missingOps)]
          ++ ["pbt-v1: registered failure ids missing: " <> T.intercalate ", " missingIds | not (null missingIds)]
          ++ ["pbt-v1: failure_id_valid needs an unregistered-id rejection" | not rejected]
  pure (Report evaluated (take maxProblems (map (clipText maxProblemLen) final)))
  where
    step (ops, regs, rejected, n, probs) raw = case raw of
      JObject fields -> case lookup "id" fields of
        Just (JString idb) | not (B.null idb) -> do
          let cid = escapeText False (clipText 100 (TE.decodeUtf8 idb))
          caseStep cid fields (ops, regs, rejected, n, probs)
        _ -> do
          shown <- showJson raw
          pure (ops, regs, rejected, n, ("case missing a string id: " <> shown) : probs)
      _ -> do
        shown <- showJson raw
        pure (ops, regs, rejected, n, ("case is not an object: " <> shown) : probs)
    caseStep cid fields (ops, regs, rejected, n, probs) = do
      let opText = case lookup "op" fields of
            Just (JString o) -> TE.decodeUtf8 o
            _ -> ""
      case lookup opText shapes of
        Nothing -> pure (ops, regs, rejected, n, (cid <> ": unknown op " <> escapeText True (clipText 60 opText)) : probs)
        Just want ->
          let wantKeys = sort (["id", "op"] ++ want)
              haveKeys = sort (map (TE.decodeUtf8 . fst) fields)
           in if haveKeys /= wantKeys
                then pure (ops, regs, rejected, n, (cid <> ": fields " <> escapeText True (clipText 120 (T.intercalate "," haveKeys)) <> " != expected " <> T.intercalate "," wantKeys) : probs)
                else do
                  res <- evaluateCase cfg cid opText fields
                  let probs' = reverse (crProblems res) ++ probs
                  if not (crRanOk res)
                    then pure (ops, regs, rejected, n, probs')
                    else do
                      let (regs', rejected') =
                            if opText == "failure_id_valid"
                              then
                                if crRegistered res
                                  then (Set.insert (crCandidate res) regs, rejected)
                                  else (regs, True)
                              else (regs, rejected)
                      pure (Set.insert opText ops, regs', rejected', n + 1, probs')

-- | Duplicate or missing ids stop the id scan at the first offender.
checkIds :: [Json] -> [Text]
checkIds = go Set.empty
  where
    go _ [] = []
    go seen (c : cs) = case c of
      JObject fields -> case lookup "id" fields of
        Just (JString i)
          | not (B.null i) && not (Set.member i seen) -> go (Set.insert i seen) cs
        _ -> ["duplicate or missing case id"]
      _ -> ["duplicate or missing case id"]

tshow :: Show a => a -> Text
tshow = T.pack . show

showJson :: Json -> IO Text
-- a lazy bounded prefix: a malformed node is never fully materialised
showJson j = pure (escapeText True (clipText maxFragmentLen (T.pack (take (maxFragmentLen + 1) (show j)))))

-- | Decodes and checks one case.  Any exception (a malformed case or an engine
-- that threw) is reported as a problem for that case; asynchronous ones propagate.
evaluateCase :: Config -> Text -> Text -> [(B.ByteString, Json)] -> IO CaseResult
evaluateCase cfg cid op fields = do
  r <- tryRecoverable (decodeAll >>= run)
  case r of
    Right res -> pure res
    Left e -> do
      m <- safeMessage e
      pure
        CaseResult
          { crProblems = [cid <> ": " <> op <> " could not be evaluated: " <> exceptionTypeName e <> ": " <> clipText maxFragmentLen m]
          , crRanOk = False
          , crCandidate = ""
          , crRegistered = False
          }
  where
    decodeAll :: IO (M.Map Text Val)
    decodeAll = do
      pairs <- mapM decodeField [(TE.decodeUtf8 k, v) | (k, v) <- fields, k /= "id", k /= "op"]
      pure (M.fromList pairs)
    decodeField (k, v) = case decode v of
      Left msg -> throwIO (ArgumentError msg)
      Right val -> pure (k, val)

    run :: M.Map Text Val -> IO CaseResult
    run v = case op of
      _ | op `elem` ["gen_primitive", "gen_bounded", "gen_bytes"] -> done <$> checkGen cfg cid op v
      _ | op `elem` ["shrink_int", "shrink_refinement"] -> done <$> checkShrinkInt cfg cid op v
      "shrink_list" -> done <$> checkShrinkList cfg cid v
      "shrink_bytes" -> done <$> checkShrinkBytes cfg cid v
      "replay_consistency" -> done <$> checkReplay cfg cid v
      "filter_exhausted" -> done <$> checkFilterExhausted cfg cid v
      "failure_id_valid" -> do
        let input = case M.lookup "input" v of
              Just (VText t) -> Just t
              _ -> Nothing
            got = maybe False isFailureId input
        ok <- sameOk (VBool got) (M.lookup "valid" v)
        pure
          CaseResult
            { crProblems = [cid <> ": failure id membership disagrees" | not ok]
            , crRanOk = True
            , crCandidate = maybe "" id input
            , crRegistered = got
            }
      _ -> do
        -- corpus_reject
        text <- case M.lookup "text" v of
          Just (VText t) -> pure t
          _ -> throwIO (ArgumentError "text must be a string")
        pure
          CaseResult
            { crProblems = [cid <> ": corpus_reject text was accepted by the strict parser" | Tfv.valid (TE.encodeUtf8 text)]
            , crRanOk = True
            , crCandidate = ""
            , crRegistered = False
            }
    done ps = CaseResult ps True "" False

sameOk :: Val -> Maybe Val -> IO Bool
sameOk got = pure . maybe False (sameVal got)

caseError :: String -> IO a
caseError = throwIO . ArgumentError

intOf :: String -> Maybe Val -> IO Integer
intOf _ (Just (VInt i)) = pure i
intOf what _ = caseError (what ++ " must be an int")

-- | An int that drives a loop or an allocation: @0 <= n <= 10000@.
sizeOf' :: String -> Maybe Val -> IO Int
sizeOf' what mv = do
  n <- intOf what mv
  when (n < 0 || n > toInteger maxSize) (caseError (what ++ " " ++ show n ++ " outside [0, " ++ show maxSize ++ "]"))
  pure (fromInteger n)

seedOf :: Maybe Val -> IO Word64
seedOf mv = do
  n <- intOf "seed" mv
  -- A fixture seed is a u64: refused outside [0, 2^64-1], never reduced modulo 2^64.
  when (n < 0 || n > 18446744073709551615) (caseError ("seed " ++ show (clipShow n) ++ " outside [0, 2^64-1]"))
  pure (fromInteger n)
  where
    clipShow :: Integer -> String
    clipShow n = let t = show n in if length t > 24 then take 24 t ++ "..." else t

propOf :: Text -> [(Text, p)] -> String -> Maybe Val -> IO p
propOf op table what mv = case mv of
  Just (VText name) | Just p <- lookup name table -> pure p
  _ -> caseError ("unknown property for " ++ T.unpack op ++ " (" ++ what ++ ")")

show' :: Val -> IO Text
show' = showVal

mismatch :: Text -> Text -> Val -> Maybe Val -> IO [Text]
mismatch cid label got expected = do
  ok <- sameOk got expected
  if ok
    then pure []
    else do
      g <- show' got
      e <- maybe (pure "<absent>") show' expected
      pure [cid <> ": " <> label <> ": got " <> g <> ", corpus " <> e]

checkGen :: Config -> Text -> Text -> M.Map Text Val -> IO [Text]
checkGen cfg cid op v = do
  seed <- seedOf (M.lookup "seed" v)
  count <- sizeOf' "count" (M.lookup "count" v)
  -- The expected outputs are in hand already: a case whose list length is not
  -- count cannot conform, so refuse it before generating anything.
  expectedOut <- case M.lookup "outputs" v of
    Just o@(VList xs) | length xs == count -> pure o
    _ -> caseError ("outputs must be a list of exactly count (" ++ show count ++ ") values")
  got <- case op of
    "gen_primitive" -> case M.lookup "type" v of
      Just (VText ty) -> genPrimitive cfg seed ty count
      _ -> caseError "type must be a string"
    "gen_bounded" -> do
      lo <- intOf "min" (M.lookup "min" v)
      hi <- intOf "max" (M.lookup "max" v)
      genBounded cfg seed lo hi count
    _ -> do
      minLen <- sizeOf' "min_len" (M.lookup "min_len" v)
      maxLen <- sizeOf' "max_len" (M.lookup "max_len" v)
      when (count * maxLen > maxGenBytes) (caseError ("count " ++ show count ++ " x max_len " ++ show maxLen ++ " exceeds the generation budget of " ++ show maxGenBytes ++ " bytes"))
      genBytes cfg seed minLen maxLen count
  ok <- pure (sameVal (VList got) expectedOut)
  pure [cid <> ": " <> op <> " outputs mismatch" | not ok]

checkShrinkInt :: Config -> Text -> Text -> M.Map Text Val -> IO [Text]
checkShrinkInt cfg cid op v = do
  prop <- propOf op intProps "int" (M.lookup "property" v)
  initial <- intOf "initial" (M.lookup "initial" v)
  target <- intOf "target" (M.lookup "target" v)
  lo <- intOf "min" (M.lookup "min" v)
  hi <- intOf "max" (M.lookup "max" v)
  (minimal, steps) <- shrinkIntWith cfg Nothing initial target lo hi 1000 (pure . prop)
  p1 <- mismatch cid (op <> " minimal mismatch") (VInt minimal) (M.lookup "expected_minimal" v)
  p2 <- mismatch cid (op <> " step count mismatch") (VInt (toInteger steps)) (M.lookup "expected_steps" v)
  -- Engine invariants, independent of the corpus's expected values.
  let p3 = [cid <> ": " <> op <> " minimal does not falsify the property" | prop minimal]
      p4 = [cid <> ": " <> op <> " minimal left [min, max]" | minimal < lo || minimal > hi]
  pure (p1 ++ p2 ++ p3 ++ p4)

checkShrinkList :: Config -> Text -> M.Map Text Val -> IO [Text]
checkShrinkList cfg cid v = do
  prop <- propOf "shrink_list" listProps "list" (M.lookup "property" v)
  initial <- case M.lookup "initial" v of
    Just (VList xs) -> mapM (intOf "element" . Just) xs
    _ -> caseError "initial must be a list of ints"
  elemTarget <- intOf "elem_target" (M.lookup "elem_target" v)
  (minimal, steps) <- shrinkListWith cfg Nothing initial elemTarget 1000 (pure . prop)
  p1 <- mismatch cid "shrink_list minimal mismatch" (VList (map VInt minimal)) (M.lookup "expected_minimal" v)
  p2 <- mismatch cid "shrink_list steps mismatch" (VInt (toInteger steps)) (M.lookup "expected_steps" v)
  pure (p1 ++ p2 ++ [cid <> ": shrink_list minimal does not falsify the property" | prop minimal])

checkShrinkBytes :: Config -> Text -> M.Map Text Val -> IO [Text]
checkShrinkBytes cfg cid v = do
  prop <- propOf "shrink_bytes" bytesProps "bytes" (M.lookup "property" v)
  initial <- case M.lookup "initial" v of
    Just (VBytes b) -> pure b
    _ -> caseError "initial must be bytes"
  (minimal, steps) <- shrinkBytesWith cfg Nothing initial 1000 (pure . prop)
  p1 <- mismatch cid "shrink_bytes minimal mismatch" (VBytes minimal) (M.lookup "expected_minimal" v)
  p2 <- mismatch cid "shrink_bytes steps mismatch" (VInt (toInteger steps)) (M.lookup "expected_steps" v)
  pure (p1 ++ p2 ++ [cid <> ": shrink_bytes minimal does not falsify the property" | prop minimal])

checkReplay :: Config -> Text -> M.Map Text Val -> IO [Text]
checkReplay cfg cid v = do
  seed <- seedOf (M.lookup "seed" v)
  prop <- propOf "replay_consistency" intProps "int" (M.lookup "property" v)
  (initialFail, minimal) <- simulateReplayWith cfg Nothing seed (pure . prop)
  p1 <- mismatch cid "replay initial_fail mismatch" (VInt initialFail) (M.lookup "initial_fail" v)
  p2 <- mismatch cid "replay minimal mismatch" (VInt minimal) (M.lookup "expected_minimal" v)
  pure (p1 ++ p2)

checkFilterExhausted :: Config -> Text -> M.Map Text Val -> IO [Text]
checkFilterExhausted cfg cid v = do
  maxRetries <- sizeOf' "max_retries" (M.lookup "max_retries" v)
  when (maxRetries < 1) (caseError "max_retries must be at least 1")
  calls <- newIORef (0 :: Int)
  let gen = filterM maxRetries (\_ -> modifyIORef' calls (+ 1) >> pure False) (int 0 10)
      opts = defaultOptions {optNumTests = 1, optMaxShrinkSteps = 1000, optCi = CiOn, optConfig = cfg}
  verdict <- check 0 gen opts (\(_ :: Integer) -> pure True)
  n <- readIORef calls
  let verdictOk = maybe False (sameVal (VText "fail")) (M.lookup "verdict" v)
      p1 = [cid <> ": an unsatisfiable filter must be a fail verdict" | not verdictOk || vPassed verdict]
      want = M.lookup "failure_id" v
  p2 <- case vFailureId verdict of
    Just fid -> mismatch cid "failure id" (VText fid) want
    Nothing -> pure [cid <> ": no failure id"]
  let p3 = [cid <> ": filter sampled " <> tshow n <> " times, expected " <> tshow maxRetries | n /= maxRetries]
  pure (p1 ++ p2 ++ p3)

