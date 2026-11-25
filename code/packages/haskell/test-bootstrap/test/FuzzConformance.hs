-- | The fuzz-v1 conformance suite (VER06o 8, 9): loads the committed corpus,
-- strictly parses it and evaluates every case with the engine's own functions, then
-- replays the Go lane's oracle vectors.  Exits non-zero unless exactly 25 cases are
-- evaluated with zero problems and every vector matches.
module Main (main) where

import Control.Monad (forM)
import qualified Data.Map.Strict as M
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Exit (exitFailure)
import System.IO (hFlush, stdout)
import System.Timeout (timeout)

import CodingAdventures.TestBootstrap.Fuzz.Corpus
import Fixture (readFuzzCorpus, readGoVectors)
import Vectors

main :: IO ()
main = do
  r <- timeout (300 * 1000000) $ do
    raw <- readFuzzCorpus
    corpusOk <- case parseFuzzCorpus raw of
      Left e -> putStrLn ("fuzz-v1: corpus rejected by the strict parser: " ++ e) >> pure False
      Right cases -> do
        report <- evaluate cases
        mapM_ (TIO.putStrLn . T.append "  problem: ") (reportProblems report)
        putStrLn ("fuzz-v1: " ++ show (reportEvaluated report) ++ "/" ++ show fuzzV1CaseCount)
        pure (reportEvaluated report == fuzzV1CaseCount && null (reportProblems report))
    table <- readGoVectors
    vectorsOk <- case parseVectors table of
      Left e -> putStrLn ("vectors: " ++ e) >> pure False
      Right vs -> do
        results <- forM vs runVector
        let bad = [e | Left e <- results]
            counts = M.fromListWith (+) [(vOp v, 1 :: Int) | v <- vs]
            thin = [op | op <- vectorOps, M.findWithDefault 0 op counts < 50]
        mapM_ (putStrLn . ("  vector problem: " ++)) (take 10 bad)
        mapM_ (\op -> putStrLn ("  too few vectors for " ++ op)) thin
        putStrLn ("vectors: " ++ show (length vs - length bad) ++ "/" ++ show (length vs))
        pure (null bad && null thin && length vs == 820)
    pure (corpusOk && vectorsOk)
  hFlush stdout
  case r of
    Just True -> pure ()
    _ -> exitFailure
