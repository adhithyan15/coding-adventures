module Main (main) where

import qualified SqlCsvSourceSpec
import Test.Hspec

main :: IO ()
main = hspec SqlCsvSourceSpec.spec
