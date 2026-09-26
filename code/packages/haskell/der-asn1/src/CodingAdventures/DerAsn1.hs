module CodingAdventures.DerAsn1
  ( Asn1Limits (..)
  , defaultAsn1Limits
  , Asn1ErrorKind (..)
  , Asn1Error
  , asn1ErrorKind
  , asn1ErrorOffset
  , asn1ErrorId
  , Asn1Decoder
  , newAsn1Decoder
  , decoderLimits
  , decoderElementsRead
  , decodeExact
  , Asn1Element
  , asn1ElementTag
  , asn1ElementHeader
  , asn1ElementValue
  , asn1ElementEncoded
  , asn1ElementDepth
  , Asn1Cursor
  , sequenceCursor
  , setCursor
  , readAsn1Cursor
  , finishAsn1Cursor
  , asn1CursorRemaining
  , decodeExplicit
  , decodeBoolean
  , DerInteger
  , decodeInteger
  , integerSignedBytes
  , integerIsNegative
  , integerToWord64
  , DerBitString
  , decodeBitString
  , bitStringBytes
  , bitStringUnusedBits
  , bitStringLength
  , decodeOctetString
  , decodeImplicitOctetString
  , decodeIa5String
  , decodeImplicitIa5String
  , decodeNull
  , ObjectIdentifier
  , decodeObjectIdentifier
  , decodeImplicitObjectIdentifier
  , oidEncoded
  , oidArcs
  , oidArcCount
  , oidEquals
  , version
  ) where

import Data.Bits ((.&.), shiftL)
import qualified Data.ByteString as BS
import Data.Char (chr)
import Control.Concurrent.MVar (MVar, modifyMVarMasked, newMVar, readMVar)
import qualified Data.Text as T
import Data.Word (Word32, Word64, Word8)
import qualified CodingAdventures.DerTlv as DT

version :: String
version = "0.1.0"

data Asn1Limits = Asn1Limits
  { asn1DerLimits :: DT.DerLimits
  , maxDepth :: Word64
  , maxTotalElements :: Word64
  , maxOidArcs :: Word64
  }
  deriving (Eq)

defaultAsn1Limits :: Asn1Limits
defaultAsn1Limits = Asn1Limits DT.defaultDerLimits 32 16384 128

data Asn1ErrorKind
  = Framing DT.DerErrorKind
  | UnexpectedTag
  | DecoderLimitMismatch
  | DepthLimitExceeded
  | ElementLimitExceeded
  | InvalidBooleanLength
  | InvalidBooleanValue
  | EmptyInteger
  | NonMinimalInteger
  | NegativeInteger
  | IntegerOverflow
  | MissingUnusedBitCount
  | InvalidUnusedBitCount
  | NonZeroBitPadding
  | BitLengthOverflow
  | NonEmptyNull
  | NonAsciiIa5String
  | EmptyObjectIdentifier
  | UnterminatedObjectIdentifier
  | NonMinimalObjectIdentifier
  | ObjectIdentifierOverflow
  | OidArcLimitExceeded
  deriving (Eq, Show)

data Asn1Error = Asn1Error
  { asn1ErrorKind :: Asn1ErrorKind
  , asn1ErrorOffset :: Int
  }

instance Show Asn1Error where
  show err = "ASN.1 DER value error " ++ show (asn1ErrorKind err)
    ++ " at byte " ++ show (asn1ErrorOffset err)

