{-# LANGUAGE BangPatterns, ScopedTypeVariables #-}
module CodingAdventures.ImageCodecPng.Filter
    ( paethPredictor
    , predictSample
    , unfilterRow
    , filterRow
    , chooseBestFilter
    , scoreFilteredRow
    ) where

import Control.Monad.ST (ST, runST)
import Data.Array.MArray (getElems, newArray, readArray, writeArray)
import Data.Array.ST (STUArray)
import Data.Bits ((.&.))
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8)

import CodingAdventures.ImageCodecPng.Constants
    ( averageFilterDivisor
    , byteMaskInt
    , byteModulus
    , filterAverage
    , filterMaxValid
    , filterNone
    , filterPaeth
    , filterSub
    , filterUp
    , signedByteThreshold
    )
import CodingAdventures.ImageCodecPng.Messages (PngError(..))

-- | RFC 2083 Paeth predictor with normative tie-breaking order (a <= b <= c).
paethPredictor :: Int -> Int -> Int -> Int
paethPredictor !a !b !c =
    let !p = a + b - c
        !pa = abs (p - a)
        !pb = abs (p - b)
        !pc = abs (p - c)
    in if pa <= pb && pa <= pc
       then a
       else if pb <= pc
            then b
            else c

-- | Compute predicted sample value for a given filter type from reconstructed/raw neighbors.
predictSample :: Word8 -> Int -> Int -> Int -> Int
predictSample !ft !left !above !aboveLeft
    | ft == filterNone    = 0
    | ft == filterSub     = left
    | ft == filterUp      = above
    | ft == filterAverage = (left + above) `div` averageFilterDivisor
    | ft == filterPaeth   = paethPredictor left above aboveLeft
    | otherwise           = 0

-- | Reconstruct a single scanline using the specified filter type and prior scanline.
unfilterRow :: Word8 -> Int -> ByteString -> ByteString -> Either PngError ByteString
unfilterRow !ft !bpp !rawRow !priorRow
    | ft > filterMaxValid = Left InvalidFilter
    | otherwise = Right (runST doUnfilter)
  where
    doUnfilter :: forall s. ST s ByteString
    doUnfilter = do
        let !stride = BS.length rawRow
            !priorLen = BS.length priorRow
        arr <- newArray (0, stride - 1) 0 :: ST s (STUArray s Int Word8)
        let loop :: Int -> ST s ()
            loop !i
                | i >= stride = return ()
                | otherwise = do
                    let !rawByte = BS.index rawRow i
                    left <- if i >= bpp && bpp > 0
                            then fromIntegral <$> readArray arr (i - bpp)
                            else return 0
                    let !above = if i < priorLen then fromIntegral (BS.index priorRow i) else 0
                        !aboveLeft = if i >= bpp && bpp > 0 && (i - bpp) < priorLen
                                     then fromIntegral (BS.index priorRow (i - bpp))
                                     else 0
                        !predVal = predictSample ft left above aboveLeft
                        !reconByte = fromIntegral ((fromIntegral rawByte + predVal) .&. byteMaskInt)
                    writeArray arr i reconByte
                    loop (i + 1)
        loop 0
        elemsList <- getElems arr
        return (BS.pack elemsList)

-- | Apply a specific filter to a scanline for encoding.
filterRow :: Word8 -> Int -> ByteString -> ByteString -> ByteString
filterRow !ft !bpp !rawRow !priorRow =
    let !stride = BS.length rawRow
        !priorLen = BS.length priorRow
        generateByte !i =
            let !rawByte = BS.index rawRow i
                !left = if i >= bpp && bpp > 0
                        then fromIntegral (BS.index rawRow (i - bpp))
                        else 0
                !above = if i < priorLen then fromIntegral (BS.index priorRow i) else 0
                !aboveLeft = if i >= bpp && bpp > 0 && (i - bpp) < priorLen
                             then fromIntegral (BS.index priorRow (i - bpp))
                             else 0
                !predVal = predictSample ft left above aboveLeft
                !diff = (fromIntegral rawByte - predVal) .&. byteMaskInt
            in fromIntegral diff
    in BS.pack [generateByte i | i <- [0 .. stride - 1]]

-- | Score a filtered scanline using sum of absolute differences.
scoreFilteredRow :: ByteString -> Int
scoreFilteredRow bs = BS.foldl' addScore 0 bs
  where
    addScore !acc !byte =
        let !v = fromIntegral byte :: Int
            !score = if v < signedByteThreshold then v else byteModulus - v
        in acc + score

-- | Choose the best filter type (0..4) that minimizes the heuristic sum of absolute differences.
-- In case of tie, the strictly lowest filter ID wins.
chooseBestFilter :: Int -> ByteString -> ByteString -> (Word8, ByteString)
chooseBestFilter !bpp !rawRow !priorRow =
    let initialFilter = filterNone
        initialData = filterRow initialFilter bpp rawRow priorRow
        initialScore = scoreFilteredRow initialData
        evaluate (bestFt, bestData, bestScore) candFt =
            let candData = filterRow candFt bpp rawRow priorRow
                candScore = scoreFilteredRow candData
            in if candScore < bestScore
               then (candFt, candData, candScore)
               else (bestFt, bestData, bestScore)
        candidateFilters = [filterSub, filterUp, filterAverage, filterPaeth]
        (winnerFt, winnerData, _) = foldl evaluate (initialFilter, initialData, initialScore) candidateFilters
    in (winnerFt, winnerData)
