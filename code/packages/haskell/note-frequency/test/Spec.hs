module Main where

import Test.Hspec
import qualified NoteFrequencySpec

main :: IO ()
main = hspec NoteFrequencySpec.spec
