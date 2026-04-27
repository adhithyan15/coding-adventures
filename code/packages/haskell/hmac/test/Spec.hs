module Main (main) where

import HmacSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
