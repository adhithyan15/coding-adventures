module Main (main) where

import Md5Spec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
