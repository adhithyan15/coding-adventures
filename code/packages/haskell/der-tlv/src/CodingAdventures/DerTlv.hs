module CodingAdventures.DerTlv
  ( DerLimits (..)
  , defaultDerLimits
  , TagClass (..)
  , DerTag (..)
  , DerElement
  , elementTag
  , elementHeader
  , elementValue
  , elementEncoded
  , DerErrorKind (..)
  , DerError (..)
  , errorId
  , decodeOne
  , decodeExact
  , DerCursor
  , newCursor
  , readCursor
  , finishCursor
  , cursorRemaining
  , cursorElementsRead
  , version
  ) where

import Data.Bits ((.&.), shiftR)
import qualified Data.ByteString as BS
import Data.Word (Word32, Word64, Word8)

version :: String
version = "0.1.0"

data DerLimits = DerLimits
  { maxInputLength :: Word64
  , maxValueLength :: Word64
  , maxElements :: Word64
  , maxTagNumber :: Word32
  }
  deriving (Eq, Show)

defaultDerLimits :: DerLimits
defaultDerLimits = DerLimits 1048576 1048576 4096 maxBound

data TagClass = Universal | Application | ContextSpecific | Private
  deriving (Eq, Show)

data DerTag = DerTag
  { tagClass :: TagClass
  , tagConstructed :: Bool
  , tagNumber :: Word32
  }
  deriving (Eq, Show)

data DerElement = DerElement DerTag BS.ByteString Int
  deriving (Eq, Show)

elementHeader :: DerElement -> BS.ByteString
elementHeader (DerElement _ encoded headerLength) = BS.take headerLength encoded

elementValue :: DerElement -> BS.ByteString
elementValue (DerElement _ encoded headerLength) = BS.drop headerLength encoded

elementEncoded :: DerElement -> BS.ByteString
elementEncoded (DerElement _ encoded _) = encoded

elementTag :: DerElement -> DerTag
elementTag (DerElement tag _ _) = tag

data DerErrorKind
  = EmptyInput | TruncatedHighTag | TruncatedLength | TruncatedValue
  | EndOfContents | NonMinimalTag | TagOverflow | IndefiniteLength
  | ReservedLength | NonMinimalLength | LengthTooWide | LengthHostOverflow
  | InputLimitExceeded | ValueLimitExceeded | ElementLimitExceeded
  | TagLimitExceeded | TrailingData
  deriving (Eq, Show)

data DerError = DerError {derErrorKind :: DerErrorKind, derErrorOffset :: Int}
  deriving (Eq, Show)

errorId :: DerErrorKind -> String
errorId kind = case kind of
  EmptyInput -> "empty-input"
  TruncatedHighTag -> "truncated-high-tag"
  TruncatedLength -> "truncated-length"
  TruncatedValue -> "truncated-value"
  EndOfContents -> "end-of-contents"
  NonMinimalTag -> "non-minimal-tag"
  TagOverflow -> "tag-overflow"
  IndefiniteLength -> "indefinite-length"
  ReservedLength -> "reserved-length"
  NonMinimalLength -> "non-minimal-length"
  LengthTooWide -> "length-too-wide"
  LengthHostOverflow -> "length-host-overflow"
  InputLimitExceeded -> "input-limit-exceeded"
  ValueLimitExceeded -> "value-limit-exceeded"
  ElementLimitExceeded -> "element-limit-exceeded"
  TagLimitExceeded -> "tag-limit-exceeded"
  TrailingData -> "trailing-data"

decodeOne :: DerLimits -> BS.ByteString -> Either DerError (DerElement, BS.ByteString)
decodeOne limits input = decodeAt limits 0 input

decodeExact :: DerLimits -> BS.ByteString -> Either DerError DerElement
decodeExact limits input = do
  (element, remainder) <- decodeAt limits 0 input
  if BS.null remainder
    then Right element
    else Left (DerError TrailingData (BS.length (elementEncoded element)))

data DerCursor = DerCursor DerLimits BS.ByteString Int Word64
  deriving (Eq, Show)

newCursor :: DerLimits -> BS.ByteString -> Either DerError DerCursor
newCursor limits input
  | fromIntegral (BS.length input) > maxInputLength limits = Left (DerError InputLimitExceeded 0)
  | otherwise = Right (DerCursor limits input 0 0)

readCursor :: DerCursor -> Either DerError (Maybe DerElement, DerCursor)
readCursor cursor@(DerCursor limits remaining offset count)
  | BS.null remaining = Right (Nothing, cursor)
  | count >= maxElements limits = Left (DerError ElementLimitExceeded offset)
  | otherwise = do
      (element, next) <- decodeAt limits offset remaining
      let consumed = BS.length remaining - BS.length next
      Right (Just element, DerCursor limits next (offset + consumed) (count + 1))

