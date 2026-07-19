module Main (main) where

import qualified DeflateSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec DeflateSpec.spec