asn1ErrorId :: Asn1ErrorKind -> String
asn1ErrorId kind = case kind of
  Framing _ -> "framing"
  UnexpectedTag -> "unexpected-tag"
  DecoderLimitMismatch -> "decoder-limit-mismatch"
  DepthLimitExceeded -> "depth-limit-exceeded"
  ElementLimitExceeded -> "element-limit-exceeded"
  InvalidBooleanLength -> "invalid-boolean-length"
  InvalidBooleanValue -> "invalid-boolean-value"
  EmptyInteger -> "empty-integer"
  NonMinimalInteger -> "non-minimal-integer"
  NegativeInteger -> "negative-integer"
  IntegerOverflow -> "integer-overflow"
  MissingUnusedBitCount -> "missing-unused-bit-count"
  InvalidUnusedBitCount -> "invalid-unused-bit-count"
  NonZeroBitPadding -> "non-zero-bit-padding"
  BitLengthOverflow -> "bit-length-overflow"
  NonEmptyNull -> "non-empty-null"
  NonAsciiIa5String -> "non-ascii-ia5-string"
  EmptyObjectIdentifier -> "empty-object-identifier"
  UnterminatedObjectIdentifier -> "unterminated-object-identifier"
  NonMinimalObjectIdentifier -> "non-minimal-object-identifier"
  ObjectIdentifierOverflow -> "object-identifier-overflow"
  OidArcLimitExceeded -> "oid-arc-limit-exceeded"

framing :: DT.DerError -> Asn1Error
framing errorValue = Asn1Error (Framing (DT.derErrorKind errorValue)) (DT.derErrorOffset errorValue)

data Asn1Decoder = Asn1Decoder Asn1Limits (MVar Word64)

newAsn1Decoder :: Asn1Limits -> IO Asn1Decoder
newAsn1Decoder limits = Asn1Decoder limits <$> newMVar 0

decoderLimits :: Asn1Decoder -> Asn1Limits
decoderLimits (Asn1Decoder limits _) = limits

decoderElementsRead :: Asn1Decoder -> IO Word64
decoderElementsRead (Asn1Decoder _ counter) = readMVar counter

data Asn1Element = Asn1Element DT.DerElement Word64 (MVar Word64)

asn1ElementTag :: Asn1Element -> DT.DerTag
asn1ElementTag (Asn1Element element _ _) = DT.elementTag element

asn1ElementHeader :: Asn1Element -> BS.ByteString
asn1ElementHeader (Asn1Element element _ _) = DT.elementHeader element

asn1ElementValue :: Asn1Element -> BS.ByteString
asn1ElementValue (Asn1Element element _ _) = DT.elementValue element

asn1ElementEncoded :: Asn1Element -> BS.ByteString
asn1ElementEncoded (Asn1Element element _ _) = DT.elementEncoded element

asn1ElementDepth :: Asn1Element -> Word64
asn1ElementDepth (Asn1Element _ depth _) = depth

valueOffset :: Asn1Element -> Int
valueOffset = BS.length . asn1ElementHeader

requireOwner :: Asn1Decoder -> MVar Word64 -> Either Asn1Error ()
requireOwner (Asn1Decoder _ owner) expected
  | owner == expected = Right ()
  | otherwise = Left (Asn1Error DecoderLimitMismatch 0)

decodeExact :: Asn1Decoder -> BS.ByteString -> IO (Either Asn1Error Asn1Element)
decodeExact (Asn1Decoder limits owner) input
  | maxDepth limits == 0 = pure (Left (Asn1Error DepthLimitExceeded 0))
  | otherwise = modifyMVarMasked owner $ \count ->
      if count >= maxTotalElements limits
        then pure (count, Left (Asn1Error ElementLimitExceeded 0))
        else case DT.decodeExact (asn1DerLimits limits) input of
          Left err -> pure (count, Left (framing err))
          Right element -> pure (count + 1, Right (Asn1Element element 0 owner))

data Asn1Cursor = Asn1Cursor DT.DerCursor Word64 Asn1Limits (MVar Word64)

childDepth :: Asn1Limits -> Asn1Element -> Either Asn1Error Word64
childDepth limits element =
  let depth = asn1ElementDepth element + 1
  in if depth >= maxDepth limits
       then Left (Asn1Error DepthLimitExceeded 0)
       else Right depth

