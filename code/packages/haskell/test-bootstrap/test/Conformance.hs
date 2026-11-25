-- | The pbt-v1 conformance suite (VER05n 8): loads the committed corpus, strictly
-- parses it and evaluates every case with the engine's own public functions.
-- Exits non-zero unless exactly 25 cases are evaluated with zero problems.
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Exit (exitFailure)
import System.IO (hFlush, stdout)
import System.Timeout (timeout)

import CodingAdventures.TestBootstrap.Pbt.Corpus
import Fixture (readCorpus)

main :: IO ()
main = do
  r <- timeout (120 * 1000000) $ do
    raw <- readCorpus
    case parseCorpus raw of
      Left e -> putStrLn ("pbt-v1: corpus rejected by the strict parser: " ++ e) >> pure False
      Right cases -> do
        report <- evaluate cases
        mapM_ (TIO.putStrLn . T.append "  problem: ") (reportProblems report)
        putStrLn ("pbt-v1: " ++ show (reportEvaluated report) ++ "/" ++ show pbtV1CaseCount)
        pure (reportEvaluated report == pbtV1CaseCount && null (reportProblems report))
  hFlush stdout
  case r of
    Just True -> pure ()
    _ -> exitFailure
