module CodingAdventures.CanonicalCbor
  ( -- * Codec
    encodeChecked
  , encodeIntoChecked
  , decode
  , decodeMapSpans

    -- * Algebraic Model
  , CborValue(..)
  , CborMapEntry(..)
  , Span(..)
  , mkSpan
  , spanLength
  , SpannedMapEntry(..)
  , DecodeMapSpansResult(..)

    -- * Error Diagnostics & Identifiers (Standing Rule #20)
  , CborError
  , cborErrorId
  , cborErrorMessage
  , CborErrorId(..)
  , mkCborError

    -- * Messages & Placeholders
  , Messages.messageFor
  , Messages.idToText
  , Messages.textToId
  , Messages.allCborErrorIds
  , Messages.allDiagnosticMessages
  , Messages.redactedPlaceholder
  , Messages.invalidPlaceholder
  , Messages.unknownErrorIdMessage

    -- * Constants
  , Constants.maxNestingDepth
  , Constants.maxEncodedBytes
  ) where

import CodingAdventures.CanonicalCbor.Codec
  ( decode
  , decodeMapSpans
  , encodeChecked
  , encodeIntoChecked
  )
import qualified CodingAdventures.CanonicalCbor.Constants as Constants
import qualified CodingAdventures.CanonicalCbor.Messages as Messages
import CodingAdventures.CanonicalCbor.Types
