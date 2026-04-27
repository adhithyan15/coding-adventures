module Main (main) where

import Argon2iSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
