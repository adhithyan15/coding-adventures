{-# LANGUAGE BangPatterns #-}
module CodingAdventures.ImageCodecPng.Decoder
    ( decodePng
    , decodePngBounded
    , validateMaxPixels
    ) where

import Data.Bits ((.&.), shiftR)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8)
import PixelContainer (PixelContainer(..))
import qualified Zip

import CodingAdventures.ImageCodecPng.Adler32 (adler32)
import CodingAdventures.ImageCodecPng.Chunk
    ( calculateCrc
    , isCriticalChunk
    , readWord16BE
    , readWord32BE
    , validChunkType
    )
import CodingAdventures.ImageCodecPng.Constants
    ( bitDepth8
    , channelsGreyscale
    , channelsGreyscaleAlpha
    , channelsTruecolour
    , channelsTruecolourAlpha
    , chunkCrcFieldSize
    , chunkLengthFieldSize
    , chunkOverhead
    , chunkTypeFieldSize
    , colourTypeGreyscale
    , colourTypeGreyscaleAlpha
    , colourTypeIndexed
    , colourTypeTruecolour
    , colourTypeTruecolourAlpha
    , compressionDeflate
    , filterAdaptive
    , filterMaxValid
    , ihdrBitDepthOffset
    , ihdrColourTypeOffset
    , ihdrCompressionOffset
    , ihdrExpectedLength
    , ihdrFilterOffset
    , ihdrHeightOffset
    , ihdrInterlaceOffset
    , ihdrWidthOffset
    , interlaceNone
    , maxUint8Sample
    , opaqueAlpha
    , plteEntrySize
    , plteMaxBytes
    , plteMinBytes
    , pngDefaultMaxPixels
    , pngMaxDimension
    , pngSignature
    , pngSignatureLength
    , transparentAlpha
    , trnsBlueOffset
    , trnsGreenOffset
    , trnsGreyscaleBytes
    , trnsRedOffset
    , trnsTruecolourBytes
    , zlibCmfMaxWindowBits
    , zlibCmfMethodDeflate
    , zlibCmfMethodMask
    , zlibCmfWindowBitsShift
    , zlibFlgPresetDictMask
    , zlibHeaderBytes
    , zlibHeaderMultiplier
    , zlibHeaderMultiple
    , zlibMinHeaderLength
    , zlibTrailerBytes
    )
import CodingAdventures.ImageCodecPng.Filter (unfilterRow)
import CodingAdventures.ImageCodecPng.Messages
    ( PngError(..)
    , tagAcTl
    , tagFcTl
    , tagFdAt
    , tagIdat
    , tagIend
    , tagIhdr
    , tagPlte
    , tagTrns
    )

-- | Validate user-supplied maxPixels ceiling before inspecting file bytes.
validateMaxPixels :: Int -> Either PngError Int
validateMaxPixels mp
    | mp <= 0 || mp > pngDefaultMaxPixels = Left InvalidMaxPixels
    | otherwise = Right mp

-- | Internal parsing accumulator state.
data ParseState = ParseState
    { psWidth           :: !Int
    , psHeight          :: !Int
    , psColourType      :: !Word8
    , psSawIhdr         :: !Bool
    , psSawIend         :: !Bool
    , psSawPlte         :: !Bool
    , psSawTrns         :: !Bool
    , psInIdat          :: !Bool
    , psIdatEnded       :: !Bool
    , psTransparentGrey :: !(Maybe Word8)
    , psTransparentRgb  :: !(Maybe (Word8, Word8, Word8))
    , psIdatParts       :: ![ByteString]
    }

initialParseState :: ParseState
initialParseState = ParseState
    { psWidth           = 0
    , psHeight          = 0
    , psColourType      = 0
    , psSawIhdr         = False
    , psSawIend         = False
    , psSawPlte         = False
    , psSawTrns         = False
    , psInIdat          = False
    , psIdatEnded       = False
    , psTransparentGrey = Nothing
    , psTransparentRgb  = Nothing
    , psIdatParts       = []
    }

-- | Decode a PNG byte stream using the default 32-megapixel allocation ceiling.
decodePng :: ByteString -> Either PngError PixelContainer
decodePng = decodePngBounded pngDefaultMaxPixels

