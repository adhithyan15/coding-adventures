module Main (main) where

import Ed25519Spec (spec)
import Test.Hspec (hspec)

main :: IO ()
main = hspec spec
