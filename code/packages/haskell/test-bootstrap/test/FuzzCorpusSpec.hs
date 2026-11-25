-- | The fuzz-v1 corpus replay and its NEGATIVE CONTROLS (VER06o 8, 9): a
-- conformance check that cannot fail proves nothing, so every case is corrupted in
-- every way its op defines and the run MUST fail, naming exactly that case; every
-- sabotaged engine entry point must fail the run too.
module FuzzCorpusSpec (tests) where

import Control.Monad (forM, forM_)
import Data.IORef
import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE

import CodingAdventures.TestBootstrap.CorpusJson.StrictJson
import CodingAdventures.TestBootstrap.Fuzz.Corpus
import CodingAdventures.TestBootstrap.Fuzz.Hooks
import CodingAdventures.TestBootstrap.Fuzz.Limits (corpusSequenceWork)
import CodingAdventures.TestBootstrap.Fuzz.Mutate (SeqResult (..))
import Fixture (readFuzzCorpus, readFuzzTfv)
import qualified CodingAdventures.TestBootstrap.Tfv as Tfv
import Harness

loadCases :: IO [Json]
loadCases = do
  raw <- readFuzzCorpus
  either (\e -> failTest ("corpus rejected: " ++ e)) pure (parseFuzzCorpus raw)

field :: B.ByteString -> Json -> Maybe Json
field k (JObject fs) = lookup k fs
field _ _ = Nothing

caseId, caseOp :: Json -> Text
caseId j = case field "id" j of
  Just (JString s) -> TE.decodeUtf8 s
  _ -> ""
caseOp j = case field "op" j of
  Just (JString s) -> TE.decodeUtf8 s
  _ -> ""

