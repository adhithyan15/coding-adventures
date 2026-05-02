module Main (main) where

import Test.Hspec
import qualified SqlBackendSpec

main :: IO ()
main = hspec SqlBackendSpec.spec