expectTag :: DT.TagClass -> Bool -> Word32 -> Asn1Element -> Either Asn1Error ()
expectTag expectedClass expectedConstructed expectedNumber element =
  let tag = asn1ElementTag element
  in if DT.tagClass tag == expectedClass
        && DT.tagConstructed tag == expectedConstructed
        && DT.tagNumber tag == expectedNumber
       then Right ()
       else Left (Asn1Error UnexpectedTag 0)

constructedCursor :: DT.TagClass -> Word32 -> Asn1Decoder -> Asn1Element -> Either Asn1Error Asn1Cursor
constructedCursor expectedClass expectedNumber decoder@(Asn1Decoder limits owner) element@(Asn1Element _ _ elementOwner) = do
  requireOwner decoder elementOwner
  expectTag expectedClass True expectedNumber element
  depth <- childDepth limits element
  cursor <- either (Left . framing) Right (DT.newCursor (asn1DerLimits limits) (asn1ElementValue element))
  pure (Asn1Cursor cursor depth limits owner)

sequenceCursor :: Asn1Decoder -> Asn1Element -> Either Asn1Error Asn1Cursor
sequenceCursor = constructedCursor DT.Universal 16

setCursor :: Asn1Decoder -> Asn1Element -> Either Asn1Error Asn1Cursor
setCursor = constructedCursor DT.Universal 17

readAsn1Cursor :: Asn1Decoder -> Asn1Cursor -> IO (Either Asn1Error (Maybe Asn1Element, Asn1Cursor))
readAsn1Cursor (Asn1Decoder limits owner) cursor@(Asn1Cursor lower depth cursorLimits cursorOwner)
  | BS.null (DT.cursorRemaining lower) = pure (Right (Nothing, cursor))
  | limits /= cursorLimits || owner /= cursorOwner = pure (Left (Asn1Error DecoderLimitMismatch 0))
  | otherwise = modifyMVarMasked owner $ \count ->
      if count >= maxTotalElements limits
        then pure (count, Left (Asn1Error ElementLimitExceeded 0))
        else case DT.readCursor lower of
          Left err -> pure (count, Left (framing err))
          Right (Nothing, next) -> pure (count, Right (Nothing, Asn1Cursor next depth cursorLimits cursorOwner))
          Right (Just element, next) ->
            pure (count + 1, Right (Just (Asn1Element element depth owner), Asn1Cursor next depth cursorLimits cursorOwner))

finishAsn1Cursor :: Asn1Cursor -> Either Asn1Error ()
finishAsn1Cursor (Asn1Cursor cursor _ _ _) = either (Left . framing) Right (DT.finishCursor cursor)

asn1CursorRemaining :: Asn1Cursor -> BS.ByteString
asn1CursorRemaining (Asn1Cursor cursor _ _ _) = DT.cursorRemaining cursor

decodeExplicit :: Asn1Decoder -> Asn1Element -> Word32 -> IO (Either Asn1Error Asn1Element)
decodeExplicit decoder@(Asn1Decoder limits owner) element@(Asn1Element _ _ elementOwner) tagNumber =
  case requireOwner decoder elementOwner >> expectTag DT.ContextSpecific True tagNumber element >> childDepth limits element of
    Left err -> pure (Left err)
    Right depth -> modifyMVarMasked owner $ \count ->
      if count >= maxTotalElements limits
        then pure (count, Left (Asn1Error ElementLimitExceeded (valueOffset element)))
        else case DT.decodeExact (asn1DerLimits limits) (asn1ElementValue element) of
          Left err -> pure (count, Left (framing err))
          Right child -> pure (count + 1, Right (Asn1Element child depth owner))

expectUniversalPrimitive :: Word32 -> Asn1Element -> Either Asn1Error ()
expectUniversalPrimitive number = expectTag DT.Universal False number

expectContextPrimitive :: Word32 -> Asn1Element -> Either Asn1Error ()
expectContextPrimitive number = expectTag DT.ContextSpecific False number

