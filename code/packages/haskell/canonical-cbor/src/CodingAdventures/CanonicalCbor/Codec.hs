{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE OverloadedStrings #-}
module CodingAdventures.CanonicalCbor.Codec
  ( encodeChecked
  , encodeIntoChecked
  , decode
  , decodeMapSpans
  ) where

import Data.Bits ((.&.), (.|.), shiftL, shiftR)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.List as List
import Data.Text.Encoding as TE
import Data.Word (Word8, Word64)

import CodingAdventures.CanonicalCbor.Constants as Constants
import CodingAdventures.CanonicalCbor.Types

-- ============================================================================
-- Encoding State & Budget Tracking
-- ============================================================================

data EncodeState = EncodeState
  { encAcc   :: ![ByteString]
  , encSize  :: !Word64
  , encLimit :: !Word64
  }

initEncodeState :: Word64 -> EncodeState
initEncodeState limit = EncodeState [] 0 limit

initEncodeIntoState :: ByteString -> Word64 -> EncodeState
initEncodeIntoState prefix limit =
  EncodeState [prefix] (fromIntegral (BS.length prefix)) limit

pushBytes :: ByteString -> EncodeState -> Either CborError EncodeState
pushBytes chunk st
  | newSize > encLimit st = Left (mkCborError EncodeTooLarge)
  | otherwise = Right (st { encAcc = chunk : encAcc st, encSize = newSize })
  where
    newSize = encSize st + fromIntegral (BS.length chunk)

-- | Strict counter bounded by maximum allowed items to prevent infinite lazy list loops (CVE DoS defense).
boundedCount :: Word64 -> [a] -> Maybe Word64
boundedCount maxAllowed xs = go 0 xs
  where
    go !n [] = Just n
    go !n (_:rest)
      | n >= maxAllowed = Nothing
      | otherwise = go (n + 1) rest

-- ============================================================================
-- Public Encoding API
-- ============================================================================

encodeChecked :: CborValue -> Either CborError ByteString
encodeChecked val = do
  st <- encodeValue val Constants.rootDepth (initEncodeState Constants.maxEncodedBytes)
  Right (BS.concat (reverse (encAcc st)))

encodeIntoChecked :: CborValue -> ByteString -> Either CborError ByteString
encodeIntoChecked val prefix
  | fromIntegral (BS.length prefix) > Constants.maxEncodedBytes =
      Left (mkCborError EncodeTooLarge)
  | otherwise = do
      st <- encodeValue val Constants.rootDepth (initEncodeIntoState prefix Constants.maxEncodedBytes)
      Right (BS.concat (reverse (encAcc st)))

-- ============================================================================
-- Encoding Internals
-- ============================================================================

encodeValue :: CborValue -> Int -> EncodeState -> Either CborError EncodeState
encodeValue _ depth _
  | depth > Constants.maxNestingDepth = Left (mkCborError EncodeTooDeep)
encodeValue val depth st =
  case val of
    CborUnsigned u ->
      pushBytes (writeTypeAndArg Constants.majorUnsigned u) st

    CborNegative n ->
      pushBytes (writeTypeAndArg Constants.majorNegative n) st

    CborBytes b -> do
      st1 <- pushBytes (writeTypeAndArg Constants.majorBytes (fromIntegral (BS.length b))) st
      pushBytes b st1

    CborText t -> do
      let bs = TE.encodeUtf8 t
      st1 <- pushBytes (writeTypeAndArg Constants.majorText (fromIntegral (BS.length bs))) st
      pushBytes bs st1

    CborArray items -> do
      let remainingBudget = encLimit st - encSize st
          maxPossibleItems = remainingBudget `div` Constants.minBytesPerArrayItem
      case boundedCount maxPossibleItems items of
        Nothing -> Left (mkCborError EncodeTooLarge)
        Just count -> do
          st1 <- pushBytes (writeTypeAndArg Constants.majorArray count) st
          encodeArrayItems items (depth + 1) st1

    CborMap entries ->
      encodeMap entries depth st

    CborTag tag inner -> do
      st1 <- pushBytes (writeTypeAndArg Constants.majorTag tag) st
      encodeValue inner (depth + 1) st1

    CborBool False ->
      pushBytes (BS.singleton Constants.wireByteFalse) st

    CborBool True ->
      pushBytes (BS.singleton Constants.wireByteTrue) st

    CborNull ->
      pushBytes (BS.singleton Constants.wireByteNull) st

encodeArrayItems :: [CborValue] -> Int -> EncodeState -> Either CborError EncodeState
encodeArrayItems [] _ st = Right st
encodeArrayItems (x:xs) d st = do
  st' <- encodeValue x d st
  encodeArrayItems xs d st'

encodeMap :: [CborMapEntry] -> Int -> EncodeState -> Either CborError EncodeState
encodeMap entries depth st = do
  let remainingBudget = encLimit st - encSize st
      maxPossibleEntries = remainingBudget `div` Constants.minBytesPerMapEntry
  count <- case boundedCount maxPossibleEntries entries of
    Nothing -> Left (mkCborError EncodeTooLarge)
    Just n  -> Right n

  -- Fold over entries threading the remaining byte budget across key encodings
  -- charging each entry key wire length + minBytesPerValue so key + minimal 1-byte value must fit
  pairs <- foldKeyEncodings entries count remainingBudget (depth + 1) []

  -- Sort keys length-first per RFC 8949 §4.2.3:
  -- Shorter byte length precedes longer byte length; equal length sorted by lexicographical byte comparison.
  let sortedPairs = List.sortBy compareMapPairs pairs

  -- Duplicate key rejection
  checkDuplicates sortedPairs

  -- Emit map header and interleaved (key, value) pairs
  st1 <- pushBytes (writeTypeAndArg Constants.majorMap count) st
  encodeMapPairs sortedPairs (depth + 1) st1

foldKeyEncodings
  :: [CborMapEntry]
  -> Word64
  -> Word64
  -> Int
  -> [(ByteString, CborValue, CborValue)]
  -> Either CborError [(ByteString, CborValue, CborValue)]
foldKeyEncodings [] _ _ _ acc = Right (reverse acc)
foldKeyEncodings (CborMapEntry k v : rest) remainingCount budget keyDepth acc = do
  let reservedForValues = remainingCount * Constants.minBytesPerValue
  if budget <= reservedForValues
    then Left (mkCborError EncodeTooLarge)
    else do
      let availForKey = budget - reservedForValues
      kWire <- encodeCheckedAtDepth k keyDepth availForKey
      let kWireLen = fromIntegral (BS.length kWire) :: Word64
      if kWireLen > availForKey
        then Left (mkCborError EncodeTooLarge)
        else foldKeyEncodings
               rest
               (remainingCount - 1)
               (budget - kWireLen)
               keyDepth
               ((kWire, k, v) : acc)

encodeCheckedAtDepth :: CborValue -> Int -> Word64 -> Either CborError ByteString
encodeCheckedAtDepth val depth budgetLimit = do
  st <- encodeValue val depth (initEncodeState budgetLimit)
  Right (BS.concat (reverse (encAcc st)))

compareMapPairs :: (ByteString, CborValue, CborValue) -> (ByteString, CborValue, CborValue) -> Ordering
compareMapPairs (k1Wire, _, _) (k2Wire, _, _) =
  case compare (BS.length k1Wire) (BS.length k2Wire) of
    EQ  -> compare k1Wire k2Wire
    ord -> ord

checkDuplicates :: [(ByteString, CborValue, CborValue)] -> Either CborError ()
checkDuplicates [] = Right ()
checkDuplicates [_] = Right ()
checkDuplicates ((k1, _, _) : next@((k2, _, _) : _))
  | k1 == k2 = Left (mkCborError DuplicateMapKey)
  | otherwise = checkDuplicates next

encodeMapPairs :: [(ByteString, CborValue, CborValue)] -> Int -> EncodeState -> Either CborError EncodeState
encodeMapPairs [] _ st = Right st
encodeMapPairs ((kWire, _, v) : rest) d st = do
  st1 <- pushBytes kWire st
  st2 <- encodeValue v d st1
  encodeMapPairs rest d st2

writeTypeAndArg :: Word8 -> Word64 -> ByteString
writeTypeAndArg major arg
  | arg <= fromIntegral Constants.aiDirectMax =
      BS.singleton (lead .|. fromIntegral arg)
  | arg <= Constants.maxUint8 =
      BS.pack [lead .|. Constants.aiOneByte, fromIntegral arg]
  | arg <= Constants.maxUint16 =
      BS.pack
        [ lead .|. Constants.aiTwoBytes
        , fromIntegral (shiftR arg Constants.shiftByte1)
        , fromIntegral arg
        ]
  | arg <= Constants.maxUint32 =
      BS.pack
        [ lead .|. Constants.aiFourBytes
        , fromIntegral (shiftR arg Constants.shiftByte3)
        , fromIntegral (shiftR arg Constants.shiftByte2)
        , fromIntegral (shiftR arg Constants.shiftByte1)
        , fromIntegral arg
        ]
  | otherwise =
      BS.pack
        [ lead .|. Constants.aiEightBytes
        , fromIntegral (shiftR arg Constants.shiftByte7)
        , fromIntegral (shiftR arg Constants.shiftByte6)
        , fromIntegral (shiftR arg Constants.shiftByte5)
        , fromIntegral (shiftR arg Constants.shiftByte4)
        , fromIntegral (shiftR arg Constants.shiftByte3)
        , fromIntegral (shiftR arg Constants.shiftByte2)
        , fromIntegral (shiftR arg Constants.shiftByte1)
        , fromIntegral arg
        ]
  where
    lead = shiftL major Constants.majorTypeShift

-- ============================================================================
-- Public Decoding API
-- ============================================================================

decode :: ByteString -> Either CborError CborValue
decode bs = do
  (val, rest, _) <- decodeValue bs 0 Constants.rootDepth
  if not (BS.null rest)
    then Left (mkCborError TrailingBytes)
    else Right val

decodeMapSpans :: ByteString -> Either CborError DecodeMapSpansResult
decodeMapSpans bs =
  case BS.uncons bs of
    Nothing -> Left (mkCborError UnexpectedEof)
    Just (b0, _) ->
      let major = shiftR (b0 .&. Constants.majorTypeMask) Constants.majorTypeShift
      in if major == Constants.majorMap
           then do
             (spannedEntries, rest) <- decodeSpannedMap bs
             if not (BS.null rest)
               then Left (mkCborError TrailingBytes)
               else Right (DecodedMap spannedEntries)
           else do
             _ <- decode bs
             Right DecodedNonMap

-- ============================================================================
-- Decoding Internals
-- ============================================================================

decodeValue :: ByteString -> Word64 -> Int -> Either CborError (CborValue, ByteString, Word64)
decodeValue bs offset depth
  | depth > Constants.maxNestingDepth = Left (mkCborError TooDeep)
  | otherwise =
      case BS.uncons bs of
        Nothing -> Left (mkCborError UnexpectedEof)
        Just (b0, restAfterByte0) ->
          let major = shiftR (b0 .&. Constants.majorTypeMask) Constants.majorTypeShift
              ai = b0 .&. Constants.additionalInfoMask
          in if major == Constants.majorSimple
               then decodeMajor7 ai restAfterByte0 offset
               else do
                 (arg, restAfterHeader, headerLen) <- readArgument ai restAfterByte0
                 let offsetAfterHeader = offset + headerLen
                 case major of
                   m | m == Constants.majorUnsigned ->
                     Right (CborUnsigned arg, restAfterHeader, offsetAfterHeader)

                   m | m == Constants.majorNegative ->
                     Right (CborNegative arg, restAfterHeader, offsetAfterHeader)

                   m | m == Constants.majorBytes ->
                     if arg > fromIntegral (BS.length restAfterHeader)
                       then Left (mkCborError LengthTooLarge)
                       else do
                         let len = fromIntegral arg :: Int
                             (slice, rest') = BS.splitAt len restAfterHeader
                         Right (CborBytes slice, rest', offsetAfterHeader + arg)

                   m | m == Constants.majorText ->
                     if arg > fromIntegral (BS.length restAfterHeader)
                       then Left (mkCborError LengthTooLarge)
                       else do
                         let len = fromIntegral arg :: Int
                             (slice, rest') = BS.splitAt len restAfterHeader
                         case TE.decodeUtf8' slice of
                           Left _  -> Left (mkCborError InvalidUtf8)
                           Right t -> Right (CborText t, rest', offsetAfterHeader + arg)

                   m | m == Constants.majorArray ->
                     if arg > fromIntegral (BS.length restAfterHeader)
                       then Left (mkCborError LengthTooLarge)
                       else do
                         (items, rest', endOffset) <-
                           decodeArrayItems (fromIntegral arg) restAfterHeader offsetAfterHeader (depth + 1) []
                         Right (CborArray items, rest', endOffset)

                   m | m == Constants.majorMap ->
                     if arg > 0 && (arg * Constants.minBytesPerMapEntry > fromIntegral (BS.length restAfterHeader) || arg * Constants.minBytesPerMapEntry < arg)
                       then Left (mkCborError LengthTooLarge)
                       else do
                         (entries, rest', endOffset) <-
                           decodeMapEntries (fromIntegral arg) restAfterHeader offsetAfterHeader (depth + 1)
                         Right (CborMap entries, rest', endOffset)

                   m | m == Constants.majorTag -> do
                     (inner, rest', endOffset) <-
                       decodeValue restAfterHeader offsetAfterHeader (depth + 1)
                     Right (CborTag arg inner, rest', endOffset)

                   _ ->
                     Left (mkCborError UnsupportedSimple)

readArgument :: Word8 -> ByteString -> Either CborError (Word64, ByteString, Word64)
readArgument ai rest
  | ai <= Constants.aiDirectMax =
      Right (fromIntegral ai, rest, Constants.headerLenDirect)

  | ai == Constants.aiOneByte =
      case BS.uncons rest of
        Nothing -> Left (mkCborError UnexpectedEof)
        Just (b, r) ->
          let val = fromIntegral b
          in if val <= fromIntegral Constants.aiDirectMax
               then Left (mkCborError NonMinimalInteger)
               else Right (val, r, Constants.headerLen1Byte)

  | ai == Constants.aiTwoBytes =
      if (fromIntegral (BS.length rest) :: Word64) < Constants.argBytes2
        then Left (mkCborError UnexpectedEof)
        else
          let b1 = fromIntegral (BS.index rest 0) :: Word64
              b0 = fromIntegral (BS.index rest 1) :: Word64
              val = shiftL b1 Constants.shiftByte1 .|. b0
              r = BS.drop (fromIntegral Constants.argBytes2) rest
          in if val <= Constants.maxUint8
               then Left (mkCborError NonMinimalInteger)
               else Right (val, r, Constants.headerLen2Bytes)

  | ai == Constants.aiFourBytes =
      if (fromIntegral (BS.length rest) :: Word64) < Constants.argBytes4
        then Left (mkCborError UnexpectedEof)
        else
          let b3 = fromIntegral (BS.index rest 0) :: Word64
              b2 = fromIntegral (BS.index rest 1) :: Word64
              b1 = fromIntegral (BS.index rest 2) :: Word64
              b0 = fromIntegral (BS.index rest 3) :: Word64
              val = shiftL b3 Constants.shiftByte3 .|. shiftL b2 Constants.shiftByte2
                .|. shiftL b1 Constants.shiftByte1 .|. b0
              r = BS.drop (fromIntegral Constants.argBytes4) rest
          in if val <= Constants.maxUint16
               then Left (mkCborError NonMinimalInteger)
               else Right (val, r, Constants.headerLen4Bytes)

  | ai == Constants.aiEightBytes =
      if (fromIntegral (BS.length rest) :: Word64) < Constants.argBytes8
        then Left (mkCborError UnexpectedEof)
        else
          let b7 = fromIntegral (BS.index rest 0) :: Word64
              b6 = fromIntegral (BS.index rest 1) :: Word64
              b5 = fromIntegral (BS.index rest 2) :: Word64
              b4 = fromIntegral (BS.index rest 3) :: Word64
              b3 = fromIntegral (BS.index rest 4) :: Word64
              b2 = fromIntegral (BS.index rest 5) :: Word64
              b1 = fromIntegral (BS.index rest 6) :: Word64
              b0 = fromIntegral (BS.index rest 7) :: Word64
              val = shiftL b7 Constants.shiftByte7 .|. shiftL b6 Constants.shiftByte6
                .|. shiftL b5 Constants.shiftByte5 .|. shiftL b4 Constants.shiftByte4
                .|. shiftL b3 Constants.shiftByte3 .|. shiftL b2 Constants.shiftByte2
                .|. shiftL b1 Constants.shiftByte1 .|. b0
              r = BS.drop (fromIntegral Constants.argBytes8) rest
          in if val <= Constants.maxUint32
               then Left (mkCborError NonMinimalInteger)
               else Right (val, r, Constants.headerLen8Bytes)

  | ai >= Constants.aiReserved28 && ai <= Constants.aiReserved30 =
      Left (mkCborError Reserved)

  | ai == Constants.aiIndefinite =
      Left (mkCborError Indefinite)

  | otherwise =
      Left (mkCborError Reserved)

decodeMajor7 :: Word8 -> ByteString -> Word64 -> Either CborError (CborValue, ByteString, Word64)
decodeMajor7 ai rest offset
  | ai == Constants.simpleFalse = Right (CborBool False, rest, offset + Constants.headerLenDirect)
  | ai == Constants.simpleTrue  = Right (CborBool True, rest, offset + Constants.headerLenDirect)
  | ai == Constants.simpleNull  = Right (CborNull, rest, offset + Constants.headerLenDirect)
  | ai == Constants.aiFloat16 || ai == Constants.aiFloat32 || ai == Constants.aiFloat64 =
      Left (mkCborError FloatNotSupported)
  | ai >= Constants.aiReserved28 && ai <= Constants.aiReserved30 = Left (mkCborError Reserved)
  | ai == Constants.aiIndefinite = Left (mkCborError Indefinite)
  | ai == Constants.aiOneByte =
      case BS.uncons rest of
        Nothing -> Left (mkCborError UnexpectedEof)
        Just (b, _) ->
          if b <= fromIntegral Constants.aiDirectMax
            then Left (mkCborError NonMinimalInteger)
            else Left (mkCborError UnsupportedSimple)
  | otherwise = Left (mkCborError UnsupportedSimple)

decodeArrayItems
  :: Int
  -> ByteString
  -> Word64
  -> Int
  -> [CborValue]
  -> Either CborError ([CborValue], ByteString, Word64)
decodeArrayItems 0 rest offset _ acc = Right (reverse acc, rest, offset)
decodeArrayItems count bs offset depth acc = do
  (!item, rest1, offset1) <- decodeValue bs offset depth
  decodeArrayItems (count - 1) rest1 offset1 depth (item : acc)

decodeMapWith
  :: (CborValue -> CborValue -> Span -> e)
  -> Int
  -> ByteString
  -> Word64
  -> Int
  -> Maybe ByteString
  -> [e]
  -> Either CborError ([e], ByteString, Word64)
decodeMapWith _ 0 rest offset _ _ acc = Right (reverse acc, rest, offset)
decodeMapWith mkEntry count bs offset depth maybePrevKeyWire acc = do
  let keyStartOffset = offset
  (!k, restK, offsetK) <- decodeValue bs offset depth
  let keyWireLen = fromIntegral (offsetK - keyStartOffset) :: Int
      keyWire = BS.take keyWireLen bs

  -- Check canonical ordering (strictly increasing length-first; duplicates violate strict order)
  case maybePrevKeyWire of
    Just prevKeyWire ->
      let lPrev = BS.length prevKeyWire
          lCurr = BS.length keyWire
      in if lCurr < lPrev || (lCurr == lPrev && keyWire <= prevKeyWire)
           then Left (mkCborError NonCanonicalMapOrder)
           else Right ()
    Nothing -> Right ()

  let valStartOffset = offsetK
  (!v, restV, offsetV) <- decodeValue restK offsetK depth
  let valSpan = Span valStartOffset offsetV
      !entry = mkEntry k v valSpan
  decodeMapWith mkEntry (count - 1) restV offsetV depth (Just keyWire) (entry : acc)

decodeMapEntries
  :: Int
  -> ByteString
  -> Word64
  -> Int
  -> Either CborError ([CborMapEntry], ByteString, Word64)
decodeMapEntries count bs offset depth =
  decodeMapWith (\k v _sp -> CborMapEntry k v) count bs offset depth Nothing []

-- ============================================================================
-- Spanned Map Decoding
-- ============================================================================

decodeSpannedMap :: ByteString -> Either CborError ([SpannedMapEntry], ByteString)
decodeSpannedMap bs =
  case BS.uncons bs of
    Nothing -> Left (mkCborError UnexpectedEof)
    Just (b0, restAfterByte0) -> do
      let ai = b0 .&. Constants.additionalInfoMask
      (arg, restAfterHeader, headerLen) <- readArgument ai restAfterByte0
      if arg > 0 && (arg * Constants.minBytesPerMapEntry > fromIntegral (BS.length restAfterHeader) || arg * Constants.minBytesPerMapEntry < arg)
        then Left (mkCborError LengthTooLarge)
        else do
          (entries, restFinal, _) <-
            decodeSpannedMapEntries (fromIntegral arg) restAfterHeader headerLen Constants.initialMapDepth
          Right (entries, restFinal)

decodeSpannedMapEntries
  :: Int
  -> ByteString
  -> Word64
  -> Int
  -> Either CborError ([SpannedMapEntry], ByteString, Word64)
decodeSpannedMapEntries count bs offset depth =
  decodeMapWith SpannedMapEntry count bs offset depth Nothing []
