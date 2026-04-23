module Main (main) where

import AesSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
