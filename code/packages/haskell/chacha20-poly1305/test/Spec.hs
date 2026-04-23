module Main (main) where

import Chacha20Poly1305Spec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
