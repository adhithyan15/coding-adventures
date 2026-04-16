module Main (main) where

import Sha512Spec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
