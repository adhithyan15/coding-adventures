{-# LANGUAGE OverloadedStrings #-}
module CorpusSpec
  ( runCorpusTests
  ) where

import Control.Monad (forM_)
import qualified Data.ByteString as BS
import qualified Data.List as List
import qualified Data.Text as T
import System.IO (hPutStrLn, stderr)

import CodingAdventures.CanonicalCbor
import SimpleJson

runCorpusTests :: IO Int
runCorpusTests = do
  cases <- loadCorpus
  if List.length cases /= 55
    then ioError (userError ("expected 55 test cases, got " ++ show (List.length cases)))
    else do
      forM_ cases runTestCase
      return (List.length cases)

runTestCase :: TestCase -> IO ()
runTestCase tc =
  case testOperation tc of
    "round-trip" -> do
      let inBytes = hexToBytes (testInput tc)
      case decode inBytes of
        Left err -> failTest tc ("decode failed: " ++ show err)
        Right val ->
          case encodeChecked val of
            Left err -> failTest tc ("encodeChecked failed: " ++ show err)
            Right outBytes -> do
              let hexOut = bytesToHex outBytes
              if hexOut /= testExpected tc
                then failTest tc ("encoded hex mismatch: expected " ++ testExpected tc ++ ", got " ++ hexOut)
                else
                  case decode outBytes of
                    Left err -> failTest tc ("re-decode failed: " ++ show err)
                    Right val2 ->
                      if val2 /= val
                        then failTest tc "round-trip value mismatch"
                        else return ()

    "encode-map" -> do
      let pairs = splitOn ';' (testInput tc)
          parsePair p =
            let (kHex, vHex) = splitPair '=' p
            in case (decode (hexToBytes kHex), decode (hexToBytes vHex)) of
                 (Right k, Right v) -> Right (CborMapEntry k v)
                 (Left err, _)      -> Left ("key decode error: " ++ show err)
                 (_, Left err)      -> Left ("val decode error: " ++ show err)
      case mapM parsePair pairs of
        Left err -> failTest tc err
        Right entries ->
          case encodeChecked (CborMap entries) of
            Left err -> failTest tc ("encodeChecked map failed: " ++ show err)
            Right outBytes ->
              let hexOut = bytesToHex outBytes
              in if hexOut /= testExpected tc
                   then failTest tc ("map encoded mismatch: expected " ++ testExpected tc ++ ", got " ++ hexOut)
                   else return ()

    "generated-round-trip" ->
      case testInput tc of
        "nested-array:128" -> do
          let val = buildNestedArray 128
              wire = buildNestedArrayWire 128
          case encodeChecked val of
            Left err -> failTest tc ("encode 128 failed: " ++ show err)
            Right enc ->
              if enc /= wire
                then failTest tc "wire mismatch for nested-array:128"
                else
                  case decode wire of
                    Left err -> failTest tc ("decode 128 failed: " ++ show err)
                    Right dec ->
                      if dec /= val
                        then failTest tc "value mismatch for nested-array:128"
                        else return ()

        "bytes-repeat:1048571:00" -> do
          let payload = BS.replicate 1048571 0
              val = CborBytes payload
          case encodeChecked val of
            Left err -> failTest tc ("encode bytes failed: " ++ show err)
            Right enc ->
              if BS.length enc /= 1048576
                then failTest tc ("expected size 1048576, got " ++ show (BS.length enc))
                else
                  case decode enc of
                    Left err -> failTest tc ("decode bytes failed: " ++ show err)
                    Right dec ->
                      if dec /= val
                        then failTest tc "value mismatch for bytes-repeat:1048571:00"
                        else return ()

        other -> failTest tc ("unknown generated input: " ++ other)

    "decode-error" -> do
      let wire = if testInput tc == "nested-array-wire:129"
                   then buildNestedArrayWire 129
                   else hexToBytes (testInput tc)
      case decode wire of
        Right val -> failTest tc ("expected decode error, succeeded with " ++ show val)
        Left err -> do
          let actualId = idToText (cborErrorId err)
              expectedId = T.pack (testExpected tc)
          if actualId /= expectedId
            then failTest tc ("expected error id " ++ T.unpack expectedId ++ ", got " ++ T.unpack actualId)
            else if not ("canonical-cbor: " `T.isPrefixOf` cborErrorMessage err)
              then failTest tc ("missing diagnostic prefix: " ++ T.unpack (cborErrorMessage err))
              else return ()

    "encode-error" -> do
      val <- case testInput tc of
        "duplicate-map-key" ->
          return (CborMap
            [ CborMapEntry (CborText "dup") (CborUnsigned 1)
            , CborMapEntry (CborText "dup") (CborUnsigned 2)
            ])
        "nested-array:129" ->
          return (buildNestedArray 129)
        "bytes-repeat:1048572:00" ->
          return (CborBytes (BS.replicate 1048572 0))
        other -> ioError (userError ("unknown encode-error input: " ++ other))

      case encodeChecked val of
        Right enc -> failTest tc ("expected encode error, succeeded with " ++ show (BS.length enc) ++ " bytes")
        Left err -> do
          let actualId = idToText (cborErrorId err)
              expectedId = T.pack (testExpected tc)
          if actualId /= expectedId
            then failTest tc ("expected error id " ++ T.unpack expectedId ++ ", got " ++ T.unpack actualId)
            else if not ("canonical-cbor: " `T.isPrefixOf` cborErrorMessage err)
              then failTest tc ("missing diagnostic prefix: " ++ T.unpack (cborErrorMessage err))
              else return ()

    other -> failTest tc ("unknown operation: " ++ other)

buildNestedArray :: Int -> CborValue
buildNestedArray n = List.foldl' (\acc _ -> CborArray [acc]) (CborArray []) [1..n]

buildNestedArrayWire :: Int -> BS.ByteString
buildNestedArrayWire n = BS.replicate n 0x81 `BS.snoc` 0x80

failTest :: TestCase -> String -> IO a
failTest tc reason = do
  hPutStrLn stderr ("FAILED [" ++ testId tc ++ "]: " ++ reason)
  ioError (userError ("Test " ++ testId tc ++ " failed: " ++ reason))

splitOn :: Char -> String -> [String]
splitOn _ "" = []
splitOn delim s =
  case break (== delim) s of
    (chunk, "")       -> [chunk]
    (chunk, _:rest)   -> chunk : splitOn delim rest

splitPair :: Char -> String -> (String, String)
splitPair delim s =
  case break (== delim) s of
    (k, _:v) -> (k, v)
    (k, [])  -> (k, "")
