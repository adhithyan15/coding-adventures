module Main (main) where

import Argon2idSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
