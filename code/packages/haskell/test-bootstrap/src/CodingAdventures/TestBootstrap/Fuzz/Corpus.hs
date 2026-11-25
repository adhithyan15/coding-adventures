-- | The production runner for the shared fuzz-v1 corpus
-- (@code/specs/fixtures/fuzz-v1/cases.json@, VER06 8, VER06o 8).
--
-- It takes the parsed JSON, strictly decodes every case, evaluates it with THIS
-- package's own engine functions (through 'Hooks', so a test can sabotage one) and
-- returns human-readable problems.  An empty list means "conforms".  It does no
-- file I/O; the test that reads the committed fixture lives in the test tree.
--
-- A runner that cannot report a problem is worse than none, so this one is strict:
--
-- * a case has exactly the fields of its op, in canonical encodings;
-- * an unknown op or a malformed value is a PROBLEM, never a silent skip;
-- * every op appears, a registered failure id is accepted and an unregistered one
--   rejected, and the case count is 25, so deleting cases is itself a problem;
-- * fixture-controlled sizes are capped (CWE-400): a hostile @steps = 10^9@ is a
--   reported problem, not a hang, and the aggregate @mutate_sequence@ work of the
--   whole corpus is capped at 2^24 bytes ACTUALLY copied (a breach zeroes the
--   allowance, so every later case fails at once);
-- * every problem about a case starts with that case's id, ESCAPED.
module CodingAdventures.TestBootstrap.Fuzz.Corpus
  ( Report (..)
  , parseFuzzCorpus
  , evaluate
  , evaluateWith
  , fuzzV1CaseCount
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

import CodingAdventures.TestBootstrap.CorpusJson.Escape (clipText, escapeText)
import CodingAdventures.TestBootstrap.CorpusJson.StrictJson
import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.CorpusJson.Values
import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Fuzz.FailureIds (allFailureIds)
import CodingAdventures.TestBootstrap.Fuzz.Hooks
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Fuzz.Mutate (Endian, SeqResult (..), parseEndian)
import CodingAdventures.TestBootstrap.Pbt.Corpus (Report (..))
import CodingAdventures.TestBootstrap.Pbt.Safe (exceptionTypeName, safeMessage, tryRecoverable)
import CodingAdventures.TestBootstrap.Util (hexLower)

fuzzV1CaseCount, maxCorpusCases, maxProblems :: Int
fuzzV1CaseCount = 25
maxCorpusCases = 10000
maxProblems = 1000

-- | Strictly decodes a fuzz-v1 document into its (undecoded) case list: exactly
-- @{"schema_version": 1, "cases": [...]}@.
parseFuzzCorpus :: B.ByteString -> Either String [Json]
parseFuzzCorpus raw = case parse raw of
  Left (JsonError e) -> Left e
  Right (JObject fields)
    | length fields == 2 -> case (lookup "schema_version" fields, lookup "cases" fields) of
        (Just (JNumber "1"), Just (JArray cases)) -> Right cases
        (Just (JNumber "1"), _) -> Left "fuzz corpus: \"cases\" must be an array"
        _ -> Left "fuzz corpus: schema_version must be 1"
  Right _ -> Left "fuzz corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}"

-- | The closed field set of each op (besides id and op).
shapes :: [(Text, [Text])]
shapes =
  [ ("mutate_bit_flip", ["input", "bit_offset", "count", "output"])
  , ("mutate_byte_flip", ["input", "byte_offset", "count", "output"])
  , ("mutate_arithmetic", ["input", "byte_offset", "delta", "width", "endian", "output"])
  , ("mutate_boundary", ["input", "byte_offset", "width", "val", "endian", "output"])
  , ("mutate_block_delete", ["input", "offset", "length", "output"])
  , ("mutate_block_insert", ["input", "offset", "insert_bytes", "output"])
  , ("mutate_block_duplicate", ["input", "src_offset", "length", "dst_offset", "output"])
  , ("mutate_splice", ["target", "donor", "target_split", "donor_split", "output"])
  , ("mutate_sequence", ["input", "seed", "steps", "output"])
  , ("corpus_admit", ["existing_edges", "trace_edges", "admitted", "new_edges"])
  , ("crash_dedup", ["crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate"])
  , ("failure_id_valid", ["input", "valid"])
  , ("private_reporting", ["target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload"])
  , ("corpus_reject", ["text"])
  ]

data CaseResult = CaseResult
  { crProblems :: [Text]
  , crRanOk :: Bool
  , crAccepted :: Bool
  }

evaluate :: [Json] -> IO Report
evaluate = evaluateWith defaultHooks (Just fuzzV1CaseCount)

-- | Evaluates every case.  @Nothing@ skips the case-count check (ad-hoc case lists
-- in tests).  The hooks carry the negative-control sabotage seams.
evaluateWith :: Hooks -> Maybe Int -> [Json] -> IO Report
evaluateWith hooks expected casesIn = do
  work <- newIORef corpusSequenceWork
  let countProblem = case expected of
        Just n | length casesIn /= n -> ["fuzz-v1: expected " <> tshow n <> " cases, got " <> tshow (length casesIn)]
        _ -> []
      tooMany = length (take (maxCorpusCases + 1) casesIn) > maxCorpusCases
      capProblem = ["fuzz-v1: more than " <> tshow maxCorpusCases <> " cases exceeds the cap of " <> tshow maxCorpusCases | tooMany]
      cases = take maxCorpusCases casesIn
      idProblem = checkIds cases
  (ops, accepted, rejected, evaluated, problems) <- foldM (step work) (Set.empty, False, False, 0 :: Int, []) cases
  let missingOps = [op | (op, _) <- shapes, not (Set.member op ops)]
      unregistered = [i | i <- allFailureIds, not (hkFailureId hooks i)]
      final =
        countProblem
          ++ capProblem
          ++ idProblem
          ++ reverse problems
          ++ ["fuzz-v1: operations never exercised: " <> T.intercalate ", " (sort missingOps) | not (null missingOps)]
          ++ ["fuzz-v1: failure_id_valid needs a registered-id acceptance" | not accepted]
          ++ ["fuzz-v1: failure_id_valid needs an unregistered-id rejection" | not rejected]
          ++ ["fuzz-v1: engine rejects registered ids " <> T.intercalate ", " unregistered | not (null unregistered)]
  pure (Report evaluated (take maxProblems (map (clipText maxProblemLen) final)))
  where
    step work (ops, acc, rej, n, probs) raw = case raw of
      JObject fields -> case lookup "id" fields of
        Just (JString idb) | not (B.null idb) -> do
          let cid = escapeText False (clipText 100 (TE.decodeUtf8 idb))
          caseStep work cid fields (ops, acc, rej, n, probs)
        _ -> pure (ops, acc, rej, n, ("case missing a string id: " <> showJson raw) : probs)
      _ -> pure (ops, acc, rej, n, ("case is not an object: " <> showJson raw) : probs)
    caseStep work cid fields (ops, acc, rej, n, probs) = do
      let opText = case lookup "op" fields of
            Just (JString o) -> TE.decodeUtf8 o
            _ -> ""
      case lookup opText shapes of
        Nothing -> pure (ops, acc, rej, n, (cid <> ": unknown op " <> escapeText True (clipText 60 opText)) : probs)
        Just want ->
          let wantKeys = sort (["id", "op"] ++ want)
              haveKeys = sort (map (TE.decodeUtf8 . fst) fields)
           in if haveKeys /= wantKeys
                then pure (ops, acc, rej, n, (cid <> ": fields " <> escapeText True (clipText 120 (T.intercalate "," haveKeys)) <> " != expected " <> T.intercalate "," wantKeys) : probs)
                else do
                  res <- evaluateCase hooks work cid opText fields
                  let probs' = reverse (crProblems res) ++ probs
                  if not (crRanOk res)
                    then pure (ops, acc, rej, n, probs')
                    else do
                      let (acc', rej')
                            | opText == "failure_id_valid" = if crAccepted res then (True, rej) else (acc, True)
                            | otherwise = (acc, rej)
                      pure (Set.insert opText ops, acc', rej', n + 1, probs')

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

-- | A bounded rendering of a malformed node (never fully materialised).
showJson :: Json -> Text
showJson j = escapeText True (clipText maxFragmentLen (T.pack (take (maxFragmentLen + 1) (show j))))

evaluateCase :: Hooks -> IORef Int -> Text -> Text -> [(B.ByteString, Json)] -> IO CaseResult
evaluateCase hooks work cid op fields = do
  r <- tryRecoverable (decodeAll >>= run)
  case r of
    Right res -> pure res
    Left e -> do
      m <- safeMessage e
      pure
        CaseResult
          { crProblems = [cid <> ": " <> op <> " could not be evaluated: " <> exceptionTypeName e <> ": " <> clipText maxFragmentLen m]
          , crRanOk = False
          , crAccepted = False
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
      "corpus_admit" -> done <$> checkCorpusAdmit hooks cid v
      "crash_dedup" -> done <$> checkCrashDedup hooks cid v
      "private_reporting" -> done <$> checkPrivate hooks cid v
      "failure_id_valid" -> do
        let input = case M.lookup "input" v of
              Just (VText t) -> Just t
              _ -> Nothing
            got = maybe False (hkFailureId hooks) input
        pure
          CaseResult
            { crProblems = [cid <> ": failure id membership disagrees" | not (sameBool got (M.lookup "valid" v))]
            , crRanOk = True
            , crAccepted = got
            }
      "corpus_reject" -> do
        text <- case M.lookup "text" v of
          Just (VText t) -> pure t
          _ -> caseError "text must be a string"
        pure (done [cid <> ": corpus_reject text was accepted by the strict parser" | hkTfvValid hooks (TE.encodeUtf8 text)])
      _ -> done <$> checkMutator hooks work cid op v
    done ps = CaseResult ps True False

sameBool :: Bool -> Maybe Val -> Bool
sameBool got (Just (VBool b)) = got == b
sameBool _ _ = False

caseError :: String -> IO a
caseError = throwIO . ArgumentError

intOf :: String -> Maybe Val -> IO Integer
intOf _ (Just (VInt i)) = pure i
intOf what _ = caseError (what ++ " must be an int")

-- | An int that drives a loop or an allocation: @0 <= n <= 10000@.
sizeOf :: String -> Maybe Val -> IO Int
sizeOf what mv = do
  n <- intOf what mv
  when (n < 0 || n > toInteger maxSize) (caseError (what ++ " " ++ show n ++ " outside [0, " ++ show maxSize ++ "]"))
  pure (fromInteger n)

bytesOf :: String -> Maybe Val -> IO B.ByteString
bytesOf _ (Just (VBytes b)) = pure b
bytesOf what _ = caseError (what ++ " must be bytes")

textOf :: String -> Maybe Val -> IO Text
textOf _ (Just (VText t)) = pure t
textOf what _ = caseError (what ++ " must be a string")

boolOf :: String -> Maybe Val -> IO Bool
boolOf _ (Just (VBool b)) = pure b
boolOf what _ = caseError (what ++ " must be a bool")

endianOf :: Maybe Val -> IO Endian
endianOf mv = do
  t <- textOf "endian" mv
  maybe (caseError "endian must be 'little' or 'big'") pure (parseEndian (T.unpack t))

textsOf :: String -> Maybe Val -> IO [Text]
textsOf _ (Just (VList xs)) | Just ts <- mapM asText xs = pure ts
  where
    asText (VText t) = Just t
    asText _ = Nothing
textsOf what _ = caseError (what ++ " must be a list of strings")

-- | Edge ids are u64: @0 <= e < 2^64@.
edgesOf :: String -> Maybe Val -> IO [Integer]
edgesOf what (Just (VList xs)) = mapM one xs
  where
    one (VInt e)
      | e >= 0 && e < 2 ^ (64 :: Int) = pure e
      | otherwise = caseError (what ++ " element outside [0, 2^64)")
    one _ = caseError (what ++ " must be a list of ints")
edgesOf what _ = caseError (what ++ " must be a list of ints")

hexClip :: B.ByteString -> Text
hexClip = clipText maxFragmentLen . T.pack . hexLower

checkMutator :: Hooks -> IORef Int -> Text -> Text -> M.Map Text Val -> IO [Text]
checkMutator hooks work cid op v = do
  let f = M.lookup
  inputM <- pure (M.lookup "input" v)
  got <- case op of
    "mutate_bit_flip" -> hkBitFlip hooks <$> bytesOf "input" inputM <*> intOf "bit_offset" (f "bit_offset" v) <*> intOf "count" (f "count" v)
    "mutate_byte_flip" -> hkByteFlip hooks <$> bytesOf "input" inputM <*> intOf "byte_offset" (f "byte_offset" v) <*> intOf "count" (f "count" v)
    "mutate_arithmetic" ->
      hkByteArithmetic hooks <$> bytesOf "input" inputM <*> intOf "byte_offset" (f "byte_offset" v) <*> intOf "delta" (f "delta" v) <*> intOf "width" (f "width" v) <*> endianOf (f "endian" v)
    "mutate_boundary" ->
      hkBoundaryInsert hooks <$> bytesOf "input" inputM <*> intOf "byte_offset" (f "byte_offset" v) <*> intOf "width" (f "width" v) <*> intOf "val" (f "val" v) <*> endianOf (f "endian" v)
    "mutate_block_delete" -> hkBlockDelete hooks <$> bytesOf "input" inputM <*> intOf "offset" (f "offset" v) <*> intOf "length" (f "length" v)
    "mutate_block_insert" -> hkBlockInsert hooks <$> bytesOf "input" inputM <*> intOf "offset" (f "offset" v) <*> bytesOf "insert_bytes" (f "insert_bytes" v)
    "mutate_block_duplicate" ->
      hkBlockDuplicate hooks <$> bytesOf "input" inputM <*> intOf "src_offset" (f "src_offset" v) <*> intOf "length" (f "length" v) <*> intOf "dst_offset" (f "dst_offset" v)
    "mutate_splice" ->
      hkSplice hooks <$> bytesOf "target" (f "target" v) <*> bytesOf "donor" (f "donor" v) <*> intOf "target_split" (f "target_split" v) <*> intOf "donor_split" (f "donor_split" v)
    _ -> sequenceResult
  want <- bytesOf "output" (f "output" v)
  let mismatch =
        [cid <> ": " <> op <> " expected " <> hexClip want <> ", got " <> hexClip got | got /= want]
  lens <- case op of
    _ | op `elem` ["mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary"] -> do
      input <- bytesOf "input" inputM
      pure [cid <> ": " <> op <> " changed the length of its input" | B.length got /= B.length input]
    "mutate_splice" -> do
      tgt <- bytesOf "target" (f "target" v)
      don <- bytesOf "donor" (f "donor" v)
      ts <- intOf "target_split" (f "target_split" v)
      ds <- intOf "donor_split" (f "donor_split" v)
      let clampTo size n = fromInteger (max 0 (min (toInteger size) n)) :: Int
          wantLen = clampTo (B.length tgt) ts + B.length don - clampTo (B.length don) ds
      pure [cid <> ": splice length is not prefix + suffix" | B.length got /= wantLen]
    _ -> pure []
  pure (mismatch ++ lens)
  where
    sequenceResult = do
      input <- bytesOf "input" (M.lookup "input" v)
      seed <- intOf "seed" (M.lookup "seed" v)
      when (seed < 0 || seed >= 2 ^ (64 :: Int)) (caseError "seed outside [0, 2^64-1]")
      steps <- sizeOf "steps" (M.lookup "steps" v)
      left <- readIORef work
      let exhausted = "mutate_sequence work budget of " ++ show corpusSequenceWork ++ " byte-steps exhausted"
      when (left <= 0) (caseError exhausted)
      r <- hkMutateSequence hooks input seed (toInteger steps) maxInputBytes left
      case r of
        WorkOk out done -> writeIORef work (max 0 (left - done)) >> pure out
        WorkExhausted _ -> writeIORef work 0 >> caseError exhausted

checkCorpusAdmit :: Hooks -> Text -> M.Map Text Val -> IO [Text]
checkCorpusAdmit hooks cid v = do
  existing <- edgesOf "existing_edges" (M.lookup "existing_edges" v)
  trace <- edgesOf "trace_edges" (M.lookup "trace_edges" v)
  admitted <- boolOf "admitted" (M.lookup "admitted" v)
  newEdges <- intOf "new_edges" (M.lookup "new_edges" v)
  let (gotAdmitted, gotNew) = hkCorpusAdmit hooks existing trace
      have = Set.fromList existing
  pure
    ( [cid <> ": corpus_admit admitted " <> tshow gotAdmitted <> ", corpus " <> tshow admitted | gotAdmitted /= admitted]
        ++ [cid <> ": corpus_admit new_edges " <> tshow gotNew <> ", corpus " <> tshow newEdges | toInteger gotNew /= newEdges]
        ++ [cid <> ": corpus_admit disagrees with set difference" | gotAdmitted /= any (`Set.notMember` have) trace]
    )

checkCrashDedup :: Hooks -> Text -> M.Map Text Val -> IO [Text]
checkCrashDedup hooks cid v = do
  ka <- textOf "crash_a_kind" (M.lookup "crash_a_kind" v)
  fa <- textsOf "crash_a_frames" (M.lookup "crash_a_frames" v)
  kb <- textOf "crash_b_kind" (M.lookup "crash_b_kind" v)
  fb <- textsOf "crash_b_frames" (M.lookup "crash_b_frames" v)
  want <- boolOf "is_duplicate" (M.lookup "is_duplicate" v)
  let got = hkDuplicateCrash hooks ka fa kb fb
  pure [cid <> ": crash_dedup is_duplicate " <> tshow got <> ", corpus " <> tshow want | got /= want]

checkPrivate :: Hooks -> Text -> M.Map Text Val -> IO [Text]
checkPrivate hooks cid v = do
  raw <- bytesOf "raw_payload" (M.lookup "raw_payload" v)
  target <- textOf "target" (M.lookup "target" v)
  fid <- textOf "failure_id" (M.lookup "failure_id" v)
  sig <- textOf "sig_hash" (M.lookup "sig_hash" v)
  tap <- textOf "tap_line" (M.lookup "tap_line" v)
  wantLeak <- boolOf "contains_raw_payload" (M.lookup "contains_raw_payload" v)
  r <- tryRecoverable (hkFormatTap hooks 1 target fid sig)
  line <- case r of
    Right l -> pure l
    Left e -> do
      m <- safeMessage e
      caseError ("format_tap_failure refused: " ++ T.unpack (exceptionTypeName e <> ": " <> clipText maxFragmentLen m))
  let leaked = not (B.null raw) && T.pack (hexLower raw) `T.isInfixOf` line
  pure
    ( [ cid <> ": tap_line expected " <> clipText maxFragmentLen (escapeText True tap) <> ", got " <> clipText maxFragmentLen (escapeText True line)
      | line /= tap
      ]
        ++ [cid <> ": privacy verdict wrong, payload leaked: " <> tshow leaked | leaked /= wantLeak]
        ++ [cid <> ": failure line spans more than one line" | T.any (\c -> c == '\n' || c == '\r') line]
    )
