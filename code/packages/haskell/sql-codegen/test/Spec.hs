-- Spec.hs — hspec entry point for sql-codegen tests.
module Main where

import Test.Hspec
import SqlCodegenSpec (spec)

main :: IO ()
main = hspec spec
