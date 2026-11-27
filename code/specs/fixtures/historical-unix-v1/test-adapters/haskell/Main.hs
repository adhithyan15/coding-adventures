{-# LANGUAGE OverloadedStrings #-}
module Main where

import Data.Aeson (encode)
import qualified Data.ByteString.Lazy.Char8 as L8
import System.Environment (getArgs)
import UnixHistory (runCommand)

main :: IO ()
main = getArgs >>= L8.putStrLn . encode . runCommand . unwords
