module Main where

import Microbenchmark (Order(BaselineFirst), Pair(..), analyze)
import qualified Microbenchmark

main :: IO ()
main = case analyze [Pair 100 100 BaselineFirst] 95 1.05 of
    Right result | analysisRatio result == 1 -> putStrLn "installed Haskell consumer passed"
    _ -> fail "installed Haskell consumer returned unexpected analysis"
  where
    analysisRatio = Microbenchmark.analysisRatio