decodeBoolean :: Asn1Element -> Either Asn1Error Bool
decodeBoolean element = do
  expectUniversalPrimitive 1 element
  case BS.unpack (asn1ElementValue element) of
    [0] -> Right False
    [255] -> Right True
    [_] -> Left (Asn1Error InvalidBooleanValue (valueOffset element))
    _ -> Left (Asn1Error InvalidBooleanLength (valueOffset element))

data DerInteger = DerInteger BS.ByteString Int

integerSignedBytes :: DerInteger -> BS.ByteString
integerSignedBytes (DerInteger bytes _) = bytes

integerIsNegative :: DerInteger -> Bool
integerIsNegative (DerInteger bytes _) = BS.head bytes .&. 0x80 /= 0

integerToWord64 :: DerInteger -> Either Asn1Error Word64
integerToWord64 integer@(DerInteger bytes offset)
  | integerIsNegative integer = Left (Asn1Error NegativeInteger offset)
  | BS.length magnitude > 8 = Left (Asn1Error IntegerOverflow offset)
  | otherwise = Right (BS.foldl' (\value byte -> shiftL value 8 + fromIntegral byte) 0 magnitude)
  where
    magnitude = if BS.head bytes == 0 then BS.tail bytes else bytes

decodeInteger :: Asn1Element -> Either Asn1Error DerInteger
decodeInteger element = do
  expectUniversalPrimitive 2 element
  let bytes = asn1ElementValue element
      offset = valueOffset element
  if BS.null bytes
    then Left (Asn1Error EmptyInteger offset)
    else if BS.length bytes > 1
      && ((BS.index bytes 0 == 0 && BS.index bytes 1 .&. 0x80 == 0)
        || (BS.index bytes 0 == 255 && BS.index bytes 1 .&. 0x80 /= 0))
      then Left (Asn1Error NonMinimalInteger offset)
      else Right (DerInteger bytes offset)

data DerBitString = DerBitString BS.ByteString Word8 Word64

bitStringBytes :: DerBitString -> BS.ByteString
bitStringBytes (DerBitString bytes _ _) = bytes

bitStringUnusedBits :: DerBitString -> Word8
bitStringUnusedBits (DerBitString _ unused _) = unused

bitStringLength :: DerBitString -> Word64
bitStringLength (DerBitString _ _ lengthValue) = lengthValue

decodeBitString :: Asn1Element -> Either Asn1Error DerBitString
decodeBitString element = do
  expectUniversalPrimitive 3 element
  let value = asn1ElementValue element
      offset = valueOffset element
  if BS.null value then Left (Asn1Error MissingUnusedBitCount offset) else do
    let unused = BS.head value
        bytes = BS.tail value
    if unused > 7 || (BS.null bytes && unused /= 0)
      then Left (Asn1Error InvalidUnusedBitCount offset)
      else if unused /= 0 && BS.last bytes .&. (shiftL 1 (fromIntegral unused) - 1) /= 0
        then Left (Asn1Error NonZeroBitPadding (offset + BS.length value - 1))
        else
          let byteCount = fromIntegral (BS.length bytes) :: Word64
          in if byteCount > maxBound `div` 8
               then Left (Asn1Error BitLengthOverflow offset)
               else Right (DerBitString bytes unused (byteCount * 8 - fromIntegral unused))

decodeOctetString :: Asn1Element -> Either Asn1Error BS.ByteString
decodeOctetString element = expectUniversalPrimitive 4 element >> Right (asn1ElementValue element)

decodeImplicitOctetString :: Asn1Element -> Word32 -> Either Asn1Error BS.ByteString
decodeImplicitOctetString element tagNumber = expectContextPrimitive tagNumber element >> Right (asn1ElementValue element)

decodeIa5Contents :: Asn1Element -> Either Asn1Error T.Text
decodeIa5Contents element =
  case BS.findIndex (> 127) bytes of
    Just index -> Left (Asn1Error NonAsciiIa5String (valueOffset element + index))
    Nothing -> Right (T.pack (map (chr . fromIntegral) (BS.unpack bytes)))
  where bytes = asn1ElementValue element

decodeIa5String :: Asn1Element -> Either Asn1Error T.Text
decodeIa5String element = expectUniversalPrimitive 22 element >> decodeIa5Contents element

decodeImplicitIa5String :: Asn1Element -> Word32 -> Either Asn1Error T.Text
decodeImplicitIa5String element tagNumber = expectContextPrimitive tagNumber element >> decodeIa5Contents element

decodeNull :: Asn1Element -> Either Asn1Error ()
decodeNull element = do
  expectUniversalPrimitive 5 element
  if BS.null (asn1ElementValue element)
    then Right ()
    else Left (Asn1Error NonEmptyNull (valueOffset element))

data ObjectIdentifier = ObjectIdentifier BS.ByteString [Word64]

oidEncoded :: ObjectIdentifier -> BS.ByteString
oidEncoded (ObjectIdentifier bytes _) = bytes

oidArcs :: ObjectIdentifier -> [Word64]
oidArcs (ObjectIdentifier _ arcs) = arcs

oidArcCount :: ObjectIdentifier -> Word64
oidArcCount = fromIntegral . length . oidArcs

oidEquals :: ObjectIdentifier -> [Word64] -> Bool
oidEquals oid expected = oidArcs oid == expected

parseBase128 :: BS.ByteString -> Int -> Int -> Either Asn1Error (Word64, Int)
parseBase128 bytes start baseOffset
  | start >= BS.length bytes = Left (Asn1Error UnterminatedObjectIdentifier (baseOffset + start))
  | BS.index bytes start == 0x80 = Left (Asn1Error NonMinimalObjectIdentifier (baseOffset + start))
  | otherwise = go start 0
  where
    go index value
      | index >= BS.length bytes = Left (Asn1Error UnterminatedObjectIdentifier (baseOffset + index))
      | value > (maxBound - fromIntegral payload) `div` 128 = Left (Asn1Error ObjectIdentifierOverflow (baseOffset + index))
      | octet .&. 0x80 == 0 = Right (candidate, index + 1)
      | otherwise = go (index + 1) candidate
      where
        octet = BS.index bytes index
        payload = octet .&. 0x7f
        candidate = value * 128 + fromIntegral payload

decodeOidContents :: Asn1Limits -> Asn1Element -> Either Asn1Error ObjectIdentifier
decodeOidContents limits element = do
  let bytes = asn1ElementValue element
      base = valueOffset element
  if BS.null bytes then Left (Asn1Error EmptyObjectIdentifier base) else do
    (combined, firstEnd) <- parseBase128 bytes 0 base
    let prefix = if combined < 40 then [0, combined]
          else if combined < 80 then [1, combined - 40]
          else [2, combined - 80]
    if maxOidArcs limits < 2
      then Left (Asn1Error OidArcLimitExceeded base)
      else do
        arcs <- parseRest bytes base firstEnd prefix
        Right (ObjectIdentifier bytes arcs)
  where
    parseRest bytes base index arcs
      | index >= BS.length bytes = Right arcs
      | otherwise = do
          (arc, next) <- parseBase128 bytes index base
          let nextArcs = arcs ++ [arc]
          if fromIntegral (length nextArcs) > maxOidArcs limits
            then Left (Asn1Error OidArcLimitExceeded (base + index))
            else parseRest bytes base next nextArcs

decodeObjectIdentifier :: Asn1Element -> Asn1Limits -> Either Asn1Error ObjectIdentifier
decodeObjectIdentifier element limits = expectUniversalPrimitive 6 element >> decodeOidContents limits element

decodeImplicitObjectIdentifier :: Asn1Element -> Word32 -> Asn1Limits -> Either Asn1Error ObjectIdentifier
decodeImplicitObjectIdentifier element tagNumber limits = expectContextPrimitive tagNumber element >> decodeOidContents limits element
