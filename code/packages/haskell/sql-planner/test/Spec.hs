module Main (main) where

import Test.Hspec
import qualified SqlPlannerSpec

main :: IO ()
main = hspec SqlPlannerSpec.spec
