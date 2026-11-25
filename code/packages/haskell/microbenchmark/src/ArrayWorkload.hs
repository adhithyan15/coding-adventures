-- | Fixed portable array workload used by the native demonstration command.
module ArrayWorkload
    ( workloadId, makeArray, expectedChecksum, indexedBatch, iterationBatch
    , monotonicClock
    ) where

import Control.Exception (throwIO)
import Data.Array (Array, (!), bounds, elems, listArray)
import Data.Int (Int64)
import Data.List (foldl')
import Data.Word (Word64)
import GHC.Clock (getMonotonicTimeNSec)

workloadId :: String
workloadId = "indexed-array-sum-4096"

makeArray :: Array Int Int
makeArray = listArray (0, 4095) [ (17 * index + 3) `mod` 251 | index <- [0 .. 4095] ]

expectedChecksum :: Int64
expectedChecksum = fromIntegral (sum (elems makeArray))

checkedSum :: Integer -> Either String Int64
checkedSum total
    | total < toInteger (minBound :: Int64) || total > toInteger (maxBound :: Int64) = Left "checksum arithmetic overflow"
    | otherwise = Right (fromInteger total)

indexedBatch :: Array Int Int -> Int64 -> Either String Int64
indexedBatch values operations
    | operations < 0 = Left "operations must be nonnegative"
    | otherwise = checkedSum (go 0 0)
  where
    (low, high) = bounds values
    go operation total
        | operation >= operations = total
        | otherwise = go (operation + 1) (foldl' (\acc index -> acc + toInteger (values ! index)) total [low .. high])

iterationBatch :: Array Int Int -> Int64 -> Either String Int64
iterationBatch values operations
    | operations < 0 = Left "operations must be nonnegative"
    | otherwise = checkedSum (go 0 0)
  where
    go operation total
        | operation >= operations = total
        | otherwise = go (operation + 1) (foldl' (\acc value -> acc + toInteger value) total (elems values))

-- GHC.Clock is a monotonic wall-clock source. Values are elapsed since this
-- clock was created, so the public signed-64-bit counter cannot overflow from
-- a long system uptime.
monotonicClock :: IO (IO Int64)
monotonicClock = do
    start <- getMonotonicTimeNSec
    pure $ do
        now <- getMonotonicTimeNSec
        if now < start then throwIO (userError "monotonic clock regressed") else do
            let elapsed = now - start :: Word64
            if elapsed > fromIntegral (maxBound :: Int64)
                then throwIO (userError "monotonic duration exceeded signed 64-bit range")
                else pure (fromIntegral elapsed)
