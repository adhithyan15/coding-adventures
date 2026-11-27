{-# LANGUAGE OverloadedStrings #-}
-- | Deterministic conformance, boundary, and injected-clock tests.
module Main (main) where

import ArrayWorkload
import Control.Monad (forM_, unless)
import Control.Exception (AsyncException(ThreadKilled), try, throw)
import Data.Aeson (FromJSON(..), eitherDecodeStrict', withObject, (.:))
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BSC
import Data.Int (Int64)
import Data.IORef (IORef, newIORef, readIORef, atomicModifyIORef', modifyIORef')
import Data.List (intercalate, isInfixOf)
import Data.Array (listArray)
import Microbenchmark
import System.Exit (exitFailure)
import System.Directory (doesFileExist)
import System.FilePath ((</>))
import System.IO.Unsafe (unsafePerformIO)

assert :: String -> Bool -> IO ()
assert label condition = unless condition (putStrLn ("FAIL: " ++ label) >> exitFailure)

{-# NOINLINE recordForcedBatchValue #-}
recordForcedBatchValue :: IORef [String] -> Int64 -> Int64
recordForcedBatchValue events value = unsafePerformIO $ do
    modifyIORef' events (++ ["batch-result-forced"])
    pure value

{-# NOINLINE scriptedBatch #-}
scriptedBatch :: IORef Int -> IORef [String] -> Int64 -> Either String Int64
scriptedBatch calls events operations = unsafePerformIO $ do
    call <- atomicModifyIORef' calls (\n -> (n + 1, n))
    pure (Right (if call == 2 then recordForcedBatchValue events 0 else 6 * operations))

right :: Show e => Either e a -> IO a
right (Right value) = pure value
right (Left err) = fail (show err)

data Fixture = Fixture Int Double [FixtureCase]
data FixtureCase = FixtureCase Int64 [Int64] Double (Maybe [Double]) String

instance FromJSON Fixture where
    parseJSON = withObject "fixture" $ \o -> Fixture <$> o .: "confidence_percent" <*> o .: "minimum_factor" <*> o .: "cases"
instance FromJSON FixtureCase where
    parseJSON = withObject "case" $ \o -> FixtureCase <$> o .: "baseline_ns" <*> o .: "candidate_ns" <*> o .: "ratio" <*> o .: "interval" <*> o .: "verdict"

main :: IO ()
main = do
    values <- fixtureTests
    collectorTests
    replayTests
    workloadTests
    nativeClockTests
    putStrLn ("microbenchmark: all checks passed (" ++ show values ++ " fixture cases)")

fixtureTests :: IO Int
fixtureTests = do
    let local = "testdata" </> "microbenchmark-pairs-v1.json"
        canonical = "../../../../code/specs/fixtures/microbenchmark-pairs-v1.json"
    bytes <- BS.readFile local
    canonicalExists <- doesFileExist canonical
    if canonicalExists then do
        expectedBytes <- BS.readFile canonical
        assert "shared fixture copied byte-for-byte" (bytes == expectedBytes)
      else pure ()
    Fixture confidence factor cases <- either (fail . show) pure (eitherDecodeStrict' bytes)
    forM_ cases $ \(FixtureCase baseline candidates expectedRatio expectedInterval expectedVerdict) -> do
        let observations = zipWith (\i candidate -> Pair baseline candidate (if even i then BaselineFirst else CandidateFirst)) [0 :: Int ..] candidates
        result <- right (analyze observations confidence factor)
        assert "fixture median ratio" (abs (analysisRatio result - expectedRatio) < 1e-12)
        assert "fixture interval" (analysisInterval result == intervalFromList expectedInterval)
        assert "fixture verdict" (analysisVerdict result == expectedVerdict)
    pure (length cases)
  where
    intervalFromList Nothing = Nothing
    intervalFromList (Just [low, high]) = Just (Interval low high)
    intervalFromList _ = Nothing

collectorTests :: IO ()
collectorTests = do
    ticks <- newIORef (0 :: Int64)
    let clock = atomicModifyIORef' ticks (\n -> (n + 100, n))
        config = defaults { configPairs = 3, configTargetNs = 100, configMaxBatch = 8, configSeed = -17 }
        batch n = Right (6 * n)
    report <- collect config 6 batch batch clock >>= right
    assert "calibrated batch size" (reportBatchSize report == 1)
    assert "retained all raw pairs" (length (reportPairs report) == 3)
    assert "odd schedule has baseline first extra" (length (filter ((== BaselineFirst) . pairOrder) (reportPairs report)) == 2)
    againTicks <- newIORef (0 :: Int64)
    replayed <- collect config 6 batch batch (atomicModifyIORef' againTicks (\n -> (n + 100, n))) >>= right
    assert "same seeded order replays" (map pairOrder (reportPairs report) == map pairOrder (reportPairs replayed))

    bad <- collect config 6 (const (Right 0)) batch clock
    assert "warmup checksum mismatch fails" (isLeft bad)
    tinyTicks <- newIORef (0 :: Int64)
    exhausted <- collect (config { configPairs = 1, configTargetNs = 1000, configMaxBatch = 1 }) 6 batch batch
        (atomicModifyIORef' tinyTicks (\n -> (n + 10, n)))
    assert "calibration exhaustion fails" (isLeft exhausted)
    overflowTicks <- newIORef (0 :: Int64)
    overflow <- collect (config { configPairs = 1, configTargetNs = 200, configMaxBatch = 2 }) maxBound (const (Right maxBound)) (const (Right maxBound))
        (atomicModifyIORef' overflowTicks (\n -> (n + 100, n)))
    assert "checksum multiplication overflow fails" (isLeft overflow)
    broken <- collect (config { configPairs = 1, configMaxBatch = 1 }) 6 (const (Left "workload failed")) batch clock
    assert "workload error fails without report" (isLeft broken)

    thrown <- collect (config { configPairs = 1, configMaxBatch = 1 }) 6 (const (error "callback exploded")) batch clock
    assert "callback exception becomes an error without a report" (isLeft thrown)
    cancelled <- try (collect (config { configPairs = 1, configMaxBatch = 1 }) 6 (const (throw ThreadKilled)) batch clock)
    assert "asynchronous cancellation escapes callback boundary" (case cancelled of Left ThreadKilled -> True; Left _ -> False; Right _ -> False)

    events <- newIORef []
    calls <- newIORef 0
    clockReads <- newIORef (0 :: Int)
    let orderedClock = do
            readIndex <- atomicModifyIORef' clockReads (\n -> (n + 1, n))
            modifyIORef' events (++ ["clock"])
            pure (fromIntegral (readIndex * 100))
    ordered <- collect (config { configPairs = 1, configTargetNs = 1, configMaxBatch = 1 }) 6
        (scriptedBatch calls events) batch orderedClock
    eventLog <- readIORef events
    clockReadCount <- readIORef clockReads
    assert "measured checksum mismatch fails" (isLeft ordered)
    assert "measured result is forced before stop clock and checked after it"
        (clockReadCount == 6 && drop (length eventLog - 2) eventLog == ["batch-result-forced", "clock"])

    zeroClock <- scriptedClock [0, 100, 100, 200, 200, 200]
    zeroMeasured <- collect (config { configPairs = 1, configMaxBatch = 1 }) 6 batch batch zeroClock
    assert "zero measured duration fails" (isLeft zeroMeasured)
    regressingClock <- scriptedClock [0, 100, 90]
    regressed <- collect (config { configPairs = 1, configMaxBatch = 1 }) 6 batch batch regressingClock
    assert "regressing clock fails" (isLeft regressed)

replayTests :: IO ()
replayTests = do
    let config = defaults { configPairs = 1, configTargetNs = 1, configMaxBatch = maxBound, configSeed = minBound }
        pair = Pair maxBound maxBound BaselineFirst
    analysis <- right (analyze [pair] 95 1.05)
    let metadata = defaultMetadata ["--pairs", "1"] "revision" "ghc 9.4.8" "windows" "x86_64"
        report = Report config 1 [pair] analysis (Just metadata)
        bytes = encodeReport report
    assert "i64 max serialized as decimal token" ("9223372036854775807" `isInfixOf` BSC.unpack bytes)
    assert "i64 min serialized as decimal token" ("-9223372036854775808" `isInfixOf` BSC.unpack bytes)
    parsed <- right (replayReport bytes)
    assert "signed64 report roundtrip" (parsed == report)
    let duplicate = BSC.pack (replaceOnce "\"seed\":-9223372036854775808" "\"seed\":-9223372036854775808,\"seed\":0" (BSC.unpack bytes))
        fractional = BSC.pack (replaceOnce "\"seed\":-9223372036854775808" "\"seed\":-9223372036854775808.0" (BSC.unpack bytes))
        partial = BSC.pack (replaceOnce "\"workload\":\"indexed-array-sum-4096\"" "\"workload\":null" (BSC.unpack bytes))
    assert "duplicate key rejected" (isLeft (replayReport duplicate))
    assert "fractional i64 spelling rejected" (isLeft (replayReport fractional))
    assert "present null metadata rejected" (isLeft (replayReport partial))
    let withWorkload token = BSC.pack (replaceOnce "\"workload\":\"indexed-array-sum-4096\"" ("\"workload\":" ++ token) (BSC.unpack bytes))
        acceptedWorkloads =
            [ ("\"indexed-array-sum-4096\"", "indexed-array-sum-4096")
            , ("\"dependent-link-order\"", "dependent-link-order")
            , ("\"\\u2003\"", "\x2003")
            , ("\"\\ud83d\\udca1\"", "\x1f4a1")
            , ("\"\\ufffd\"", "\xfffd")
            , ("\"\\\\ud800\"", "\\ud800")
            ]
        invalidWorkloads =
            [ "\"\"", "\" \"", "\"\\t\"", "\"\\n\"", "\"\\u000b\"", "\"\\f\"", "\"\\r\""
            , "\"\\u0000\"", "\"bad\\u0000id\"", "null", "17", "true", "[]", "{}"
            , "\"\\ud800\"", "\"\\udfff\""
            ]
    forM_ acceptedWorkloads $ \(token, expected) ->
        assert ("workload preserved: " ++ expected)
            (case replayReport (withWorkload token) of
                Right decoded -> (metadataWorkload <$> reportMetadata decoded) == Just expected
                Left _ -> False)
    forM_ invalidWorkloads $ \token ->
        assert ("invalid workload rejected: " ++ token) (isLeft (replayReport (withWorkload token)))
    assert "metadata-free report roundtrip" (isRight (replayReport (encodeReport (report { reportMetadata = Nothing }))))
    let emptyArg = report { reportMetadata = Just metadata { metadataInvocation = [""] } }
        nulMetadata = report { reportMetadata = Just metadata { metadataRevision = "bad\0value" } }
        nulArg = report { reportMetadata = Just metadata { metadataInvocation = ["bad\0value"] } }
        nulProfile = report { reportMetadata = Just metadata { metadataProfile = Just "bad\0value" } }
        emptyProfile = report { reportMetadata = Just metadata { metadataProfile = Just "" } }
        whitespaceRevision = report { reportMetadata = Just metadata { metadataRevision = " " } }
    assert "empty argv element is valid" (isRight (replayReport (encodeReport emptyArg)))
    assert "NUL in descriptive metadata rejected" (isLeft (replayReport (encodeReport nulMetadata)))
    assert "NUL in invocation rejected" (isLeft (replayReport (encodeReport nulArg)))
    assert "NUL in profile rejected" (isLeft (replayReport (encodeReport nulProfile)))
    assert "empty profile rejected" (isLeft (replayReport (encodeReport emptyProfile)))
    assert "whitespace-only descriptive string remains valid" (isRight (replayReport (encodeReport whitespaceRevision)))
    let unknownField = BSC.pack (replaceOnce "\"batch_size\":1" "\"extra\":1,\"batch_size\":1" (BSC.unpack bytes))
        oversized = BS.replicate (4 * 1024 * 1024 + 1) 32
        tooDeep = BSC.pack (replicate 129 '[' ++ replicate 129 ']')
    assert "unknown field rejected" (isLeft (replayReport unknownField))
    assert "size limit enforced" (isLeft (replayReport oversized))
    assert "nesting bound checked before JSON parse" (isLeft (replayReport tooDeep))
    let tooManyValues = BSC.pack ("[" ++ intercalate "," (replicate 65536 "0") ++ "]")
    assert "JSON value budget is enforced before decoding" (case replayReport tooManyValues of
        Left message -> message == "report exceeds JSON value limit"
        Right _ -> False)
    let maximumPairs =
            [ Pair 1 1 (if even index then BaselineFirst else CandidateFirst)
            | index <- [0..9999]
            ]
    assert "analysis accepts 10000 pairs" (isRight (analyze maximumPairs 95 1.05))
    assert "analysis rejects 10001 pairs" (isLeft (analyze (maximumPairs ++ [pair]) 95 1.05))
    assert "analysis rejects infinite input after a bounded prefix" (isLeft (analyze (repeat pair) 95 1.05))
    maximumAnalysis <- right (analyze maximumPairs 95 1.05)
    let maximumConfig = config { configPairs = 10000 }
        maximumMetadata = metadata { metadataInvocation = replicate 256 "arg" }
        maximumReport = Report maximumConfig 1 maximumPairs maximumAnalysis (Just maximumMetadata)
    assert "maximum metadata report stays within the exact JSON value budget"
        (isRight (replayReport (encodeReport maximumReport)))
    let unbalancedConfig = config { configPairs = 2 }
        unbalancedPairs = [pair, pair { pairOrder = BaselineFirst }]
    unbalancedAnalysis <- right (analyze unbalancedPairs 95 1.05)
    assert "unbalanced replay rejected" (isLeft (replayReport (encodeReport (Report unbalancedConfig 1 unbalancedPairs unbalancedAnalysis Nothing))))

workloadTests :: IO ()
workloadTests = do
    let custom = listArray (4, 7) [21, -3, 8, 19]
    assert "indexed helper traverses supplied array" (indexedBatch custom 2 == Right 90)
    assert "iterator helper traverses supplied array" (iterationBatch custom 2 == Right 90)
    assert "zero operations" (indexedBatch custom 0 == Right 0 && iterationBatch custom 0 == Right 0)
    assert "negative operations rejected" (isLeft (indexedBatch custom (-1)) && isLeft (iterationBatch custom (-1)))
    let large = listArray (0, 1) [maxBound :: Int, 1]
    assert "checksum overflow rejected" (isLeft (indexedBatch large 1))
    assert "known production checksum" (indexedBatch makeArray 1 == Right expectedChecksum && iterationBatch makeArray 1 == Right expectedChecksum)

nativeClockTests :: IO ()
nativeClockTests = do
    clock <- monotonicClock
    first <- clock
    second <- clock
    assert "native monotonic clock produces nonnegative ordered elapsed ticks" (first >= 0 && second >= first)

scriptedClock :: [Int64] -> IO Clock
scriptedClock [] = pure (pure 0)
scriptedClock ticks = do
    remaining <- newIORef ticks
    pure $ atomicModifyIORef' remaining $ \values -> case values of
        [] -> ([], last ticks)
        value : rest -> (rest, value)

replaceOnce :: String -> String -> String -> String
replaceOnce needle replacement source = case splitAtNeedle source of
    Nothing -> source
    Just (before, after) -> before ++ replacement ++ after
  where
    splitAtNeedle [] = Nothing
    splitAtNeedle text | needle `isPrefixOf` text = Just ([], drop (length needle) text)
    splitAtNeedle (ch : rest) = do
        (before, after) <- splitAtNeedle rest
        pure (ch : before, after)
    isPrefixOf prefix text = take (length prefix) text == prefix

isLeft :: Either a b -> Bool
isLeft (Left _) = True
isLeft _ = False
isRight :: Either a b -> Bool
isRight (Right _) = True
isRight _ = False
