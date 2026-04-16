module Hyperloglog
    ( description
    , HyperLogLog(..)
    , defaultPrecision
    , new
    , add
    , addMany
    , count
    , merge
    , mergeMany
    ) where

import qualified Data.ByteString as BS
import Data.Bits (FiniteBits(countLeadingZeros), (.&.), shiftR)
import Data.List (foldl')
import HashFunctions (hashBytes64)

description :: String
description = "Haskell HyperLogLog approximate cardinality package"

defaultPrecision :: Int
defaultPrecision = 10

data HyperLogLog = HyperLogLog
    { hllPrecision :: Int
    , hllRegisters :: [Int]
    }
    deriving (Eq, Show)

new :: HyperLogLog
new = HyperLogLog defaultPrecision (replicate (2 ^ defaultPrecision) 0)

add :: BS.ByteString -> HyperLogLog -> (HyperLogLog, Bool)
add bytesValue hll =
    let precisionValue = hllPrecision hll
        registersValue = hllRegisters hll
        hashedValue = hashBytes64 bytesValue
        bucketIndex = fromIntegral (hashedValue .&. fromIntegral ((2 ^ precisionValue) - 1))
        shiftedValue = hashedValue `shiftR` precisionValue
        rankValue =
            min
                (64 - precisionValue + 1)
                (countLeadingZeros shiftedValue + 1)
        oldRegister = registersValue !! bucketIndex
        newRegister = max oldRegister rankValue
        updatedRegisters = replaceAt bucketIndex newRegister registersValue
     in (HyperLogLog precisionValue updatedRegisters, newRegister /= oldRegister)

addMany :: [BS.ByteString] -> HyperLogLog -> (HyperLogLog, Bool)
addMany bytesValues initialHll =
    foldl'
        step
        (initialHll, False)
        bytesValues
  where
    step (currentHll, changed) value =
        let (nextHll, valueChanged) = add value currentHll
         in (nextHll, changed || valueChanged)

count :: HyperLogLog -> Integer
count hll =
    max 0 (round correctedEstimate)
  where
    precisionValue = hllPrecision hll
    registersValue = hllRegisters hll
    registerCountValue = fromIntegral (2 ^ precisionValue) :: Double
    alphaValue =
        case round registerCountValue :: Int of
            16 -> 0.673
            32 -> 0.697
            64 -> 0.709
            _ -> 0.7213 / (1 + 1.079 / registerCountValue)
    harmonicDenominator =
        sum [2 ** negate (fromIntegral registerValue :: Double) | registerValue <- registersValue]
    rawEstimate = alphaValue * registerCountValue * registerCountValue / harmonicDenominator
    zeroRegisters = length (filter (== 0) registersValue)
    correctedEstimate
        | rawEstimate <= 2.5 * registerCountValue && zeroRegisters > 0 =
            registerCountValue
                * log (registerCountValue / fromIntegral zeroRegisters)
        | otherwise = rawEstimate

merge :: HyperLogLog -> HyperLogLog -> HyperLogLog
merge leftHll rightHll
    | hllPrecision leftHll /= hllPrecision rightHll =
        error "HyperLogLog precision mismatch"
    | otherwise =
        HyperLogLog
            { hllPrecision = hllPrecision leftHll
            , hllRegisters =
                zipWith max (hllRegisters leftHll) (hllRegisters rightHll)
            }

mergeMany :: [HyperLogLog] -> HyperLogLog
mergeMany = foldl' merge new

replaceAt :: Int -> value -> [value] -> [value]
replaceAt indexValue replacement valuesList =
    take indexValue valuesList
        ++ [replacement]
        ++ drop (indexValue + 1) valuesList
