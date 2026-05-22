module Main (main) where

import Test.Hspec
import qualified SqlExecutionEngineSpec

main :: IO ()
main = hspec SqlExecutionEngineSpec.spec
