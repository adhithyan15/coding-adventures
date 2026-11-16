module CodingAdventures.CanonicalCbor.Types
  ( CborErrorId(..)
  , CborError
  , cborErrorId
  , cborErrorMessage
  , mkCborError
  , Span(..)
  , mkSpan
  , spanLength
  , CborMapEntry(..)
  , SpannedMapEntry(..)
  , DecodeMapSpansResult(..)
  , CborValue(..)
  ) where

import Control.Exception (Exception)
import Data.ByteString (ByteString)
import Data.Text (Text)
import qualified Data.Text as T
import Data.Word (Word64)
import CodingAdventures.CanonicalCbor.Messages (CborErrorId(..))
import qualified CodingAdventures.CanonicalCbor.Messages as Messages

data CborError = CborError
  { cborErrorId :: !CborErrorId
  , cborErrorMessage :: !Text
  } deriving (Eq)

instance Show CborError where
  show (CborError eid msg) =
    "CborError {id = " ++ show eid ++ ", message = " ++ show (T.unpack msg) ++ "}"

instance Exception CborError

mkCborError :: CborErrorId -> CborError
mkCborError eid = CborError eid (Messages.messageFor eid)

data Span = Span
  { spanStart :: !Word64
  , spanFinish :: !Word64
  } deriving (Eq)

mkSpan :: Word64 -> Word64 -> Maybe Span
mkSpan s f
  | s <= f = Just (Span s f)
  | otherwise = Nothing

spanLength :: Span -> Word64
spanLength (Span s f) = f - s

instance Show Span where
  show (Span s f) = "[" ++ show s ++ ", " ++ show f ++ ")"

data CborMapEntry = CborMapEntry
  { cborMapKey :: !CborValue
  , cborMapVal :: !CborValue
  } deriving (Eq, Show)

data SpannedMapEntry = SpannedMapEntry
  { spannedKey :: !CborValue
  , spannedVal :: !CborValue
  , spannedValueSpan :: !Span
  } deriving (Eq)

-- | Custom Show instance strictly redacting key and value payloads to prevent
-- confidential data leakage in logging or debug outputs (Standing Rule #20).
instance Show SpannedMapEntry where
  show (SpannedMapEntry _ _ sp) =
    "SpannedMapEntry(key: " ++ T.unpack Messages.redactedPlaceholder ++
    ", value: " ++ T.unpack Messages.redactedPlaceholder ++
    ", value_span: " ++ show sp ++ ")"

data DecodeMapSpansResult
  = DecodedMap ![SpannedMapEntry]
  | DecodedNonMap
  deriving (Eq, Show)

data CborValue
  = CborUnsigned !Word64
  | CborNegative !Word64
  | CborBytes !ByteString
  | CborText !Text
  | CborArray ![CborValue]
  | CborMap ![CborMapEntry]
  | CborTag !Word64 !CborValue
  | CborBool !Bool
  | CborNull
  deriving (Eq, Show)
