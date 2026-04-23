module Main (main) where

import AesModesSpec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
