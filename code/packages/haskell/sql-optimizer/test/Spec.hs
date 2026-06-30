-- Spec.hs — hspec entry point for sql-optimizer tests.
module Main where

import Test.Hspec
import SqlOptimizerSpec (spec)

main :: IO ()
main = hspec spec
