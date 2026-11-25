module Main (main) where

import Control.Monad (forM_, unless)
import Data.Bits (popCount, shiftL)
import Data.List (sort)
import RankStatistics

assert :: String -> Bool -> IO ()
assert label condition = unless condition (fail label)

main :: IO ()
main = do
    let vectors :: [(Int, Int, Int, Int)]
        vectors =
            [ (64, 95, 23, 40), (65, 99, 21, 43), (127, 95, 51, 75)
            , (128, 95, 52, 75), (256, 99, 106, 149), (1000, 95, 468, 531)
            , (10000, 50, 4965, 5034), (10000, 95, 4901, 5098)
            , (10000, 99, 4870, 5129)
            ]
    forM_ vectors $ \(n, confidence, low, high) -> do
        result <- either fail pure (medianInterval (map fromIntegral [0 .. n - 1]) confidence)
        assert ("wide cutoff endpoint vector " ++ show (n, confidence))
            (result == Just (Interval (fromIntegral low) (fromIntegral high)))
    forM_ [1 .. 12] $ \n -> forM_ [1 .. 99] $ \confidence -> do
        expected <- either fail pure (boundedOracle n confidence)
        actual <- either fail pure (exactMedianRank n confidence)
        assert ("bounded sign oracle " ++ show (n, confidence)) (expected == actual)
    let d = encodeFloat 1 (fst (floatRange (0 :: Double)) - floatDigits (0 :: Double))
        nearMin = encodeFloat (floatRadix (0 :: Double) ^ (floatDigits (0 :: Double) - 1) - 1)
            (fst (floatRange (0 :: Double)) - floatDigits (0 :: Double))
    assert "positive subnormal midpoint" (median [d, 2 * d] == Right (2 * d))
    assert "negative subnormal midpoint" (median [-2 * d, -d] == Right (-2 * d))
    assert "normal boundary midpoint" (median [nearMin, encodeFloat 1 (fst (floatRange (0 :: Double)) - 1)] == Right (encodeFloat 1 (fst (floatRange (0 :: Double)) - 1)))
    assert "opposite extreme midpoint" (median [-encodeFloat (floatRadix (0 :: Double) ^ floatDigits (0 :: Double) - 1) (snd (floatRange (0 :: Double)) - floatDigits (0 :: Double)), encodeFloat (floatRadix (0 :: Double) ^ floatDigits (0 :: Double) - 1) (snd (floatRange (0 :: Double)) - floatDigits (0 :: Double))] == Right 0)
    let maximumFinite = encodeFloat (floatRadix (0 :: Double) ^ floatDigits (0 :: Double) - 1) (snd (floatRange (0 :: Double)) - floatDigits (0 :: Double))
    assert "maximum finite midpoint" (median [maximumFinite, maximumFinite] == Right maximumFinite)
    forM_ [1 .. 500] $ \trial -> do
        let sampleSize = 1 + trial `mod` 97
            sample = [ fromIntegral (((i * 1103515245 + trial * 12345) `mod` 2001) - 1000) | i <- [0 .. sampleSize - 1] ]
            ordered = sort sample
        center <- either fail pure (median sample)
        interval <- either fail pure (medianInterval sample 90)
        assert "generated median within data range" (center >= head ordered && center <= last ordered)
        assert "generated interval contains sample median" (maybe True (\ci -> intervalLow ci <= center && center <= intervalHigh ci) interval)
    assert "empty rejected" (case median [] of Left _ -> True; _ -> False)
    assert "nonfinite rejected" (case median [0 / 0] of Left _ -> True; _ -> False)
    putStrLn "rank-statistics: all deterministic and exact-cutoff checks passed"

boundedOracle :: Int -> Int -> Either String Int
boundedOracle n confidence = do
    let cutoff = toInteger (100 - confidence) * (2 ^ n)
        limit = (1 :: Int) `shiftL` n
        tailFor k = toInteger (length [mask | mask <- [0 .. limit - 1], popCount mask < k])
        valid k = 200 * tailFor k <= cutoff
    pure (length (takeWhile valid [1 .. n `div` 2]))
