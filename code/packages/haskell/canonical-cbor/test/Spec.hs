module Main (main) where

import System.Exit (exitSuccess)
import System.IO (hSetBuffering, stdout, BufferMode(LineBuffering))

import CorpusSpec (runCorpusTests)
import CanonicalCborSpec (runCanonicalCborTests)

main :: IO ()
main = do
  hSetBuffering stdout LineBuffering
  putStrLn "============================================================"
  putStrLn "Running Canonical CBOR Test Suite (RFC 8949 CBR01 Conformance)"
  putStrLn "============================================================"

  corpusCount <- runCorpusTests
  putStrLn ("[OK] Conformance corpus: " ++ show corpusCount ++ " test cases passed.")

  specCount <- runCanonicalCborTests
  putStrLn ("[OK] CVE resilience, properties & fuzzing: " ++ show specCount ++ " test suites passed.")

  putStrLn "============================================================"
  putStrLn ("ALL " ++ show (corpusCount + specCount) ++ " TEST SUITES PASSED SUCCESSFULLY.")
  putStrLn "============================================================"
  exitSuccess
