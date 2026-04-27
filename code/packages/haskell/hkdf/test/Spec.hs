module Main (main) where

import HkdfSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
