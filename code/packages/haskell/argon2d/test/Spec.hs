module Main (main) where

import Argon2dSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
