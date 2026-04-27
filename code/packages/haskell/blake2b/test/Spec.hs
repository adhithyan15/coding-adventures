module Main (main) where

import Blake2bSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
