-- | The pbt-v1 corpus replay and its NEGATIVE CONTROLS (VER05n 8): a
-- conformance check that cannot fail proves nothing, so every case is corrupted
-- in every way its op defines and the run MUST fail, naming exactly that case.
module CorpusSpec (tests) where

import Control.Monad (forM, forM_)
import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import Data.Either (isLeft)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE

import CodingAdventures.TestBootstrap.CorpusJson.StrictJson
import CodingAdventures.TestBootstrap.Pbt.Corpus
import CodingAdventures.TestBootstrap.Pbt.Types
import CodingAdventures.TestBootstrap.Prng
import Fixture (readCorpus)
import Harness

loadCases :: IO [Json]
loadCases = do
  raw <- readCorpus
  either (\e -> failTest ("corpus rejected: " ++ e)) pure (parseCorpus raw)

-- JSON helpers --------------------------------------------------------------

field :: B.ByteString -> Json -> Maybe Json
field k (JObject fs) = lookup k fs
field _ _ = Nothing

caseId :: Json -> Text
caseId j = case field "id" j of
  Just (JString s) -> TE.decodeUtf8 s
  _ -> ""

caseOp :: Json -> Text
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

-- | Every corruption an op defines.
corruptions :: Text -> [(String, Json -> Json)]
corruptions op = case T.unpack op of
  o
    | o `elem` ["gen_primitive", "gen_bounded", "gen_bytes"] ->
        [ ("outputs", modField "outputs" corrupt)
        , ("short outputs", modField "outputs" (\case JArray xs -> JArray (init xs); j -> j))
        ]
    | o `elem` ["shrink_int", "shrink_refinement", "shrink_list", "shrink_bytes"] ->
        [ ("minimal", modField "expected_minimal" corrupt)
        , ("steps+1", modField "expected_steps" corrupt)
        ]
  "replay_consistency" -> [("initial_fail", modField "initial_fail" corrupt), ("minimal", modField "expected_minimal" corrupt)]
  "filter_exhausted" -> [("failure id", setField "failure_id" (JString "property-panic")), ("verdict", setField "verdict" (JString "pass"))]
  "failure_id_valid" -> [("valid flipped", modField "valid" corrupt)]
  "corpus_reject" -> [("a valid TFV in place of a reject text", setField "text" (JString "tfv 1\n"))]
  _ -> []

eval :: [Json] -> IO Report
eval = evaluate

evalWith :: Config -> [Json] -> IO Report
evalWith cfg = evaluateWith cfg (Just pbtV1CaseCount)

problemsOf :: Report -> [Text]
problemsOf = reportProblems

anyP :: (Text -> Bool) -> Report -> Bool
anyP f = any f . problemsOf

replaceAt :: Int -> a -> [a] -> [a]
replaceAt i x xs = take i xs ++ [x] ++ drop (i + 1) xs

indexOfOp :: Text -> [Json] -> Int
indexOfOp op cs = length (takeWhile ((/= op) . caseOp) cs)

