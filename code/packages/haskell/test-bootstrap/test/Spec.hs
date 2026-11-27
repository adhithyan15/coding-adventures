module Main (main) where

import Harness (runTests)
import qualified ApiSpec
import qualified BudgetSpec
import qualified CanonSpec
import qualified CorpusSpec
import qualified GenSpec
import qualified HazardSpec
import qualified JsonTfvSpec
import qualified PrngSpec
import qualified RunnerSpec
import qualified ShrinkSpec

main :: IO ()
main =
  runTests "test-bootstrap" $
    concat
      [ PrngSpec.tests
      , CanonSpec.tests
      , JsonTfvSpec.tests
      , ShrinkSpec.tests
      , GenSpec.tests
      , RunnerSpec.tests
      , BudgetSpec.tests
      , HazardSpec.tests
      , CorpusSpec.tests
      , ApiSpec.tests
      ]
