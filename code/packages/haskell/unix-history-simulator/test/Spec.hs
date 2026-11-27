{-# LANGUAGE OverloadedStrings #-}
module Main where

import Data.Aeson (Value(..))
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import qualified Data.Text as T
import qualified Data.Vector as V
import System.Exit (exitFailure)
import UnixHistory

main :: IO ()
main = do
  let (openedState, opened) = openFile newKernel "/hello"
      (afterFirstRead, firstRead, bytes) = readFileAt openedState 0 8
      (independentState, secondOpen) = openFile afterFirstRead "/hello"
      (_, independentRead, independentBytes) = readFileAt independentState 1 8
      (closedState, closed) = closeFile afterFirstRead 0
      (_, reused) = openFile closedState "/empty"
      missing = snd (openFile newKernel "/missing")
      directoryOpen = snd (openFile newKernel "/etc")
      listed = listDirectory newKernel "/"
      badList = listDirectory newKernel "/hello"
      result = runCommand "cat /hello"
      checks =
        [ bytes == "Hello, U"
        , fieldInt "fd" opened == Just 0
        , fieldInt "nextOffset" firstRead == Just 8
        , fieldInt "fd" secondOpen == Just 1
        , independentBytes == "Hello, U"
        , fieldInt "offset" independentRead == Just 0
        , fieldInt "fd" reused == Just 0
        , fieldInt "fd" closed == Just 0
        , fieldString "error" missing == Just "ENOENT"
        , fieldString "error" directoryOpen == Just "EISDIR"
        , fieldNames listed == ["hello", "empty", "etc"]
        , fieldString "error" badList == Just "ENOTDIR"
        , fieldString "output" result == Just "Hello, Unix!\n"
        , eventCount result == Number 5
        , fieldString "output" (runCommand "  \t") == Just ""
        ]
  if and checks then putStrLn "Haskell simulator component tests passed" else exitFailure

fieldInt :: String -> Value -> Maybe Int
fieldInt key (Object objectValue) =
  case KM.lookup (K.fromString key) objectValue of
    Just (Number number) -> Just (round number)
    _ -> Nothing
fieldInt _ _ = Nothing

fieldString :: String -> Value -> Maybe String
fieldString key (Object objectValue) =
  case KM.lookup (K.fromString key) objectValue of
    Just (String value) -> Just (T.unpack value)
    _ -> Nothing
fieldString _ _ = Nothing

fieldNames :: Value -> [String]
fieldNames (Object objectValue) =
  case KM.lookup "names" objectValue of
    Just (Array values) -> [T.unpack value | String value <- V.toList values]
    _ -> []
fieldNames _ = []

eventCount :: Value -> Value
eventCount (Object objectValue) =
  case KM.lookup "events" objectValue of
    Just (Array values) -> Number (fromIntegral (V.length values))
    _ -> Null
eventCount _ = Null
