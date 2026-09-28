module CodingAdventures.X509Extension
  ( X509ExtensionErrorKind (..)
  , X509ExtensionError
  , x509ExtensionErrorKind
  , x509ExtensionErrorOffset
  , x509ExtensionErrorId
  , X509Extension
  , extensionId
  , critical
  , extensionValue
  , decodeX509Extension
  ) where

import qualified Data.ByteString as BS
import CodingAdventures.DerAsn1
import qualified CodingAdventures.DerTlv as DT

data X509ExtensionErrorKind
  = Structure Asn1ErrorKind
  | MissingExtensionId
  | InvalidExtensionId Asn1ErrorKind
  | InvalidCritical Asn1ErrorKind
  | EncodedDefaultCritical
  | MissingExtensionValue
  | InvalidExtensionValue Asn1ErrorKind
  | TrailingElement
  deriving (Eq, Show)

data X509ExtensionError = X509ExtensionError X509ExtensionErrorKind Int

x509ExtensionErrorKind :: X509ExtensionError -> X509ExtensionErrorKind
x509ExtensionErrorKind (X509ExtensionError kind _) = kind

x509ExtensionErrorOffset :: X509ExtensionError -> Int
x509ExtensionErrorOffset (X509ExtensionError _ offset) = offset

x509ExtensionErrorId :: X509ExtensionErrorKind -> String
x509ExtensionErrorId kind = case kind of
  Structure _ -> "structure"
  MissingExtensionId -> "missing-extension-id"
  InvalidExtensionId _ -> "invalid-extension-id"
  InvalidCritical _ -> "invalid-critical"
  EncodedDefaultCritical -> "encoded-default-critical"
  MissingExtensionValue -> "missing-extension-value"
  InvalidExtensionValue _ -> "invalid-extension-value"
  TrailingElement -> "trailing-element"

instance Show X509ExtensionError where
  show err = "X.509 extension error "
    ++ x509ExtensionErrorId (x509ExtensionErrorKind err)
    ++ " at byte " ++ show (x509ExtensionErrorOffset err)

data X509Extension = X509Extension ObjectIdentifier Bool BS.ByteString

extensionId :: X509Extension -> ObjectIdentifier
extensionId (X509Extension oid _ _) = oid

critical :: X509Extension -> Bool
critical (X509Extension _ value _) = value

extensionValue :: X509Extension -> BS.ByteString
extensionValue (X509Extension _ _ value) = value

decodeX509Extension :: Asn1Decoder -> Asn1Element -> IO (Either X509ExtensionError X509Extension)
decodeX509Extension decoder root = case sequenceCursor decoder root of
  Left err -> pure (Left (structureError err (asn1ErrorOffset err)))
  Right start -> do
    first <- readChild decoder valueBase valueLength start
    case first of
      Left err -> pure (Left err)
      Right (Nothing, _) -> pure (Left (X509ExtensionError MissingExtensionId valueBase))
      Right (Just oidElement, afterOid) ->
        case decodeObjectIdentifier oidElement (decoderLimits decoder) of
          Left err -> pure (Left (semanticError InvalidExtensionId valueBase err))
          Right oid -> decodeAfterOid oid afterOid
  where
    valueBase = BS.length (asn1ElementHeader root)
    valueLength = BS.length (asn1ElementValue root)

    decodeAfterOid oid cursor = do
      second <- readChild decoder valueBase valueLength cursor
      case second of
        Left err -> pure (Left err)
        Right (Nothing, _) -> pure (Left (X509ExtensionError MissingExtensionValue (childOffset valueBase valueLength cursor)))
        Right (Just element, next) ->
          if DT.tagNumber (asn1ElementTag element) == 1
            then case decodeBoolean element of
              Left err -> pure (Left (semanticError InvalidCritical (childOffset valueBase valueLength cursor) err))
              Right False -> pure (Left (X509ExtensionError EncodedDefaultCritical (childOffset valueBase valueLength cursor)))
              Right True -> readValue oid True next
            else decodeValue oid False element (childOffset valueBase valueLength cursor) next

    readValue oid isCritical cursor = do
      valueResult <- readChild decoder valueBase valueLength cursor
      case valueResult of
        Left err -> pure (Left err)
        Right (Nothing, _) -> pure (Left (X509ExtensionError MissingExtensionValue (childOffset valueBase valueLength cursor)))
        Right (Just element, next) ->
          decodeValue oid isCritical element (childOffset valueBase valueLength cursor) next

    decodeValue oid isCritical element offset cursor =
      case decodeOctetString element of
        Left err -> pure (Left (semanticError InvalidExtensionValue offset err))
        Right bytes -> do
          trailing <- readChild decoder valueBase valueLength cursor
          pure $ case trailing of
            Left err -> Left err
            Right (Just _, _) -> Left (X509ExtensionError TrailingElement (childOffset valueBase valueLength cursor))
            Right (Nothing, _) -> Right (X509Extension oid isCritical bytes)

childOffset :: Int -> Int -> Asn1Cursor -> Int
childOffset valueBase valueLength cursor =
  valueBase + valueLength - BS.length (asn1CursorRemaining cursor)

readChild
  :: Asn1Decoder
  -> Int
  -> Int
  -> Asn1Cursor
  -> IO (Either X509ExtensionError (Maybe Asn1Element, Asn1Cursor))
readChild decoder valueBase valueLength cursor = do
  result <- readAsn1Cursor decoder cursor
  pure $ case result of
    Right value -> Right value
    Left err -> Left (structureError err (mappedOffset err))
  where
    start = childOffset valueBase valueLength cursor
    mappedOffset err = case asn1ErrorKind err of
      Framing _ -> valueBase + asn1ErrorOffset err
      _ -> start + asn1ErrorOffset err

structureError :: Asn1Error -> Int -> X509ExtensionError
structureError err offset = X509ExtensionError (Structure (asn1ErrorKind err)) offset

semanticError
  :: (Asn1ErrorKind -> X509ExtensionErrorKind)
  -> Int
  -> Asn1Error
  -> X509ExtensionError
semanticError constructor offset err =
  X509ExtensionError (constructor (asn1ErrorKind err)) (offset + asn1ErrorOffset err)