finishCursor :: DerCursor -> Either DerError ()
finishCursor (DerCursor _ remaining offset _)
  | BS.null remaining = Right ()
  | otherwise = Left (DerError TrailingData offset)

cursorRemaining :: DerCursor -> BS.ByteString
cursorRemaining (DerCursor _ remaining _ _) = remaining

cursorElementsRead :: DerCursor -> Word64
cursorElementsRead (DerCursor _ _ _ count) = count

decodeAt :: DerLimits -> Int -> BS.ByteString -> Either DerError (DerElement, BS.ByteString)
decodeAt limits base input
  | fromIntegral available > maxInputLength limits = Left (DerError InputLimitExceeded base)
  | available == 0 = Left (DerError EmptyInput base)
  | otherwise = do
      (tag, identifierLength) <- decodeIdentifier limits base input
      (valueLength, lengthLength, lengthOffset) <- decodeLength base input identifierLength
      if valueLength > maxValueLength limits
        then Left (DerError ValueLimitExceeded lengthOffset)
        else do
          let headerLength = identifierLength + lengthLength
          if valueLength > fromIntegral (maxBound :: Int) - fromIntegral headerLength
            then Left (DerError LengthHostOverflow lengthOffset)
            else do
              let encodedLength = headerLength + fromIntegral valueLength
              if encodedLength > available
                then Left (DerError TruncatedValue (base + available))
                else Right (DerElement tag (BS.take encodedLength input) headerLength, BS.drop encodedLength input)
  where
    available = BS.length input

decodeIdentifier :: DerLimits -> Int -> BS.ByteString -> Either DerError (DerTag, Int)
decodeIdentifier limits base input =
  let first = BS.index input 0
      cls = [Universal, Application, ContextSpecific, Private] !! fromIntegral (first `shiftR` 6)
      constructed = first .&. 0x20 /= 0
      low = first .&. 0x1f
   in if low /= 0x1f
        then finish cls constructed (fromIntegral low) 1 base
        else do
          (number, identifierLength) <- highTag 1 0
          if number < 31
            then Left (DerError NonMinimalTag base)
            else finish cls constructed number identifierLength base
  where
    finish cls constructed number identifierLength tagOffset
      | number > maxTagNumber limits = Left (DerError TagLimitExceeded tagOffset)
      | cls == Universal && number == 0 = Left (DerError EndOfContents base)
      | otherwise = Right (DerTag cls constructed number, identifierLength)
    highTag index number
      | index >= BS.length input = Left (DerError TruncatedHighTag (base + index))
      | index == 1 && payload == 0 = Left (DerError NonMinimalTag (base + index))
      | number > (maxBound - fromIntegral payload) `div` 128 = Left (DerError TagOverflow (base + index))
      | candidate > maxTagNumber limits = Left (DerError TagLimitExceeded (base + index))
      | octet .&. 0x80 == 0 = Right (candidate, index + 1)
      | otherwise = highTag (index + 1) candidate
      where
        octet = BS.index input index
        payload = octet .&. 0x7f
        candidate = number * 128 + fromIntegral payload

decodeLength :: Int -> BS.ByteString -> Int -> Either DerError (Word64, Int, Int)
decodeLength base input identifierLength
  | identifierLength >= BS.length input = Left (DerError TruncatedLength lengthOffset)
  | first < 0x80 = Right (fromIntegral first, 1, lengthOffset)
  | first == 0x80 = Left (DerError IndefiniteLength lengthOffset)
  | first == 0xff = Left (DerError ReservedLength lengthOffset)
  | count > 8 = Left (DerError LengthTooWide lengthOffset)
  | identifierLength + 1 + count > BS.length input = Left (DerError TruncatedLength (base + BS.length input))
  | BS.index input (identifierLength + 1) == 0 = Left (DerError NonMinimalLength (lengthOffset + 1))
  | value < 128 = Left (DerError NonMinimalLength lengthOffset)
  | value > fromIntegral (maxBound :: Int) = Left (DerError LengthHostOverflow lengthOffset)
  | otherwise = Right (value, count + 1, lengthOffset)
  where
    lengthOffset = base + identifierLength
    first = BS.index input identifierLength
    count = fromIntegral (first .&. 0x7f)
    octets = BS.take count (BS.drop (identifierLength + 1) input)
    value = BS.foldl' accumulate 0 octets
    accumulate :: Word64 -> Word8 -> Word64
    accumulate total octet = total * 256 + fromIntegral octet
