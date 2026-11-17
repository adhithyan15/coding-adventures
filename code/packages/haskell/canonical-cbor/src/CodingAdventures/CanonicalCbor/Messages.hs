{-# LANGUAGE OverloadedStrings #-}
module CodingAdventures.CanonicalCbor.Messages
  ( CborErrorId(..)
  , messageFor
  , idToText
  , textToId
  , allCborErrorIds
  , allDiagnosticMessages
  , redactedPlaceholder
  , invalidPlaceholder
  , unknownErrorIdMessage
  ) where

import Data.Text (Text)

data CborErrorId
  = UnexpectedEof
  | TrailingBytes
  | Reserved
  | Indefinite
  | NonMinimalInteger
  | InvalidUtf8
  | NonCanonicalMapOrder
  | UnsupportedSimple
  | FloatNotSupported
  | TooDeep
  | LengthTooLarge
  | DuplicateMapKey
  | EncodeTooDeep
  | EncodeTooLarge
  | UnknownErrorId
  deriving (Eq, Ord, Show, Enum, Bounded)

redactedPlaceholder :: Text
redactedPlaceholder = "<redacted>"

invalidPlaceholder :: Text
invalidPlaceholder = "<invalid>"

unknownErrorIdMessage :: Text
unknownErrorIdMessage = "canonical-cbor: unknown error id"

messageFor :: CborErrorId -> Text
messageFor UnexpectedEof        = "canonical-cbor: unexpected end of input"
messageFor TrailingBytes        = "canonical-cbor: trailing bytes after decoded item"
messageFor Reserved             = "canonical-cbor: reserved additional-info value"
messageFor Indefinite           = "canonical-cbor: indefinite item rejected"
messageFor NonMinimalInteger    = "canonical-cbor: argument is not in smallest form"
messageFor InvalidUtf8          = "canonical-cbor: text is not valid UTF-8"
messageFor NonCanonicalMapOrder = "canonical-cbor: map key order is not canonical"
messageFor UnsupportedSimple    = "canonical-cbor: unsupported simple value"
messageFor FloatNotSupported    = "canonical-cbor: floats are not supported"
messageFor TooDeep              = "canonical-cbor: nesting depth limit exceeded"
messageFor LengthTooLarge       = "canonical-cbor: length exceeds remaining input"
messageFor DuplicateMapKey      = "canonical-cbor: duplicate map key"
messageFor EncodeTooDeep        = "canonical-cbor: nesting depth limit exceeded during encoding"
messageFor EncodeTooLarge       = "canonical-cbor: encoded output exceeds maximum size"
messageFor UnknownErrorId       = unknownErrorIdMessage

idToText :: CborErrorId -> Text
idToText UnexpectedEof        = "unexpected-eof"
idToText TrailingBytes        = "trailing-bytes"
idToText Reserved             = "reserved"
idToText Indefinite           = "indefinite"
idToText NonMinimalInteger    = "non-minimal-integer"
idToText InvalidUtf8          = "invalid-utf8"
idToText NonCanonicalMapOrder = "non-canonical-map-order"
idToText UnsupportedSimple    = "unsupported-simple"
idToText FloatNotSupported    = "float-not-supported"
idToText TooDeep              = "too-deep"
idToText LengthTooLarge       = "length-too-large"
idToText DuplicateMapKey      = "duplicate-map-key"
idToText EncodeTooDeep        = "encode-too-deep"
idToText EncodeTooLarge       = "encode-too-large"
idToText UnknownErrorId       = "unknown"

textToId :: Text -> CborErrorId
textToId "unexpected-eof"          = UnexpectedEof
textToId "trailing-bytes"          = TrailingBytes
textToId "reserved"                = Reserved
textToId "indefinite"              = Indefinite
textToId "non-minimal-integer"     = NonMinimalInteger
textToId "invalid-utf8"            = InvalidUtf8
textToId "non-canonical-map-order" = NonCanonicalMapOrder
textToId "unsupported-simple"      = UnsupportedSimple
textToId "float-not-supported"     = FloatNotSupported
textToId "too-deep"                = TooDeep
textToId "length-too-large"        = LengthTooLarge
textToId "duplicate-map-key"       = DuplicateMapKey
textToId "encode-too-deep"         = EncodeTooDeep
textToId "encode-too-large"        = EncodeTooLarge
textToId _                         = UnknownErrorId

allCborErrorIds :: [CborErrorId]
allCborErrorIds =
  [ UnexpectedEof
  , TrailingBytes
  , Reserved
  , Indefinite
  , NonMinimalInteger
  , InvalidUtf8
  , NonCanonicalMapOrder
  , UnsupportedSimple
  , FloatNotSupported
  , TooDeep
  , LengthTooLarge
  , DuplicateMapKey
  , EncodeTooDeep
  , EncodeTooLarge
  ]

allDiagnosticMessages :: [Text]
allDiagnosticMessages = map messageFor allCborErrorIds
