{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE FlexibleContexts #-}
-- | Paired batch collection, analysis, and strict portable report replay.
module Microbenchmark
    ( Config(..), defaults, Order(..), Pair(..), Metadata(..), Interval(..)
    , Analysis(..), Report(..), Batch, Clock, analyze, collect
    , encodeReport, replayReport, defaultMetadata
    ) where

import Data.Aeson (Value(..), eitherDecodeStrict', encode, object, (.=))
import Control.Monad (foldM)
import Control.Exception (SomeAsyncException, SomeException, displayException, evaluate, fromException, tryJust)
import Data.Array.ST (newListArray, readArray, runSTArray, writeArray)
import Data.Array (elems)
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BSC
import qualified Data.ByteString.Lazy as BL
import Data.IORef (newIORef, readIORef, writeIORef)
import Data.Int (Int64)
import qualified Data.Set as Set
import qualified Data.Text as T
import qualified Data.Vector as V
import Data.Scientific (toBoundedInteger, toRealFloat)
import Data.Word (Word64)
import RankStatistics (Interval(..), median, medianInterval)

data Config = Config { configPairs :: Int, configConfidencePercent :: Int, configMinimumFactor :: Double, configSeed :: Int64, configTargetNs :: Int64, configMaxBatch :: Int64 } deriving (Eq, Show)
defaults :: Config
defaults = Config 40 95 1.05 0 100000 65536

data Order = BaselineFirst | CandidateFirst deriving (Eq, Show)
data Pair = Pair { baselineNs :: Int64, candidateNs :: Int64, pairOrder :: Order } deriving (Eq, Show)
data Metadata = Metadata { metadataWorkload :: String, metadataInvocation :: [String], metadataRevision :: String, metadataRuntime :: String, metadataOs :: String, metadataArch :: String, metadataProfile :: Maybe String } deriving (Eq, Show)
data Analysis = Analysis { analysisRatio :: Double, analysisInterval :: Maybe Interval, analysisVerdict :: String } deriving (Eq, Show)
data Report = Report { reportConfig :: Config, reportBatchSize :: Int64, reportPairs :: [Pair], reportAnalysis :: Analysis, reportMetadata :: Maybe Metadata } deriving (Eq, Show)
type Batch = Int64 -> Either String Int64
type Clock = IO Int64

isFinite :: Double -> Bool
isFinite value = not (isNaN value || isInfinite value)

validateConfig :: Config -> Either String ()
validateConfig c
    | configPairs c < 1 || configPairs c > 10000 = Left "pairs must be from 1 to 10000"
    | configConfidencePercent c < 1 || configConfidencePercent c > 99 = Left "confidence must be from 1 to 99"
    | not (isFinite (configMinimumFactor c)) || configMinimumFactor c <= 1 = Left "factor must be finite and greater than 1"
    | configTargetNs c <= 0 || configMaxBatch c <= 0 = Left "target and max batch must be positive"
    | otherwise = Right ()

analyze :: [Pair] -> Int -> Double -> Either String Analysis
analyze pairs confidence factor = do
    if null pairs then Left "at least one pair is required" else Right ()
    if not (atMostPairLimit pairs) then Left "at most 10000 pairs are supported" else Right ()
    if confidence < 1 || confidence > 99 then Left "confidence must be from 1 to 99" else Right ()
    if not (isFinite factor) || factor <= 1 then Left "factor must be finite and greater than 1" else Right ()
    ratios <- mapM ratio pairs
    center <- median ratios
    ranks <- medianInterval ratios confidence
    let verdict = case ranks of
            Nothing -> "inconclusive"
            Just ci | intervalHigh ci < 1 / factor -> "faster"
                    | intervalLow ci > factor -> "slower"
                    | intervalLow ci >= 1 / factor && intervalHigh ci <= factor -> "equivalent"
                    | otherwise -> "inconclusive"
    pure (Analysis center ranks verdict)
  where
    ratio p | baselineNs p <= 0 || candidateNs p <= 0 = Left "pair durations must be positive"
            | pairOrder p `notElem` [BaselineFirst, CandidateFirst] = Left "invalid pair order"
            | otherwise = Right (fromIntegral (candidateNs p) / fromIntegral (baselineNs p))

-- Inspect no more than 10,001 cons cells, even for an infinite input list.
atMostPairLimit :: [a] -> Bool
atMostPairLimit = go 0
  where
    go count _ | count > 10000 = False
    go _ [] = True
    go count (_:rest) = go (count + 1) rest

collect :: Config -> Int64 -> Batch -> Batch -> Clock -> IO (Either String Report)
collect config checksum baseline candidate clock = case validateConfig config of
    Left err -> pure (Left err)
    Right () -> do
        warmupBaseline <- verify baseline 1
        warmup <- case warmupBaseline of
            Left err -> pure (Left err)
            Right () -> verify candidate 1
        case warmup of
            Left err -> pure (Left err)
            Right () -> do
                lastTick <- newIORef (-1 :: Int64)
                let readClock = do
                        now <- clock
                        previous <- readIORef lastTick
                        if now < 0 || (previous >= 0 && now < previous)
                            then pure (Left "clock is negative or regressed")
                            else writeIORef lastTick now >> pure (Right now)
                    measure fn count calibration = do
                        case expected count of
                            Left err -> pure (Left err)
                            Right wanted -> do
                                before <- readClock
                                case before of
                                    Left err -> pure (Left err)
                                    Right start -> do
                                        actual <- runBatch fn count
                                        after <- readClock
                                        pure $ do
                                            value <- actual
                                            finish <- after
                                            if value /= wanted then Left "checksum mismatch"
                                            else if finish < start || (not calibration && finish == start)
                                                then Left "duration must be positive and clock must not regress"
                                                else Right (finish - start)
                    calibrate size = do
                        a <- measure baseline size True
                        b <- measure candidate size True
                        case (a, b) of
                            (Right da, Right db) | da >= configTargetNs config && db >= configTargetNs config -> pure (Right size)
                            (Left err, _) -> pure (Left err)
                            (_, Left err) -> pure (Left err)
                            _ | size >= configMaxBatch config -> pure (Left "calibration limit exhausted")
                              | otherwise -> calibrate (if size > configMaxBatch config `div` 2 then configMaxBatch config else size * 2)
                chosen <- calibrate 1
                case chosen of
                    Left err -> pure (Left err)
                    Right size -> do
                        let orders = shuffle (map even [0 .. configPairs config - 1]) (fromIntegral (configSeed config))
                        values <- mapM (measurePair size measure baseline candidate) orders
                        case sequence values of
                            Left err -> pure (Left err)
                            Right pairs -> pure $ do
                                analysis <- analyze pairs (configConfidencePercent config) (configMinimumFactor config)
                                Right (Report config size pairs analysis Nothing)
  where
    expected n = let value = toInteger checksum * toInteger n in
        if value < toInteger (minBound :: Int64) || value > toInteger (maxBound :: Int64)
            then Left "checksum arithmetic overflow" else Right (fromInteger value)
    verify fn n = case expected n of
        Left err -> pure (Left err)
        Right wanted -> do
            actual <- runBatch fn n
            pure $ do
                value <- actual
                if value == wanted then Right () else Left "checksum mismatch"
    runBatch fn n = do
        result <- (tryJust synchronousException $ do
            returned <- evaluate (fn n)
            case returned of
                Left err -> evaluate (length err) >> pure (Left err)
                Right value -> evaluate value >> pure (Right value))
            :: IO (Either SomeException (Either String Int64))
        pure $ case result of
            Left exception -> Left ("batch callback failed: " ++ displayException exception)
            Right value -> value
        where
            synchronousException exception = case fromException exception :: Maybe SomeAsyncException of
                Just _ -> Nothing
                Nothing -> Just exception

measurePair :: Int64 -> (Batch -> Int64 -> Bool -> IO (Either String Int64)) -> Batch -> Batch -> Bool -> IO (Either String Pair)
measurePair size measure baseline candidate baseFirst = do
    let first = if baseFirst then baseline else candidate
        second = if baseFirst then candidate else baseline
    a <- measure first size False
    case a of
        Left err -> pure (Left err)
        Right firstDuration -> do
            b <- measure second size False
            pure $ do
                secondDuration <- b
                let baseDuration = if baseFirst then firstDuration else secondDuration
                    candidateDuration = if baseFirst then secondDuration else firstDuration
                    order = if baseFirst then BaselineFirst else CandidateFirst
                Right (Pair baseDuration candidateDuration order)

-- The odd-sized schedule gives baseline one extra first position. Fisher-Yates
-- swaps consume a local wrapping LCG state, so this is repeatable per seed.
shuffle :: [Bool] -> Word64 -> [Bool]
shuffle [] _ = []
shuffle input initial = elems (runSTArray $ do
    values <- newListArray (0, length input - 1) input
    let go i state
            | i <= 0 = pure ()
            | otherwise = do
                let next = state * 6364136223846793005 + 1442695040888963407
                    j = fromIntegral (next `mod` fromIntegral (i + 1))
                a <- readArray values i
                b <- readArray values j
                writeArray values i b
                writeArray values j a
                go (i - 1) next
    go (length input - 1) initial
    pure values)

defaultMetadata :: [String] -> String -> String -> String -> String -> Metadata
defaultMetadata invocation revision runtime os arch = Metadata "indexed-array-sum-4096" invocation revision runtime os arch Nothing

encodeReport :: Report -> BS.ByteString
encodeReport = BL.toStrict . encode . reportValue

reportValue :: Report -> Value
reportValue report = object (standard ++ metadataFields)
  where
    c = reportConfig report; a = reportAnalysis report
    standard =
        [ "config" .= object ["pairs" .= configPairs c, "confidence_percent" .= configConfidencePercent c, "minimum_factor" .= configMinimumFactor c, "seed" .= configSeed c, "target_ns" .= configTargetNs c, "max_batch" .= configMaxBatch c]
        , "batch_size" .= reportBatchSize report
        , "pairs" .= [object ["baseline_ns" .= baselineNs p, "candidate_ns" .= candidateNs p, "order" .= orderText (pairOrder p)] | p <- reportPairs report]
        , "analysis" .= object ["ratio" .= analysisRatio a, "interval" .= fmap (\ci -> [intervalLow ci, intervalHigh ci]) (analysisInterval a), "verdict" .= analysisVerdict a]
        ]
    metadataFields = case reportMetadata report of
        Nothing -> []
        Just m -> ["workload" .= metadataWorkload m, "invocation" .= metadataInvocation m, "revision" .= metadataRevision m, "runtime" .= metadataRuntime m, "os" .= metadataOs m, "arch" .= metadataArch m] ++ maybe [] (\p -> ["profile" .= p]) (metadataProfile m)

orderText :: Order -> String
orderText BaselineFirst = "baseline-first"
orderText CandidateFirst = "candidate-first"

replayReport :: BS.ByteString -> Either String Report
replayReport bytes
    | BS.length bytes > 4 * 1024 * 1024 = Left "report exceeds 4 MiB limit"
    | otherwise = do
        preflightKeys bytes
        value <- either (Left . ("invalid JSON: " ++)) Right (eitherDecodeStrict' bytes)
        _ <- checkTree 0 1 value
        report <- decodeReport value
        let expectedFirst = (configPairs (reportConfig report) + 1) `div` 2
            actualFirst = length (filter ((== BaselineFirst) . pairOrder) (reportPairs report))
        if actualFirst /= expectedFirst then Left "pair order is not balanced" else Right ()
        result <- analyze (reportPairs report) (configConfidencePercent (reportConfig report)) (configMinimumFactor (reportConfig report))
        if result /= reportAnalysis report then Left "reported analysis disagrees with raw pairs" else Right report

-- A lexical key pass rejects duplicates before Aeson collapses them; Aeson
-- remains responsible for the complete JSON grammar and decoded value tree.
preflightKeys :: BS.ByteString -> Either String ()
preflightKeys input = go 0 [] 0
  where
    size = BS.length input
    byte = BS.index input
    skip i | i < size && byte i `elem` [9, 10, 13, 32] = skip (i + 1)
           | otherwise = i
    addValue count =
        let next = count + 1
         in if next > 65536 then Left "report exceeds JSON value limit" else Right next
    go :: Int -> [Maybe (Set.Set String)] -> Int -> Either String ()
    go i stack values
        | i >= size = Right ()
        | byte i == 34 = do
            end <- stringEnd (i + 1)
            let after = skip (end + 1)
                isKey = after < size && byte after == 58
            stack' <- if isKey then insertKey i end after stack else Right stack
            values' <- if isKey then Right values else addValue values
            go (end + 1) stack' values'
        | byte i == 123 = do
            if length stack >= 128 then Left "report exceeds nesting limit" else Right ()
            values' <- addValue values
            go (i + 1) (Just Set.empty : stack) values'
        | byte i == 91 = do
            if length stack >= 128 then Left "report exceeds nesting limit" else Right ()
            values' <- addValue values
            go (i + 1) (Nothing : stack) values'
        | byte i == 125 || byte i == 93 = go (i + 1) (drop 1 stack) values
        | byte i `elem` [9, 10, 13, 32, 44, 58] = go (i + 1) stack values
        | otherwise = do
            values' <- addValue values
            go (scalarEnd i) stack values'
    stringEnd i | i >= size = Left "unterminated string"
                 | byte i == 92 = stringEnd (i + 2)
                 | byte i == 34 = Right i
                 | otherwise = stringEnd (i + 1)
    insertKey start end colon stack = case stack of
        Just keys : rest -> do
            key <- either (const (Left "invalid object key")) Right (eitherDecodeStrict' (BS.take (end - start + 1) (BS.drop start input)) :: Either String String)
            if Set.member key keys then Left ("duplicate JSON key: " ++ key) else do
                integerSpelling key (skip (colon + 1))
                Right (Just (Set.insert key keys) : rest)
        _ -> Right stack
    integerSpelling key start
        | key `notElem` ["pairs", "confidence_percent", "seed", "target_ns", "max_batch", "batch_size", "baseline_ns", "candidate_ns"] = Right ()
        | start >= size || not (byte start == 45 || (byte start >= 48 && byte start <= 57)) = Right ()
        | otherwise =
            let end = numberEnd start
                spelling = BSC.unpack (BS.take (end - start) (BS.drop start input))
                digits = if take 1 spelling == "-" then drop 1 spelling else spelling
             in if not (null digits) && all (\ch -> ch >= '0' && ch <= '9') digits
                    then Right ()
                    else Left ("integer field must use an integer JSON token: " ++ key)
    numberEnd i | i >= size || byte i `elem` [9, 10, 13, 32, 44, 93, 125] = i
                | otherwise = numberEnd (i + 1)
    scalarEnd i | i >= size || byte i `elem` [9, 10, 13, 32, 44, 58, 91, 93, 123, 125] = i
                | otherwise = scalarEnd (i + 1)

checkTree :: Int -> Int -> Value -> Either String Int
checkTree depth nodes value
    | depth > 128 = Left "report exceeds nesting limit"
    | nodes > 65536 = Left "report exceeds JSON value limit"
    | otherwise = case value of
        Array items -> foldM (\count item -> checkTree (depth + 1) (count + 1) item) nodes items
        Object fields -> foldM (\count item -> checkTree (depth + 1) (count + 1) item) nodes (KM.elems fields)
        _ -> Right nodes

decodeReport :: Value -> Either String Report
decodeReport value = do
    fields <- asObject "report" value
    rejectUnknown "report" ["config", "batch_size", "pairs", "analysis", "workload", "invocation", "revision", "runtime", "os", "arch", "profile"] fields
    config <- required "config" fields >>= decodeConfig
    batchSize <- required "batch_size" fields >>= readInt64 "batch_size"
    rawPairs <- required "pairs" fields >>= asArray "pairs"
    if length rawPairs /= configPairs config then Left "pair count does not match config" else Right ()
    pairs <- mapM decodePair rawPairs
    if batchSize <= 0 || batchSize > configMaxBatch config then Left "batch_size outside configured bounds" else Right ()
    analysis <- required "analysis" fields >>= decodeAnalysis
    metadata <- decodeMetadata fields
    pure (Report config batchSize pairs analysis metadata)

decodeConfig :: Value -> Either String Config
decodeConfig value = do
    fields <- asObject "config" value
    rejectUnknown "config" ["pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch"] fields
    pairs <- required "pairs" fields >>= nonnegativeInt "pairs"
    confidence <- required "confidence_percent" fields >>= nonnegativeInt "confidence_percent"
    factor <- required "minimum_factor" fields >>= finiteDouble "minimum_factor"
    seed <- required "seed" fields >>= readInt64 "seed"
    target <- required "target_ns" fields >>= readInt64 "target_ns"
    maxBatch <- required "max_batch" fields >>= readInt64 "max_batch"
    let config = Config pairs confidence factor seed target maxBatch
    validateConfig config >> pure config

decodePair :: Value -> Either String Pair
decodePair value = do
    fields <- asObject "pair" value
    rejectUnknown "pair" ["baseline_ns", "candidate_ns", "order"] fields
    base <- required "baseline_ns" fields >>= readInt64 "baseline_ns"
    candidate <- required "candidate_ns" fields >>= readInt64 "candidate_ns"
    order <- required "order" fields >>= readText "order"
    parsed <- case order of "baseline-first" -> Right BaselineFirst; "candidate-first" -> Right CandidateFirst; _ -> Left "invalid pair order"
    if base <= 0 || candidate <= 0 then Left "pair durations must be positive" else Right (Pair base candidate parsed)

decodeAnalysis :: Value -> Either String Analysis
decodeAnalysis value = do
    fields <- asObject "analysis" value
    rejectUnknown "analysis" ["ratio", "interval", "verdict"] fields
    ratio <- required "ratio" fields >>= finiteDouble "ratio"
    rawInterval <- required "interval" fields
    interval <- case rawInterval of
        Null -> Right Nothing
        Array xs | V.length xs == 2 -> do
            lo <- finiteDouble "interval low" (xs V.! 0)
            hi <- finiteDouble "interval high" (xs V.! 1)
            if lo <= hi then Right (Just (Interval lo hi)) else Left "interval endpoints are reversed"
        _ -> Left "interval must be null or contain two values"
    verdict <- required "verdict" fields >>= readText "verdict"
    if verdict `elem` ["faster", "slower", "equivalent", "inconclusive"] then Right (Analysis ratio interval verdict) else Left "invalid verdict"

decodeMetadata :: KM.KeyMap Value -> Either String (Maybe Metadata)
decodeMetadata fields = do
    let names = ["workload", "invocation", "revision", "runtime", "os", "arch"]
        present = filter (`KM.member` fields) names
        hasProfile = KM.member "profile" fields
    if null present
        then if hasProfile then Left "profile requires standard metadata" else Right Nothing
        else if length present /= length names then Left "standard metadata must be complete" else do
            workload <- required "workload" fields >>= validWorkloadText "workload"
            rawInvocation <- required "invocation" fields >>= asArray "invocation"
            invocation <- mapM (textWithoutNul "invocation entry") rawInvocation
            revision <- required "revision" fields >>= nonemptyText "revision"
            runtime <- required "runtime" fields >>= nonemptyText "runtime"
            os <- required "os" fields >>= nonemptyText "os"
            arch <- required "arch" fields >>= nonemptyText "arch"
            profile <- if hasProfile then Just <$> (required "profile" fields >>= nonemptyText "profile") else Right Nothing
            pure (Just (Metadata workload invocation revision runtime os arch profile))

asObject :: String -> Value -> Either String (KM.KeyMap Value)
asObject _ (Object fields) = Right fields
asObject name _ = Left (name ++ " must be an object")
asArray :: String -> Value -> Either String [Value]
asArray _ (Array values) = Right (V.toList values)
asArray name _ = Left (name ++ " must be an array")
required :: Key.Key -> KM.KeyMap Value -> Either String Value
required name fields = maybe (Left ("missing field: " ++ Key.toString name)) Right (KM.lookup name fields)
rejectUnknown :: String -> [Key.Key] -> KM.KeyMap Value -> Either String ()
rejectUnknown name allowed fields = case filter (`notElem` allowed) (KM.keys fields) of
    [] -> Right ()
    key : _ -> Left ("unknown " ++ name ++ " field: " ++ Key.toString key)
readText :: String -> Value -> Either String String
readText _ (String value) = Right (T.unpack value)
readText name _ = Left (name ++ " must be a string")
validWorkloadText :: String -> Value -> Either String String
validWorkloadText name value = do
    result <- readText name value
    let asciiSpace c = c == ' ' || (c >= '\t' && c <= '\r')
    if null result || '\0' `elem` result || all asciiSpace result
        then Left (name ++ " must be nonblank and contain no NUL")
        else Right result
nonemptyText :: String -> Value -> Either String String
nonemptyText name value = do
    result <- textWithoutNul name value
    if null result then Left (name ++ " must not be empty") else Right result

textWithoutNul :: String -> Value -> Either String String
textWithoutNul name value = do
    result <- readText name value
    if '\0' `elem` result then Left (name ++ " must not contain NUL") else Right result
nonnegativeInt :: String -> Value -> Either String Int
nonnegativeInt name value = do
    result <- readInt64 name value
    if result < 0 || toInteger result > toInteger (maxBound :: Int) then Left (name ++ " out of range") else Right (fromIntegral result)
readInt64 :: String -> Value -> Either String Int64
readInt64 name (Number value) = maybe (Left (name ++ " must be an exact signed 64-bit integer")) Right (toBoundedInteger value)
readInt64 name _ = Left (name ++ " must be an integer")
finiteDouble :: String -> Value -> Either String Double
finiteDouble name (Number value) = let result = toRealFloat value in if isFinite result then Right result else Left (name ++ " must be finite")
finiteDouble name _ = Left (name ++ " must be a number")
