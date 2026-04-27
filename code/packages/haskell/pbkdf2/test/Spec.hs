module Main (main) where

import Pbkdf2Spec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
