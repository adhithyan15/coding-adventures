module Main (main) where

import Test.Hspec (hspec)
import qualified ZstdSpec

main :: IO ()
main = hspec ZstdSpec.spec