setField :: B.ByteString -> Json -> Json -> Json
setField k v (JObject fs) = JObject [(k', if k' == k then v else x) | (k', x) <- fs]
setField _ _ j = j

modField :: B.ByteString -> (Json -> Json) -> Json -> Json
modField k f j = maybe j (\v -> setField k (f v) j) (field k j)

dropField :: B.ByteString -> Json -> Json
dropField k (JObject fs) = JObject [(k', x) | (k', x) <- fs, k' /= k]
dropField _ j = j

addField :: B.ByteString -> Json -> Json -> Json
addField k v (JObject fs) = JObject (fs ++ [(k, v)])
addField _ _ j = j

intNode :: Integer -> Json
intNode n = JObject [("kind", JString "int"), ("value", JString (BC.pack (show n)))]

-- | A different value of the same shape.
corrupt :: Json -> Json
corrupt = \case
  JBool b -> JBool (not b)
  JString s -> JString (B.append s "x")
  JArray [] -> JArray [intNode 0]
  JArray (x : xs) -> JArray (corrupt x : xs)
  JObject fs -> case lookup "kind" fs of
    Just (JString "int") -> case lookup "value" fs of
      Just (JString v) -> intNode (read (BC.unpack v) + 1)
      _ -> JNull
    _ -> case lookup "hex" fs of
      Just (JString h) ->
        JObject [("kind", JString "bytes"), ("hex", JString (if B.null h then "00" else B.cons (if BC.head h == '0' then 0x31 else 0x30) (B.drop 1 h)))]
      _ -> JNull
  other -> other

mutators :: [Text]
mutators = ["mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary", "mutate_block_delete", "mutate_block_insert", "mutate_block_duplicate", "mutate_splice", "mutate_sequence"]

-- | Every corruption an op defines; each keeps the case RUNNABLE, so the only
-- problems are its own.
corruptions :: Text -> [(String, Json -> Json)]
corruptions op
  | op `elem` mutators = [("output", modField "output" corrupt)]
  | otherwise = case T.unpack op of
      "corpus_admit" -> [("new_edges", modField "new_edges" corrupt), ("admitted", modField "admitted" corrupt)]
      "crash_dedup" -> [("is_duplicate", modField "is_duplicate" corrupt)]
      "failure_id_valid" -> [("valid", modField "valid" corrupt)]
      "private_reporting" -> [("tap_line", modField "tap_line" corrupt), ("contains_raw_payload", modField "contains_raw_payload" corrupt)]
      "corpus_reject" -> [("a valid TFV in place of a reject text", setField "text" (JString "tfv 1\n"))]
      _ -> []

problemsOf :: Report -> [Text]
problemsOf = reportProblems

anyP :: (Text -> Bool) -> Report -> Bool
anyP f = any f . problemsOf

replaceAt :: Int -> a -> [a] -> [a]
replaceAt i x xs = take i xs ++ [x] ++ drop (i + 1) xs

indexOfOp :: Text -> [Json] -> Int
indexOfOp op cs = length (takeWhile ((/= op) . caseOp) cs)

-- | Every sabotage: one wrong engine function at a time.
sabotages :: [(String, Hooks -> Hooks)]
sabotages =
  [ ("bit_flip", \h -> h {hkBitFlip = \d _ _ -> d})
  , ("byte_flip", \h -> h {hkByteFlip = \d _ _ -> d})
  , ("byte_arithmetic", \h -> h {hkByteArithmetic = \d _ _ _ _ -> d})
  , ("boundary_insert", \h -> h {hkBoundaryInsert = \d _ _ _ _ -> d})
  , ("block_delete", \h -> h {hkBlockDelete = \d _ _ -> d})
  , ("block_insert", \h -> h {hkBlockInsert = \d _ _ -> d})
  , ("block_duplicate", \h -> h {hkBlockDuplicate = \d _ _ _ -> d})
  , ("splice", \h -> h {hkSplice = \t _ _ _ -> t})
  , ("mutate_sequence", \h -> h {hkMutateSequence = \d _ _ _ _ -> pure (WorkOk d 1)})
  , ("corpus_admit false", \h -> h {hkCorpusAdmit = \_ _ -> (False, 0)})
  , ("corpus_admit counts duplicates", \h -> h {hkCorpusAdmit = \e t -> (True, length t + length e)})
  , ("duplicate_crash always", \h -> h {hkDuplicateCrash = \_ _ _ _ -> True})
  , ("duplicate_crash never", \h -> h {hkDuplicateCrash = \_ _ _ _ -> False})
  , ("failure id accepts everything", \h -> h {hkFailureId = const True})
  , ("failure id accepts nothing", \h -> h {hkFailureId = const False})
  , ("tap line", \h -> h {hkFormatTap = \_ _ _ _ -> pure "not ok 1 - x"})
  , ("tap line leaks the payload", \h -> h {hkFormatTap = \_ _ _ _ -> pure "not ok 1 - ct_compare [fuzz-crash:3a8f19b2] deadbeef"})
  , ("tap line with a newline", \h -> h {hkFormatTap = \_ _ _ _ -> pure "not ok 1\n- ct_compare [fuzz-crash:3a8f19b2]"})
  , ("tfv accepts everything", \h -> h {hkTfvValid = const True})
  ]

tests :: [Test]
tests =
  [ test "the committed corpus conforms exactly, and so does its TFV" $ do
      cases <- loadCases
      assertEq "case count" 25 (length cases)
      report <- evaluate cases
      assertEq "problems" [] (problemsOf report)
      assertEq "evaluated" 25 (reportEvaluated report)
      tfv <- readFuzzTfv
      assertTrue "cases.tfv is accepted by the strict validator" (Tfv.valid tfv)
  , test "every op has a corruption table entry" $ do
      cases <- loadCases
      let ops = foldr (\c acc -> if caseOp c `elem` acc then acc else caseOp c : acc) [] cases
      assertEq "14 ops" 14 (length ops)
      forM_ cases $ \c -> assertTrue (T.unpack (caseId c) ++ " has a corruption") (not (null (corruptions (caseOp c))))
  , test "every corruption of every case fails and names only that case" $ do
      cases <- loadCases
      totals <- forM (zip [0 ..] cases) $ \(i, original) ->
        forM (corruptions (caseOp original)) $ \(label, mutate) -> do
          report <- evaluate (replaceAt i (mutate original) cases)
          assertTrue (T.unpack (caseId original) ++ ": " ++ label ++ " was not detected") (not (null (problemsOf report)))
          forM_ (problemsOf report) $ \p ->
            assertTrue ("problem names another case: " ++ T.unpack p) ((caseId original <> ": ") `T.isPrefixOf` p)
          pure (1 :: Int)
      assertTrue "at least 25 corruptions" (sum (map sum totals) >= 25)
  , test "every field of every case wrong-typed is a problem, never a crash" $ do
      cases <- loadCases
      forM_ (zip [0 ..] cases) $ \(i, original) -> case original of
        JObject fs ->
          forM_ [k | (k, _) <- fs, k /= "id", k /= "op", not (caseOp original == "failure_id_valid" && k == "input")] $ \k ->
            forM_ [bad0 | bad0 <- [JNull, JBool True, JBool False, JNumber "3", JArray [JNull]], Just bad0 /= lookup k fs] $ \bad -> do
              report <- evaluate (replaceAt i (setField k bad original) cases)
              let label = T.unpack (caseId original) ++ "." ++ BC.unpack k ++ " wrong-typed"
              assertTrue (label ++ " names the case") (any ((caseId original <> ": ") `T.isPrefixOf`) (problemsOf report))
              forM_ (problemsOf report) $ \p ->
                assertTrue ("problem names another case: " ++ T.unpack p) (((caseId original <> ": ") `T.isPrefixOf` p) || ("fuzz-v1: " `T.isPrefixOf` p))
        _ -> pure ()
  , test "every sabotaged engine entry point fails the run" $ do
      cases <- loadCases
      forM_ sabotages $ \(label, sabotage) -> do
        report <- evaluateWith (sabotage defaultHooks) (Just 25) cases
        assertTrue ("sabotage not detected: " ++ label) (not (null (problemsOf report)))
        -- and nothing leaks: the unsabotaged run is clean again
        clean <- evaluate cases
        assertEq ("clean after " ++ label) [] (problemsOf clean)
  , test "a dropped, duplicated or truncated corpus fails" $ do
      cases <- loadCases
      r1 <- evaluate (tail cases)
      assertTrue "dropped first" (anyP ("expected 25 cases, got 24" `T.isInfixOf`) r1)
      r2 <- evaluate (take 10 cases)
      assertTrue "truncated" (anyP ("operations never exercised" `T.isInfixOf`) r2)
      r3 <- evaluate (cases ++ [head cases])
      assertTrue "duplicated" (anyP ("duplicate or missing case id" `T.isInfixOf`) r3)
      raw <- readFuzzCorpus
      assertLeft "truncated JSON" (parseFuzzCorpus (B.take (B.length raw - 10) raw))
      assertLeft "empty object" (parseFuzzCorpus "{}")
      assertLeft "schema 2" (parseFuzzCorpus "{\"schema_version\": 2, \"cases\": []}")
      assertLeft "cases not an array" (parseFuzzCorpus "{\"schema_version\": 1, \"cases\": 3}")
      assertLeft "extra key" (parseFuzzCorpus "{\"schema_version\": 1, \"cases\": [], \"x\": 1}")
      r4 <- evaluate []
      assertEq "none evaluated" 0 (reportEvaluated r4)
      assertTrue "empty corpus fails" (not (null (problemsOf r4)))
  , test "removing every case of one op fails (each op must be exercised)" $ do
      cases <- loadCases
      let ops = foldr (\c acc -> if caseOp c `elem` acc then acc else caseOp c : acc) [] cases
      forM_ ops $ \op -> do
        report <- evaluateWith defaultHooks Nothing (filter ((/= op) . caseOp) cases)
        assertTrue ("dropping " ++ T.unpack op ++ " was not noticed") (anyP ("never exercised" `T.isInfixOf`) report || anyP ("needs a" `T.isInfixOf`) report)
  , test "unknown op, extra field and missing field are problems" $ do
      cases <- loadCases
      r1 <- evaluate (replaceAt 0 (setField "op" (JString "mystery") (head cases)) cases)
      assertTrue "unknown op" (anyP ("unknown op" `T.isInfixOf`) r1)
      r2 <- evaluate (replaceAt 0 (addField "extra" (JBool True) (head cases)) cases)
      assertTrue "extra field" (anyP (\p -> (caseId (head cases) <> ": fields") `T.isPrefixOf` p) r2)
      r3 <- evaluate (replaceAt 0 (dropField "count" (head cases)) cases)
      assertTrue "missing field" (anyP ("fields" `T.isInfixOf`) r3)
  , test "malformed ids and cases are problems, not crashes" $ do
      cases <- loadCases
      r1 <- evaluate (replaceAt 1 (setField "id" (JString (TE.encodeUtf8 (caseId (head cases)))) (cases !! 1)) cases)
      assertTrue "duplicate id" (anyP ("duplicate or missing case id" `T.isInfixOf`) r1)
      r2 <- evaluate (replaceAt 0 (setField "id" (JArray [JString "x"]) (head cases)) cases)
      assertTrue "non-string id" (anyP ("duplicate or missing case id" `T.isInfixOf`) r2)
      r3 <- evaluate (replaceAt 2 (JNumber "7") cases)
      assertTrue "not an object" (anyP ("case is not an object" `T.isInfixOf`) r3)
      r4 <- evaluate (replaceAt 2 (JObject [("op", JString "mutate_bit_flip")]) cases)
      assertTrue "no id" (anyP ("case missing a string id" `T.isInfixOf`) r4)
      -- a hostile id is escaped in every problem about the case
      let evil = "fuzz/\x202e\n/evil"
      r5 <- evaluate (replaceAt 0 (setField "output" (JNull) (setField "id" (JString (TE.encodeUtf8 evil)) (head cases))) cases)
      assertTrue "escaped id" (all (\p -> not (T.any (\c -> c == '\n' || c == '\x202e') p)) (problemsOf r5))
  , test "hostile sizes are reported, not executed" $ do
      cases <- loadCases
      let i = indexOfOp "mutate_sequence" cases
          c = cases !! i
      r1 <- evaluate (replaceAt i (setField "steps" (intNode 1000000000) c) cases)
      assertTrue "steps cap" (anyP (\p -> (caseId c <> ": mutate_sequence could not be evaluated") `T.isPrefixOf` p && "outside [0, 10000]" `T.isInfixOf` p) r1)
      r2 <- evaluate (replaceAt i (setField "seed" (intNode (2 ^ (64 :: Int))) c) cases)
      assertTrue "seed range" (anyP ("seed outside" `T.isInfixOf`) r2)
      let spent = defaultHooks {hkMutateSequence = \_ _ _ _ _ -> pure (WorkExhausted 7)}
      r3 <- evaluateWith spent (Just 25) cases
      assertTrue "work budget" (anyP (\p -> (caseId c <> ": mutate_sequence could not be evaluated") `T.isPrefixOf` p && "work budget" `T.isInfixOf` p) r3)
      let e = indexOfOp "corpus_admit" cases
          ec = cases !! e
      r4 <- evaluate (replaceAt e (setField "trace_edges" (JArray [intNode (-1)]) ec) cases)
      assertTrue "edge range" (anyP ("outside [0, 2^64)" `T.isInfixOf`) r4)
  , test "the aggregate mutate_sequence cap: a breach zeroes the allowance for every later case" $ do
      cases <- loadCases
      let i = indexOfOp "mutate_sequence" cases
          c = cases !! i
          first = setField "id" (JString "fuzz/hog") c
          later = setField "id" (JString "fuzz/later") c
          -- a hog that spends the WHOLE allowance and succeeds: the next case has none left
          spendAll = defaultHooks {hkMutateSequence = \d _ _ _ cap -> pure (WorkOk d cap)}
      r1 <- evaluateWith spendAll Nothing [first, later]
      assertTrue "the later honest case fails at once" (anyP (\p -> "fuzz/later: mutate_sequence" `T.isPrefixOf` p && "work budget" `T.isInfixOf` p) r1)
      -- a hog that breaches: it is reported AND the allowance is zeroed
      breach <- pure defaultHooks {hkMutateSequence = \_ _ _ _ _ -> pure (WorkExhausted 3)}
      r2 <- evaluateWith breach Nothing [first, later]
      assertTrue "the hog is reported" (anyP (\p -> "fuzz/hog: mutate_sequence" `T.isPrefixOf` p && "work budget" `T.isInfixOf` p) r2)
      assertTrue "the later case fails at once" (anyP (\p -> "fuzz/later: mutate_sequence" `T.isPrefixOf` p && "work budget" `T.isInfixOf` p) r2)
      -- the allowance each case is handed is what is LEFT of 2^24
      seen <- newIORef []
      let spy = defaultHooks {hkMutateSequence = \d _ _ _ cap -> modifyIORef' seen (cap :) >> pure (WorkOk d 1000)}
      _ <- evaluateWith spy Nothing [first, later]
      caps <- readIORef seen
      assertEq "allowances" [corpusSequenceWork - 1000, corpusSequenceWork] caps
      alone <- evaluateWith defaultHooks Nothing [later]
      assertFalse "alone it passes the sequence case" (anyP ("fuzz/later: mutate_sequence" `T.isPrefixOf`) alone)
  , test "the corpus runner never reads a message to decide: a bad endian is a problem for its case" $ do
      cases <- loadCases
      let i = indexOfOp "mutate_arithmetic" cases
          c = cases !! i
      r <- evaluate (replaceAt i (setField "endian" (JString "middle") c) cases)
      assertTrue "bad endian" (anyP (\p -> (caseId c <> ": mutate_arithmetic could not be evaluated") `T.isPrefixOf` p) r)
  ]