-- | Decode a PNG byte stream with a caller-enforced pixel ceiling.
decodePngBounded :: Int -> ByteString -> Either PngError PixelContainer
decodePngBounded maxPixLimit input = do
    maxPixels <- validateMaxPixels maxPixLimit

    if BS.length input < pngSignatureLength
        then Left FileTooShort
        else if BS.take pngSignatureLength input /= pngSignature
            then Left InvalidSignature
            else do
                finalState <- parseChunks maxPixels initialParseState pngSignatureLength
                reconstructImage finalState
  where
    inputLen = BS.length input

    parseChunks :: Int -> ParseState -> Int -> Either PngError ParseState
    parseChunks !maxPix !st !pos
        | pos == inputLen = Right st
        | otherwise =
            let !remLen = inputLen - pos
            in if remLen < chunkLengthFieldSize + chunkTypeFieldSize
               then Left TruncatedChunk
               else do
                   let !len32 = readWord32BE input pos
                       !lenInt = fromIntegral len32 :: Int
                   if remLen < chunkOverhead || (fromIntegral len32 :: Integer) > fromIntegral (remLen - chunkOverhead)
                       then Left TruncatedChunk
                       else do
                           let !typeStart = pos + chunkLengthFieldSize
                               !typeBytes = BS.take chunkTypeFieldSize (BS.drop typeStart input)
                           if not (validChunkType typeBytes)
                               then Left InvalidChunkType
                               else do
                                   let !dataStart = typeStart + chunkTypeFieldSize
                                       !dataBytes = BS.take lenInt (BS.drop dataStart input)
                                       !crcPos = dataStart + lenInt
                                       !declaredCrc = readWord32BE input crcPos
                                       !actualCrc = calculateCrc typeBytes dataBytes
                                   if actualCrc /= declaredCrc
                                       then Left ChunkCrcMismatch
                                       else if not (psSawIhdr st) && typeBytes /= tagIhdr
                                           then Left ChunkBeforeIhdr
                                           else handleChunk maxPix st typeBytes lenInt dataBytes crcPos

    handleChunk :: Int -> ParseState -> ByteString -> Int -> ByteString -> Int -> Either PngError ParseState
    handleChunk !maxPix !st !ctype !lenInt !cdata !crcPos
        | ctype == tagIhdr = do
            if psSawIhdr st
                then Left DuplicateIhdr
                else if lenInt /= ihdrExpectedLength
                    then Left InvalidIhdrLength
                    else do
                        let !w32 = readWord32BE cdata ihdrWidthOffset
                            !h32 = readWord32BE cdata ihdrHeightOffset
                        if w32 == 0 || h32 == 0
                            then Left InvalidDimensions
                            else if w32 > fromIntegral pngMaxDimension || h32 > fromIntegral pngMaxDimension
                                then Left DimensionLimit
                                else do
                                    let !w = fromIntegral w32 :: Int
                                        !h = fromIntegral h32 :: Int
                                        !pixelCount = (fromIntegral w :: Integer) * fromIntegral h
                                    if pixelCount > fromIntegral maxPix
                                        then Left PixelLimit
                                        else do
                                            let !bitDepth = BS.index cdata ihdrBitDepthOffset
                                                !colType = BS.index cdata ihdrColourTypeOffset
                                                !comp = BS.index cdata ihdrCompressionOffset
                                                !flt = BS.index cdata ihdrFilterOffset
                                                !intl = BS.index cdata ihdrInterlaceOffset
                                            if comp /= compressionDeflate || flt /= filterAdaptive || intl /= interlaceNone
                                                then Left UnsupportedFeature
                                                else if bitDepth /= bitDepth8 || colType == colourTypeIndexed
                                                    then Left UnsupportedFeature
                                                    else if colType /= colourTypeGreyscale
                                                         && colType /= colourTypeTruecolour
                                                         && colType /= colourTypeGreyscaleAlpha
                                                         && colType /= colourTypeTruecolourAlpha
                                                        then Left UnsupportedFeature
                                                        else continueWith st
                                                            { psSawIhdr = True
                                                            , psWidth = w
                                                            , psHeight = h
                                                            , psColourType = colType
                                                            }
        | ctype == tagPlte = do
            if psSawPlte st || not (null (psIdatParts st)) || psSawTrns st
                then Left InvalidPlte
                else if psColourType st /= colourTypeTruecolour && psColourType st /= colourTypeTruecolourAlpha
                    then Left InvalidPlte
                    else if lenInt < plteMinBytes || lenInt > plteMaxBytes || lenInt `rem` plteEntrySize /= 0
                        then Left InvalidPlte
                        else continueWith st { psSawPlte = True }

        | ctype == tagTrns = do
            if psSawTrns st || not (null (psIdatParts st))
                then Left InvalidTrns
                else if psColourType st == colourTypeGreyscale
                    then if lenInt /= trnsGreyscaleBytes
                         then Left InvalidTrns
                         else do
                             let !sample = readWord16BE cdata 0
                             if sample > maxUint8Sample
                                 then Left InvalidTrns
                                 else continueWith st { psSawTrns = True, psTransparentGrey = Just (fromIntegral sample) }
                    else if psColourType st == colourTypeTruecolour
                        then if lenInt /= trnsTruecolourBytes
                             then Left InvalidTrns
                             else do
                                 let !r = readWord16BE cdata trnsRedOffset
                                     !g = readWord16BE cdata trnsGreenOffset
                                     !b = readWord16BE cdata trnsBlueOffset
                                 if r > maxUint8Sample || g > maxUint8Sample || b > maxUint8Sample
                                     then Left InvalidTrns
                                     else continueWith st { psSawTrns = True, psTransparentRgb = Just (fromIntegral r, fromIntegral g, fromIntegral b) }
                        else Left InvalidTrns

        | ctype == tagIdat = do
            if psIdatEnded st
                then Left NonconsecutiveIdat
                else continueWith st { psInIdat = True, psIdatParts = cdata : psIdatParts st }

        | ctype == tagIend = do
            if lenInt /= 0
                then Left InvalidIend
                else if crcPos + chunkCrcFieldSize /= inputLen
                    then Left TrailingData
                    else Right (st { psSawIend = True })

        | ctype == tagAcTl || ctype == tagFcTl || ctype == tagFdAt =
            Left UnsupportedFeature

        | isCriticalChunk ctype =
            Left UnknownCriticalChunk

        | otherwise =
            continueWith st
      where
        continueWith !nextSt =
            let !updatedSt = if ctype /= tagIdat && psInIdat nextSt
                             then nextSt { psInIdat = False, psIdatEnded = True }
                             else nextSt
                !nextPos = crcPos + chunkCrcFieldSize
            in parseChunks maxPix updatedSt nextPos

    reconstructImage :: ParseState -> Either PngError PixelContainer
    reconstructImage !st = do
        if not (psSawIhdr st && psSawIend st && not (null (psIdatParts st)))
            then Left MissingRequiredChunk
            else do
                let !zlibData = BS.concat (reverse (psIdatParts st))
                if BS.length zlibData < zlibMinHeaderLength
                    then Left InvalidZlibHeader
                    else do
                        let !cmf = BS.index zlibData 0
                            !flg = BS.index zlibData 1
                            !hdrWord = (fromIntegral cmf :: Int) * zlibHeaderMultiplier + fromIntegral flg
                        if (cmf .&. zlibCmfMethodMask) /= zlibCmfMethodDeflate
                           || (cmf `shiftR` zlibCmfWindowBitsShift) > zlibCmfMaxWindowBits
                           || (hdrWord `rem` zlibHeaderMultiple) /= 0
                            then Left InvalidZlibHeader
                            else if (flg .&. zlibFlgPresetDictMask) /= 0
                                then Left PresetDictionary
                                else decompressAndFilter st zlibData

    decompressAndFilter :: ParseState -> ByteString -> Either PngError PixelContainer
    decompressAndFilter !st !zlibData = do
        let !colType = psColourType st
            !channels
                | colType == colourTypeGreyscale      = channelsGreyscale
                | colType == colourTypeTruecolour     = channelsTruecolour
                | colType == colourTypeGreyscaleAlpha = channelsGreyscaleAlpha
                | otherwise                           = channelsTruecolourAlpha
            !width = psWidth st
            !height = psHeight st
            !stride = width * channels
            !expectedUncomp = height * (stride + 1)
            !deflatePayload = BS.take (BS.length zlibData - zlibMinHeaderLength) (BS.drop zlibHeaderBytes zlibData)

        rawInflateRes <- case Zip.rawInflateCounted deflatePayload expectedUncomp of
            Left Zip.OutputLimitExceeded -> Left InflatedLengthMismatch
            Left _ -> Left InflateFailed
            Right res -> Right res

        let !decompressed = Zip.rawInflateOutput rawInflateRes
        if BS.length decompressed /= expectedUncomp
            then Left InflatedLengthMismatch
            else if Zip.rawInflateBytesConsumed rawInflateRes /= BS.length deflatePayload
                then Left IdatCavity
                else do
                    let !declaredAdler = readWord32BE zlibData (BS.length zlibData - zlibTrailerBytes)
                        !actualAdler = adler32 decompressed
                    if actualAdler /= declaredAdler
                        then Left AdlerMismatch
                        else do
                            verifyFilterBytes height stride decompressed
                            unfilterAndConvert st channels stride decompressed

    verifyFilterBytes :: Int -> Int -> ByteString -> Either PngError ()
    verifyFilterBytes !height !stride !decompressed =
        let checkRow !r
                | r >= height = Right ()
                | otherwise =
                    let !ft = BS.index decompressed (r * (stride + 1))
                    in if ft > filterMaxValid
                       then Left InvalidFilter
                       else checkRow (r + 1)
        in checkRow 0

    unfilterAndConvert :: ParseState -> Int -> Int -> ByteString -> Either PngError PixelContainer
    unfilterAndConvert !st !channels !stride !decompressed = do
        let !width = psWidth st
            !height = psHeight st
            !greyTrns = psTransparentGrey st
            !rgbTrns = psTransparentRgb st

            convertScanlines :: Int -> ByteString -> [ByteString] -> Either PngError [ByteString]
            convertScanlines !rowIdx !priorRow !acc
                | rowIdx >= height = Right (reverse acc)
                | otherwise = do
                    let !sourceOffset = rowIdx * (stride + 1)
                        !filterType = BS.index decompressed sourceOffset
                        !rawRow = BS.take stride (BS.drop (sourceOffset + 1) decompressed)
                    unfiltered <- unfilterRow filterType channels rawRow priorRow
                    let !rgbaRow
                            | channels == channelsGreyscale      = convertGreyscale width greyTrns unfiltered
                            | channels == channelsGreyscaleAlpha = convertGreyscaleAlpha width unfiltered
                            | channels == channelsTruecolour     = convertTruecolour width rgbTrns unfiltered
                            | otherwise                          = unfiltered
                    convertScanlines (rowIdx + 1) unfiltered (rgbaRow : acc)

            initialPrior = BS.replicate stride 0

        rows <- convertScanlines 0 initialPrior []
        let !allPixels = BS.concat rows
        return (PixelContainer width height allPixels)

    convertGreyscale :: Int -> Maybe Word8 -> ByteString -> ByteString
    convertGreyscale !w !trnsVal !row =
        let generatePixel !col =
                let !v = BS.index row col
                    !a = if trnsVal == Just v then transparentAlpha else opaqueAlpha
                in [v, v, v, a]
        in BS.pack (concatMap generatePixel [0 .. w - 1])

    convertGreyscaleAlpha :: Int -> ByteString -> ByteString
    convertGreyscaleAlpha !w !row =
        let generatePixel !col =
                let !baseIdx = col * 2
                    !v = BS.index row baseIdx
                    !a = BS.index row (baseIdx + 1)
                in [v, v, v, a]
        in BS.pack (concatMap generatePixel [0 .. w - 1])

    convertTruecolour :: Int -> Maybe (Word8, Word8, Word8) -> ByteString -> ByteString
    convertTruecolour !w !trnsVal !row =
        let generatePixel !col =
                let !baseIdx = col * 3
                    !r = BS.index row baseIdx
                    !g = BS.index row (baseIdx + 1)
                    !b = BS.index row (baseIdx + 2)
                    !a = if trnsVal == Just (r, g, b) then transparentAlpha else opaqueAlpha
                in [r, g, b, a]
        in BS.pack (concatMap generatePixel [0 .. w - 1])
