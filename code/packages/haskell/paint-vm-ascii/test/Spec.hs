module Main (main) where

import Test.Hspec (hspec)
import qualified PaintVmAsciiSpec

main :: IO ()
main = hspec PaintVmAsciiSpec.spec
