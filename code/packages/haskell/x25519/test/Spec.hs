module Main (main) where

import Test.Hspec (hspec)
import X25519Spec (spec)

main :: IO ()
main = hspec spec