tests :: [Test]
tests =
  [ test "the committed corpus conforms exactly" $ do
      cases <- loadCases
      assertEq "case count" 25 (length cases)
      report <- eval cases
      assertEq "problems" [] (problemsOf report)
      assertEq "evaluated" 25 (reportEvaluated report)
  , test "the independent reference cases are the ones we replay" $ do
      cases <- loadCases
      let ids = map caseId cases
      assertTrue "headline" ("pbt/shrink_int/54321_le_100" `elem` ids)
      assertTrue "bytes" ("pbt/shrink_bytes/len_lt_5" `elem` ids)
      assertEq "unique" (length ids) (length (foldr (\x acc -> if x `elem` acc then acc else x : acc) [] ids))
  , test "every corruption of every case fails and names only that case" $ do
      cases <- loadCases
      totals <- forM (zip [0 ..] cases) $ \(i, original) ->
        forM (corruptions (caseOp original)) $ \(label, mutate) -> do
          report <- eval (replaceAt i (mutate original) cases)
          let want = caseId original <> ": "
          assertTrue (T.unpack (caseId original) ++ ": " ++ label ++ " was not detected") (not (null (problemsOf report)))
          forM_ (problemsOf report) $ \p ->
            assertTrue ("problem names another case: " ++ T.unpack p) (want `T.isPrefixOf` p)
          pure (1 :: Int)
      assertTrue "at least 40 corruptions" (sum (map sum totals) >= 40)
  , test "every field of every case wrong-typed is a problem, never a crash" $ do
      cases <- loadCases
      forM_ (zip [0 ..] cases) $ \(i, original) -> case original of
        JObject fs ->
          -- (a non-string failure-id input is legitimately "not registered", so that field is exempt)
          forM_ [k | (k, _) <- fs, k /= "id", k /= "op", not (caseOp original == "failure_id_valid" && k == "input")] $ \k ->
            forM_ [bad0 | bad0 <- [JNull, JBool True, JBool False, JNumber "3", JArray [JNull]], Just bad0 /= lookup k fs] $ \bad -> do
              report <- eval (replaceAt i (setField k bad original) cases)
              let label = T.unpack (caseId original) ++ "." ++ BC.unpack k ++ " wrong-typed"
              assertTrue (label ++ " names the case") (any ((caseId original <> ": ") `T.isPrefixOf`) (problemsOf report))
              -- the only other lines allowed are the corpus-wide summaries (an op whose only case broke)
              forM_ (problemsOf report) $ \p ->
                assertTrue ("problem names another case: " ++ T.unpack p) (((caseId original <> ": ") `T.isPrefixOf` p) || ("pbt-v1: " `T.isPrefixOf` p))
        _ -> pure ()
  , test "a dropped case fails" $ do
      cases <- loadCases
      report <- eval (take 3 cases ++ drop 4 cases)
      assertTrue "count" (anyP ("expected 25 cases, got 24" `T.isInfixOf`) report)
  , test "a truncated corpus fails" $ do
      cases <- loadCases
      report <- eval (take 5 cases)
      assertTrue "never exercised" (anyP ("operations never exercised" `T.isInfixOf`) report)
      assertTrue "ids missing" (anyP ("registered failure ids missing" `T.isInfixOf`) report)
  , test "dropping the unregistered-id case fails" $ do
      cases <- loadCases
      report <- evaluateWith defaultConfig Nothing (filter ((/= "pbt/failure_id/unregistered") . caseId) cases)
      assertTrue "rejection" (anyP ("unregistered-id rejection" `T.isInfixOf`) report)
  , test "an empty corpus fails" $ do
      report <- eval []
      assertTrue "problems" (not (null (problemsOf report)))
      assertEq "none evaluated" 0 (reportEvaluated report)
  , test "unknown op, extra field and missing field are problems" $ do
      cases <- loadCases
      r1 <- eval (replaceAt 0 (setField "op" (JString "mystery") (head cases)) cases)
      assertTrue "unknown op" (anyP ("unknown op" `T.isInfixOf`) r1)
      r2 <- eval (replaceAt 0 (addField "extra" (JBool True) (head cases)) cases)
      assertTrue "extra field" (anyP (("pbt/gen_primitive/bool_seed0: fields" `T.isPrefixOf`)) r2)
      r3 <- eval (replaceAt 0 (dropField "count" (head cases)) cases)
      assertTrue "missing field" (anyP ("fields" `T.isInfixOf`) r3)
  , test "malformed ids and cases are problems, not crashes" $ do
      cases <- loadCases
      r1 <- eval (replaceAt 1 (setField "id" (JString (TE.encodeUtf8 (caseId (head cases)))) (cases !! 1)) cases)
      assertTrue "duplicate id" (anyP ("duplicate or missing case id" `T.isInfixOf`) r1)
      r2 <- eval (replaceAt 0 (setField "id" (JArray [JString "x"]) (head cases)) cases)
      assertTrue "non-string id" (anyP ("duplicate or missing case id" `T.isInfixOf`) r2)
      r3 <- eval (replaceAt 2 (JNumber "7") cases)
      assertTrue "not an object" (anyP ("case is not an object" `T.isInfixOf`) r3)
      r4 <- eval (replaceAt 2 (JObject [("op", JString "gen_bool")]) cases)
      assertTrue "no id" (anyP ("case missing a string id" `T.isInfixOf`) r4)
  , test "hostile sizes are reported, not executed" $ do
      cases <- loadCases
      let c0 = head cases
      r1 <- eval (replaceAt 0 (setField "count" (intNode 1000000000) c0) cases)
      assertTrue "count cap" (anyP (\p -> "pbt/gen_primitive/bool_seed0: gen_primitive could not be evaluated" `T.isPrefixOf` p && "outside [0, 10000]" `T.isInfixOf` p) r1)
      r2 <- eval (replaceAt 0 (setField "count" (intNode 5) c0) cases)
      assertTrue "outputs length" (anyP ("exactly count (5)" `T.isInfixOf`) r2)
      let i = indexOfOp "gen_bytes" cases
          big = setField "outputs" (JArray (replicate 10000 (JObject [("kind", JString "bytes"), ("hex", JString "")])))
              (setField "count" (intNode 10000) (setField "max_len" (intNode 10000) (cases !! i)))
      r3 <- eval (replaceAt i big cases)
      assertTrue "generation budget" (anyP ("generation budget" `T.isInfixOf`) r3)
  , test "malformed values are problems" $ do
      cases <- loadCases
      forM_
        [ JObject [("kind", JString "int"), ("value", JString "007")]
        , JObject [("kind", JString "int"), ("value", JNumber "5")]
        , JObject [("kind", JString "int")]
        , JObject [("kind", JString "bytes"), ("hex", JString "FF")]
        , JObject [("kind", JString "bytes"), ("hex", JString "abc")]
        , JObject [("kind", JString "int"), ("value", JString "-0")]
        , JObject [("kind", JString "float")]
        , JNull
        , JNumber "3"
        ]
        $ \bad -> do
          r <- eval (replaceAt 0 (setField "seed" bad (head cases)) cases)
          assertTrue ("rejects " ++ take 60 (show bad)) (anyP (("pbt/gen_primitive/bool_seed0:" `T.isPrefixOf`)) r)
  , test "a fixture integer is capped at 1,000 digits" $ do
      cases <- loadCases
      let digits n = intNode' (BC.replicate n '9')
          intNode' t = JObject [("kind", JString "int"), ("value", JString t)]
      r1001 <- eval (replaceAt 0 (setField "seed" (digits 1001) (head cases)) cases)
      assertTrue "1001 digits refused" (anyP ("digits" `T.isInfixOf`) r1001)
      r1000 <- eval (replaceAt 0 (setField "seed" (digits 1000) (head cases)) cases)
      assertFalse "1000 digits accepted by the decoder" (anyP ("digits" `T.isInfixOf`) r1000)
      assertTrue "but it is not a u64 seed" (anyP ("outside [0, 2^64-1]" `T.isInfixOf`) r1000)
      rneg <- eval (replaceAt 0 (setField "seed" (intNode' (B.cons 0x2d (BC.replicate 1000 '9'))) (head cases)) cases)
      assertFalse "-999.. 1000 digits accepted by the decoder" (anyP ("digits" `T.isInfixOf`) rneg)
      assertTrue "negative seed refused" (anyP ("outside [0, 2^64-1]" `T.isInfixOf`) rneg)
  , test "a fixture seed outside [0, 2^64-1] is refused, never reduced modulo 2^64" $ do
      cases <- loadCases
      let withSeed n = replaceAt 0 (setField "seed" (intNode n) (head cases)) cases
          id0 = "pbt/gen_primitive/bool_seed0: "
      forM_ [-1, 2 ^ (64 :: Int), 2 ^ (64 :: Int) + 5, negate (2 ^ (64 :: Int))] $ \n -> do
        r <- eval (withSeed n)
        assertTrue ("refused " ++ show n) (anyP (\p -> (id0 `T.isPrefixOf` p) && ("outside [0, 2^64-1]" `T.isInfixOf` p)) r)
      -- the boundaries are accepted (they evaluate and merely mismatch the seed-0 outputs)
      forM_ [0, 2 ^ (64 :: Int) - 1] $ \n -> do
        r <- eval (withSeed n)
        assertFalse ("accepted " ++ show n) (anyP ("outside [0, 2^64-1]" `T.isInfixOf`) r)
  , test "unknown property and wrong property family are problems" $ do
      cases <- loadCases
      let i = indexOfOp "shrink_int" cases
      r1 <- eval (replaceAt i (setField "property" (JString "no_such_property") (cases !! i)) cases)
      assertTrue "unknown" (anyP ("unknown property" `T.isInfixOf`) r1)
      r2 <- eval (replaceAt i (setField "property" (JString "list_sum_le_20") (cases !! i)) cases)
      assertTrue "wrong family" (anyP ("unknown property" `T.isInfixOf`) r2)
  , test "parseCorpus requires the exact document shape" $ do
      assertEq "empty cases" (Right []) (fmap (map caseId) (parseCorpus "{\"schema_version\": 1, \"cases\": []}"))
      mapM_
        (\t -> either (\m -> assertTrue (BC.unpack t) (length m > 0)) (const (failTest (BC.unpack t))) (parseCorpus t))
        [ "{\"cases\": []}"
        , "{\"schema_version\": 2, \"cases\": []}"
        , "{\"schema_version\": \"1\", \"cases\": []}"
        , "{\"schema_version\": 1, \"cases\": {}}"
        , "[]"
        , "{\"schema_version\": 1, \"cases\": [], \"x\": 1}"
        , "{\"schema_version\": 1, \"cases\": [], \"cases\": []}"
        ]
  , test "ops with properties the committed corpus does not use still evaluate" $ do
      cases <- loadCases
      let i = indexOfOp "shrink_int" cases
          j = indexOfOp "shrink_bytes" cases
          k = indexOfOp "shrink_list" cases
          mk idx prop initial = setField "property" (JString prop) (setField "initial" initial (cases !! idx))
      r1 <- eval (replaceAt i (mk i "int_is_even" (intNode 7)) cases)
      assertTrue "even: mismatch reported against the corpus values" (anyP (("pbt/shrink_int/54321_le_100:" `T.isPrefixOf`)) r1)
      r2 <- eval (replaceAt i (mk i "int_abs_le_10" (intNode 500)) cases)
      assertTrue "abs: mismatch reported" (anyP (("pbt/shrink_int/54321_le_100:" `T.isPrefixOf`)) r2)
      r3 <- eval (replaceAt j (mk j "bytes_no_zero_byte" (JObject [("kind", JString "bytes"), ("hex", JString "0102")])) cases)
      assertTrue "no-zero-byte: mismatch reported" (anyP (("pbt/shrink_bytes/len_lt_5:" `T.isPrefixOf`)) r3)
      r4 <- eval (replaceAt k (mk k "list_sum_le_20" (JArray [intNode 30])) cases)
      assertTrue "list: mismatch reported" (anyP (("pbt/shrink_list/consecutive_duplicates:" `T.isPrefixOf`)) r4)
      -- a replay whose property cannot be falsified, and a zero max_retries
      let rp = indexOfOp "replay_consistency" cases
      r5 <- eval (replaceAt rp (setField "property" (JString "int_abs_le_10") (cases !! rp)) cases)
      assertTrue "replay mismatch reported" (anyP (("pbt/replay/seed42_le_100:" `T.isPrefixOf`)) r5)
      let fe = indexOfOp "filter_exhausted" cases
      r6 <- eval (replaceAt fe (setField "max_retries" (intNode 0) (cases !! fe)) cases)
      assertTrue "max_retries 0" (anyP ("at least 1" `T.isInfixOf`) r6)
      r7 <- eval (replaceAt fe (setField "max_retries" (intNode 7) (cases !! fe)) cases)
      assertEq "any retry count conforms" [] (problemsOf r7)
  , test "the whole corpus text round trips through the strict parser" $ do
      raw <- readCorpus
      assertTrue "parses" (not (isLeft (parseCorpus raw)))
      assertLeft "trailing byte" (parseCorpus (B.append raw "x"))
  , test "a case list above the cap is a problem" $ do
      let big = replicate (maxCorpusCases + 1) (JObject [("id", JString "c"), ("op", JString "nope")])
      report <- evaluateWith defaultConfig Nothing big
      assertTrue "cap" (anyP ("exceeds the cap" `T.isInfixOf`) report)
      assertTrue "bounded problem list" (length (problemsOf report) <= maxProblems)
  , test "hostile case ids are escaped in problems" $ do
      cases <- loadCases
      let evil = JString "pbt/x\n\x1b[31mFORGED\x202e"
      report <- eval (replaceAt 0 (setField "id" evil (setField "op" (JString "mystery") (head cases))) cases)
      assertFalse "no raw newline or bidi override" (anyP (\p -> "\n" `T.isInfixOf` p || "\x202e" `T.isInfixOf` p) report)
  , test "problems are clipped" $ do
      cases <- loadCases
      let evil = JString (B.replicate 5000 0x61)
      report <- eval (replaceAt 0 (setField "id" evil (setField "op" (JString "mystery") (head cases))) cases)
      assertTrue "each problem is short" (all ((<= 410) . T.length) (problemsOf report))
  , -- sabotage ----------------------------------------------------------
    test "a changed bisection is caught" $ do
      cases <- loadCases
      r <- evalWith defaultConfig {cfgBisect = \cur diff -> cur - (diff `div` 2) - 1} cases
      assertTrue "problems" (not (null (problemsOf r)))
  , test "a changed unit step is caught" $ do
      cases <- loadCases
      r <- evalWith defaultConfig {cfgUnit = \cur diff -> cur - 2 * signum diff} cases
      assertTrue "problems" (not (null (problemsOf r)))
  , test "floor division equals the arithmetic shift, so that hook still conforms" $ do
      cases <- loadCases
      r <- evalWith defaultConfig {cfgBisect = \cur diff -> cur - floor' diff} cases
      assertEq "conforms" [] (problemsOf r)
  , test "a skipped deletion phase is caught" $ do
      cases <- loadCases
      r <- evalWith defaultConfig {cfgSkipDeletion = True} cases
      assertTrue "problems" (not (null (problemsOf r)))
  , test "a shifted RNG is caught" $ do
      cases <- loadCases
      r <- evalWith defaultConfig {cfgNewRng = \s -> newRng (s + 1)} cases
      assertTrue "problems" (not (null (problemsOf r)))
  , test "a wrong rejection threshold is caught by the corpus when it rejects enough" $ do
      cases <- loadCases
      r <- evalWith defaultConfig {cfgNewRng = newRngWithThreshold (const (2 ^ (63 :: Int)))} cases
      assertTrue "problems" (not (null (problemsOf r)))
  , test "the blind spot: a threshold of 0 (the classic -n mod n bug) is NOT caught by the corpus" $ do
      -- Documented, not a defect: the corpus spans are too small for the rejection
      -- branch to fire, which is exactly why PrngSpec pins the forced rejection draw.
      cases <- loadCases
      r <- evalWith defaultConfig {cfgNewRng = newRngWithThreshold (const 0)} cases
      assertEq "corpus still conforms" [] (problemsOf r)
  , test "an engine that throws is a problem for the case" $ do
      cases <- loadCases
      r <- evalWith defaultConfig {cfgNewRng = \_ -> ioError (userError "rng boom")} cases
      assertTrue "reported" (anyP ("could not be evaluated: IOException: user error (rng boom)" `T.isInfixOf`) r)
  , test "the default configuration restores production behaviour" $ do
      cases <- loadCases
      r <- evalWith defaultConfig cases
      assertEq "conforms" [] (problemsOf r)
  ]
  where
    floor' diff = diff `div` 2

