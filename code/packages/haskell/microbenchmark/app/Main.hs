{-# LANGUAGE ScopedTypeVariables #-}
-- | Native command line entry point for collection and replay.
module Main (main) where

import ArrayWorkload
import Control.Exception (IOException, try)
import qualified Data.ByteString as BS
import Data.Int (Int64)
import Data.Char (isSpace)
import Data.Version (showVersion)
import Microbenchmark
import System.Environment (getArgs, getProgName, lookupEnv)
import System.Exit (exitFailure)
import System.Info (arch, compilerName, compilerVersion, os)
import System.IO (IOMode(ReadMode), hFlush, stdout, withBinaryFile)
import Text.Read (readMaybe)

main :: IO ()
main = do
    args <- getArgs
    result <- try (run args) :: IO (Either IOException Bool)
    case result of
        Left err -> putStrLn ("error: " ++ show err) >> exitFailure
        Right True -> pure ()
        Right False -> exitFailure

run :: [String] -> IO Bool
run ["--help"] = do
    program <- getProgName
    putStrLn ("Usage: " ++ program ++ " [--pairs N] [--confidence N] [--factor F] [--seed I64] [--target-ns I64] [--max-batch I64] [--revision TEXT] | --replay FILE")
    hFlush stdout
    pure True
run ["--replay", path] = do
    content <- withBinaryFile path ReadMode (\handle -> BS.hGet handle (4 * 1024 * 1024 + 1))
    case replayReport content of
        Left err -> putStrLn ("error: " ++ err) >> pure False
        Right report -> putStrLn ("replay valid: " ++ show (length (reportPairs report)) ++ " pairs, ratio=" ++ show (analysisRatio (reportAnalysis report))) >> hFlush stdout >> pure True
run args = case parseOptions defaults Nothing args of
    Left err -> putStrLn ("error: " ++ err) >> pure False
    Right (config, explicitRevision) -> do
        revision <- case explicitRevision of
            Just value -> pure value
            Nothing -> do
                primary <- lookupEnv "GIT_REVISION"
                fallback <- lookupEnv "GITHUB_SHA"
                pure (firstNonblank [primary, fallback, Just "unknown"])
        clock <- monotonicClock
        let values = makeArray
            baseline = indexedBatch values
            candidate = iterationBatch values
        measured <- collect config expectedChecksum baseline candidate clock
        case measured of
            Left err -> putStrLn ("error: " ++ err) >> pure False
            Right report -> do
                args' <- getArgs
                let platform = if os == "mingw32" then "windows" else os
                    metadata = defaultMetadata args' revision (compilerName ++ " " ++ showVersion compilerVersion) platform arch
                    bytes = encodeReport (report { reportMetadata = Just metadata })
                BS.putStr bytes
                BS.putStr (BS.singleton 10)
                hFlush stdout
                pure True

firstNonblank :: [Maybe String] -> String
firstNonblank values = case [value | Just value <- values, not (all isSpace value)] of
    value : _ -> value
    [] -> "unknown"

parseOptions :: Config -> Maybe String -> [String] -> Either String (Config, Maybe String)
parseOptions config revision [] = validate config >> Right (config, revision)
parseOptions _ _ ("--revision" : value : _) | all isSpace value = Left "--revision must not be blank"
parseOptions _ _ ("--revision" : []) = Left "missing value for --revision"
parseOptions config revision (flag : value : rest) = case flag of
    "--pairs" -> intOption flag value (\n -> config { configPairs = n })
    "--confidence" -> intOption flag value (\n -> config { configConfidencePercent = n })
    "--factor" -> doubleOption flag value (\n -> config { configMinimumFactor = n })
    "--seed" -> int64Option flag value (\n -> config { configSeed = n })
    "--target-ns" -> int64Option flag value (\n -> config { configTargetNs = n })
    "--max-batch" -> int64Option flag value (\n -> config { configMaxBatch = n })
    "--revision" -> parseOptions config (Just value) rest
    _ -> Left ("unknown option: " ++ flag)
  where
    intOption name raw update = maybe (Left ("invalid integer for " ++ name)) (\n -> parseOptions (update n) revision rest) (readMaybe raw)
    int64Option name raw update = maybe (Left ("invalid signed 64-bit integer for " ++ name)) (\n -> parseOptions (update n) revision rest) (readMaybe raw :: Maybe Int64)
    doubleOption name raw update = maybe (Left ("invalid number for " ++ name)) (\n -> parseOptions (update n) revision rest) (readMaybe raw)
parseOptions _ _ [flag] = Left ("missing value for " ++ flag)

validate :: Config -> Either String ()
validate c
    | configPairs c < 1 || configPairs c > 10000 = Left "pairs must be from 1 to 10000"
    | configConfidencePercent c < 1 || configConfidencePercent c > 99 = Left "confidence must be from 1 to 99"
    | configMinimumFactor c <= 1 || isNaN (configMinimumFactor c) || isInfinite (configMinimumFactor c) = Left "factor must be finite and greater than 1"
    | configTargetNs c <= 0 || configMaxBatch c <= 0 = Left "target and max batch must be positive"
    | otherwise = Right ()
