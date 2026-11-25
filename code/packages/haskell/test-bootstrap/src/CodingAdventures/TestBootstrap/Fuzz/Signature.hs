-- | Crash signatures, kind normalisation and the one public failure line
-- (VER06 5.2, 6 and 9.3; VER06o 5.2 and 7).
--
-- Nothing in here reads an exception MESSAGE and nothing calls @show@ on a value
-- the target supplied: a kind comes from the exception's TYPE constructor name.
module CodingAdventures.TestBootstrap.Fuzz.Signature
  ( crashSignature
  , duplicateCrash
  , formatTapFailure
  , tapToken
  , tapTokenBytes
  , tapFailures
  , kindOf
  , sanitizeKind
  ) where

import Control.Exception (SomeException (..))
import Control.Monad (when)
import Data.Bits ((.&.))
import qualified Data.ByteString as B
import Data.Char (isAscii, isAlphaNum)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Typeable (tyConName, typeOf, typeRepTyCon)
import Data.Word (Word8)

import CodingAdventures.TestBootstrap.Error (argError)
import CodingAdventures.TestBootstrap.Fuzz.FailureIds
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Fuzz.Types
import CodingAdventures.TestBootstrap.Prng (fnv1a64)
import CodingAdventures.TestBootstrap.Util (hexLower)

-- | The low 32 bits of @fnv1a64 (kind <> ":" <> frames joined by ":")@, as 8
-- lowercase hex digits.  Frames are NOT escaped (VER06 9.4 item 6): one frame
-- @"x:y"@ and two frames @"x"@, @"y"@ hash alike.  This lane never produces a
-- frame at all (GHC exceptions carry no portable stack), so the ambiguity cannot
-- arise from Haskell targets.
crashSignature :: Text -> [Text] -> Text
crashSignature kind frames =
  let bytes = TE.encodeUtf8 (kind <> ":" <> T.intercalate ":" frames)
      low = fnv1a64 bytes .&. 0xFFFFFFFF
      hex8 = hexLower (B.pack (map (fromIntegral . (low `div`)) [0x1000000, 0x10000, 0x100, 1]))
   in T.pack hex8

duplicateCrash :: Text -> [Text] -> Text -> [Text] -> Bool
duplicateCrash ka fa kb fb = crashSignature ka fa == crashSignature kb fb

-- | The target token, exactly as VER06 9.3 and every other lane: truncated to 128
-- BYTES first, then every BYTE outside @[A-Za-z0-9._/-]@ (a control character, a
-- space, @#@, a newline, each byte of a multi-byte character) becomes @_@.
tapTokenBytes :: B.ByteString -> B.ByteString
tapTokenBytes = B.map keep . B.take maxTargetLen
  where
    keep :: Word8 -> Word8
    keep w
      | (w >= 0x30 && w <= 0x39) || (w >= 0x41 && w <= 0x5A) || (w >= 0x61 && w <= 0x7A) || w == 0x2E || w == 0x5F || w == 0x2F || w == 0x2D = w
      | otherwise = 0x5F

tapToken :: Text -> Text
tapToken = TE.decodeLatin1 . tapTokenBytes . TE.encodeUtf8

-- | The only public failure line: @not ok N - token [failure-id:hex8]@.  It
-- carries no input bytes, seed, message or stack trace (VER00 7.3).  The
-- registered-grammar fields are CHECKED, not escaped.
formatTapFailure :: Integer -> Text -> Text -> Text -> IO Text
formatTapFailure n target failureId sig = do
  when (n < 1 || n > 2147483647) (argError "test_num must be in [1, 2147483647]")
  when (not (isRegistered failureId)) (argError "failure id is not one of the registered fuzz failure ids")
  when (not (isHex8 sig)) (argError "sig hash must be exactly 8 lowercase hex digits")
  pure ("not ok " <> T.pack (show n) <> " - " <> tapToken target <> " [" <> failureId <> ":" <> sig <> "]")
  where
    isHex8 s = T.length s == 8 && T.all (\c -> isAscii c && (c `elem` ("0123456789abcdef" :: String))) s

-- | A report as public, opaque failure lines (none for a clean one): one per
-- crash signature, then one for timeouts and one for ooms, whose hash is the
-- signature of the bare kind (they have no stack to hash).
tapFailures :: FuzzReport -> Text -> Integer -> IO [Text]
tapFailures report target firstTest = do
  crashLines <- mapM (\(i, rec') -> formatTapFailure (firstTest + i) target failureCrash (crSignature rec')) (zip [0 ..] (rCrashes report))
  let extras =
        [(failureTimeout, "timeout") | rTimeouts report > 0]
          ++ [(failureOom, "oom") | rOoms report > 0]
  extraLines <-
    mapM
      (\(i, (fid, kind)) -> formatTapFailure (firstTest + toInteger (length crashLines) + i) target fid (crashSignature kind []))
      (zip [0 ..] extras)
  pure (crashLines ++ extraLines)

-- | The crash kind of a caught exception: its type constructor name and nothing
-- else (not its message, not any field).  Sanitised, capped at 64.
kindOf :: SomeException -> Text
kindOf (SomeException e) = sanitizeKind (T.pack (tyConName (typeRepTyCon (typeOf e))))

-- | Every character outside @[A-Za-z0-9._<>$/-]@ becomes @_@, capped at 64
-- characters; empty input is @<unreadable>@.
sanitizeKind :: Text -> Text
sanitizeKind t
  | T.null t = "<unreadable>"
  | otherwise = T.map ok (T.take maxKindLen t)
  where
    ok c
      | isAscii c && (isAlphaNum c || c `elem` ("._<>$/-" :: String)) = c
      | otherwise = '_'
