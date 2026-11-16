{-# LANGUAGE BangPatterns #-}
module CodingAdventures.ImageCodecPng.Encoder
    ( encodePng
    , validateEncodeContainer
    ) where

import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import PixelContainer (PixelContainer(..))
import qualified Zip

import CodingAdventures.ImageCodecPng.Adler32 (adler32)
import CodingAdventures.ImageCodecPng.Chunk (makeChunk, word32ToBytesBE)
import CodingAdventures.ImageCodecPng.Constants
    ( bitDepth8
    , bytesPerPixelRgba
    , colourTypeTruecolourAlpha
    , compressionDeflate
    , filterAdaptive
    , interlaceNone
    , pngDefaultMaxPixels
    , pngMaxDimension
    , pngSignature
    , zlibHeaderCmfDefault
    , zlibHeaderFlgDefault
    )
import CodingAdventures.ImageCodecPng.Filter (chooseBestFilter)
import CodingAdventures.ImageCodecPng.Messages
    ( PngError(..)
    , tagIdat
    , tagIend
    , tagIhdr
    )

-- | Validate PixelContainer dimensions and buffer length prior to encoding.
validateEncodeContainer :: PixelContainer -> Either PngError (Int, Int)
validateEncodeContainer pc =
    let !w = pcWidth pc
        !h = pcHeight pc
    in if w <= 0 || h <= 0 || w > pngMaxDimension || h > pngMaxDimension
       then Left InvalidImageDimensions
       else
           let !pixelCount = (fromIntegral w :: Integer) * fromIntegral h
           in if pixelCount > fromIntegral pngDefaultMaxPixels
              then Left InvalidImageDimensions
              else
                  let !expectedLen = (fromIntegral w :: Integer) * fromIntegral h * fromIntegral bytesPerPixelRgba
                  in if fromIntegral (BS.length (pcPixels pc)) /= expectedLen
                     then Left InvalidPixelDataLength
                     else Right (w, h)

-- | Encode a PixelContainer into deterministic RFC 2083 RGBA8 PNG bytes.
encodePng :: PixelContainer -> Either PngError ByteString
encodePng pc = do
    (width, height) <- validateEncodeContainer pc
    let !stride = width * bytesPerPixelRgba
        !pixels = pcPixels pc

        filterScanlines :: Int -> ByteString -> [ByteString] -> [ByteString]
        filterScanlines !rowIdx !priorRow !acc
            | rowIdx >= height = reverse acc
            | otherwise =
                let !start = rowIdx * stride
                    !rawRow = BS.take stride (BS.drop start pixels)
                    (!winnerFilter, !winnerData) = chooseBestFilter bytesPerPixelRgba rawRow priorRow
                    !scanline = BS.singleton winnerFilter <> winnerData
                in filterScanlines (rowIdx + 1) rawRow (scanline : acc)

        initialPrior = BS.replicate stride 0
        filteredRows = filterScanlines 0 initialPrior []
        filteredAll = BS.concat filteredRows

        !adlerChecksum = adler32 filteredAll
        !deflated = Zip.rawDeflate filteredAll

        !zlibData = BS.pack [zlibHeaderCmfDefault, zlibHeaderFlgDefault]
                 <> deflated
                 <> word32ToBytesBE adlerChecksum

        !ihdrData = word32ToBytesBE (fromIntegral width)
                 <> word32ToBytesBE (fromIntegral height)
                 <> BS.pack [ bitDepth8
                            , colourTypeTruecolourAlpha
                            , compressionDeflate
                            , filterAdaptive
                            , interlaceNone
                            ]

        !ihdrChunk = makeChunk tagIhdr ihdrData
        !idatChunk = makeChunk tagIdat zlibData
        !iendChunk = makeChunk tagIend BS.empty

    return (pngSignature <> ihdrChunk <> idatChunk <> iendChunk)
