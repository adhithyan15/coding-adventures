module Main (main) where

import Sha1Spec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
