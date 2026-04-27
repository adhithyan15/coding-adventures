module Main where

import System.Environment (getArgs)
import System.Exit (ExitCode(..), exitWith)

import ScaffoldGenerator (runWithArgs)

main :: IO ()
main = do
    args <- getArgs
    exitCode <- runWithArgs args
    exitWith (if exitCode == 0 then ExitSuccess else ExitFailure exitCode)
