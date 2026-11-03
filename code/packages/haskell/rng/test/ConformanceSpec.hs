{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE ScopedTypeVariables #-}
-- ============================================================================
-- ConformanceSpec.hs — the shared rng corpus (spec rng.md, "Conformance corpus")
-- ============================================================================
--
-- code/specs/fixtures/rng-v1/cases.txt is the flat form of the corpus every
-- language's rng package must reproduce; its results come from the
-- independent reference code/scripts/rng_conformance.py. Each line is
-- whitespace-separated:
--
--   case <id> <generator> <seed>
--   next_u32 <result>
--   next_u64 <result>
--   next_float <16 hex digits of the IEEE-754 bits>
--   next_int_in_range <min> <max> <result | error>
--   end
--
-- cabal runs the test from the package directory.
-- ============================================================================
module ConformanceSpec (spec) where

import Control.Exception (ErrorCall, evaluate, try)
import Data.Int (Int64)
import Data.Word (Word64)
import GHC.Float (castDoubleToWord64)
import Numeric (readHex)
import Rng
import Test.Hspec

corpusPath :: FilePath
corpusPath = "../../../specs/fixtures/rng-v1/cases.txt"

-- | Parse lowercase hex digits; the corpus guarantees exactly one parse.
hex :: String -> Integer
hex text = case readHex text of
  [(n, "")] -> n
  _ -> error ("bad hex " ++ text)

-- | The three generators behind one interface, each step returning the next.
data AnyRng = AnyLcg LCG | AnyXs Xorshift64 | AnyPcg PCG32

start :: String -> Word64 -> AnyRng
start "lcg" s = AnyLcg (newLCG s)
start "xorshift64" s = AnyXs (newXorshift64 s)
start "pcg32" s = AnyPcg (newPCG32 s)
start other _ = error ("unknown generator " ++ other)

step :: (forall g. RandomGen g => g -> (a, g)) -> AnyRng -> (a, AnyRng)
step f (AnyLcg g) = let (v, g') = f g in (v, AnyLcg g')
step f (AnyXs g) = let (v, g') = f g in (v, AnyXs g')
step f (AnyPcg g) = let (v, g') = f g in (v, AnyPcg g')

-- | Run every line, counting cases and calls; fails the test on a mismatch.
runCorpus :: [String] -> IO (Int, Int)
runCorpus = go Nothing "" 0 0
  where
    go _ _ cases calls [] = pure (cases, calls)
    go g ident cases calls (line : rest) = case words line of
      ("case" : i : name : seed : _) -> go (Just (start name (read seed))) i (cases + 1) calls rest
      ["end"] -> go Nothing ident cases calls rest
      ["next_u32", want] -> do
        let (v, g') = step nextU32 (unwrap g)
        (ident, toInteger v) `shouldBe` (ident, read want)
        go (Just g') ident cases (calls + 1) rest
      ["next_u64", want] -> do
        let (v, g') = step nextU64 (unwrap g)
        (ident, toInteger v) `shouldBe` (ident, read want)
        go (Just g') ident cases (calls + 1) rest
      ["next_float", want] -> do
        let (v, g') = step nextFloat (unwrap g)
        (ident, toInteger (castDoubleToWord64 v)) `shouldBe` (ident, hex want)
        go (Just g') ident cases (calls + 1) rest
      ["next_int_in_range", lo, hi, "error"] -> do
        -- 'error' before any draw, so the same generator carries on.
        result <- try (evaluate (fst (step (nextIntInRange (read lo) (read hi)) (unwrap g))))
        case result of
          Left (_ :: ErrorCall) -> pure ()
          Right v -> expectationFailure (ident ++ ": expected an error, got " ++ show (v :: Int64))
        go g ident cases (calls + 1) rest
      ["next_int_in_range", lo, hi, want] -> do
        let (v, g') = step (nextIntInRange (read lo) (read hi)) (unwrap g)
        (ident, v) `shouldBe` (ident, read want)
        go (Just g') ident cases (calls + 1) rest
      _ -> expectationFailure ("malformed corpus line: " ++ line) >> pure (cases, calls)
    unwrap = maybe (error "call outside a case") id

spec :: Spec
spec = do
  describe "rng-v1 corpus" $
    it "reproduces every case" $ do
      contents <- readFile corpusPath
      (cases, calls) <- runCorpus [l | l <- lines contents, take 1 l `notElem` ["", "#"]]
      -- Guard against a truncated or empty file passing vacuously.
      cases `shouldSatisfy` (>= 27)
      calls `shouldSatisfy` (>= 150)

  describe "regressions for the v1 range bug" $ do
    it "reaches beyond 2^32 for wide ranges" $ do
      let draws = take 64 (unfold (nextIntInRange 0 (2 ^ (40 :: Int))) (newPCG32 3))
      any (>= 2 ^ (32 :: Int)) draws `shouldBe` True
    it "covers the whole Int64 domain without dividing by zero" $ do
      let draws = take 64 (unfold (nextIntInRange minBound maxBound) (newLCG 3))
      (any (< 0) draws && any (> 0) draws) `shouldBe` True
    it "terminates for span 2^63 + 1" $ do
      let draws = take 100 (unfold (nextIntInRange minBound 0) (newXorshift64 3))
      all (<= 0) draws `shouldBe` True
  where
    unfold f g = let (v, g') = f g in v : unfold f g'
