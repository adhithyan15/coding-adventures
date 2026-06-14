{-# LANGUAGE OverloadedStrings #-}
-- Conduit.Response — Haskell response type and builder helpers.
--
-- A `Response` is a pure Haskell record.  The trampolines in `Conduit.App`
-- convert it to a `Ptr CResponse` (owned by conduit-capi) just before returning
-- from the handler.
--
-- HALT
-- ----
-- `halt` throws a `ConduitHalt` exception.  Trampolines catch it and return
-- its embedded response directly, short-circuiting normal handler flow.
-- This mirrors the `halt` in the Swift / C# / Dart ports.
--
-- REDIRECT SAFETY
-- ---------------
-- `redirect` rejects characters that are illegal in HTTP header values:
--   CR (\r), LF (\n)   — classic header-injection / response-splitting
--   NUL (\x00)         — truncates the C string in `withCString`; an attacker
--                        can use this to produce a truncated Location header
--   VT (\x0B), FF (\x0C) — treated as line terminators by some proxies
-- conduit-capi also strips CR/LF, but we validate eagerly so the error is
-- visible to the Haskell caller rather than being silently dropped.

module Conduit.Response
  ( Response (..)
  , ConduitHalt (..)

    -- * Constructors
  , respond
  , html
  , json
  , textPlain
  , redirect
  , halt

    -- * Low-level: materialise into a C response pointer
  , pokeResponse
  ) where

import           Control.Exception     (Exception, throwIO)
import qualified Data.ByteString       as BS
import           Data.ByteString       (ByteString)
import qualified Data.Text             as T
import           Data.Text             (Text)
import qualified Data.Text.Encoding    as TE
import           Foreign.C.String      (withCString)
import           Foreign.C.Types       (CSize (..))
import           Foreign.Ptr           (Ptr, castPtr)
import           Data.Word             (Word16)

import           Conduit.FFI

-- ── Haskell response type ─────────────────────────────────────────────────────

-- | An HTTP response: status code, body bytes, and response headers.
data Response = Response
  { respStatus  :: !Word16              -- ^ HTTP status code (100–599)
  , respBody    :: !ByteString          -- ^ raw response body
  , respHeaders :: ![(Text, Text)]      -- ^ (name, value) pairs
  } deriving (Show, Eq)

-- ── Halt exception ────────────────────────────────────────────────────────────

-- | Thrown by `halt` to short-circuit handler dispatch.
-- The trampoline catches this and returns the embedded response.
newtype ConduitHalt = ConduitHalt Response
  deriving (Show)

instance Exception ConduitHalt

-- ── Builder helpers ───────────────────────────────────────────────────────────

-- | Build a bare response with no headers.
respond :: Word16 -> ByteString -> Response
respond status body = Response status body []

-- | Build an HTML response with Content-Type: text/html; charset=utf-8.
html :: Word16 -> Text -> Response
html status body = Response status (TE.encodeUtf8 body)
  [("Content-Type", "text/html; charset=utf-8")]

-- | Build a JSON response with Content-Type: application/json.
json :: Word16 -> Text -> Response
json status body = Response status (TE.encodeUtf8 body)
  [("Content-Type", "application/json")]

-- | Build a plain-text response with Content-Type: text/plain; charset=utf-8.
textPlain :: Word16 -> Text -> Response
textPlain status body = Response status (TE.encodeUtf8 body)
  [("Content-Type", "text/plain; charset=utf-8")]

-- | Build a redirect response.
-- Throws if `location` contains characters illegal in HTTP header values.
redirect :: Word16 -> Text -> Response
redirect status location
  | T.any isBadHeaderChar location =
      error "Conduit.Response.redirect: location must not contain CR, LF, NUL, VT, or FF"
  | otherwise =
      Response status BS.empty [("Location", location)]
  where
    -- NUL truncates the C string in withCString; CR/LF/VT/FF enable header
    -- injection and HTTP response splitting in various proxy implementations.
    isBadHeaderChar c = c == '\r' || c == '\n'
                     || c == '\x00' || c == '\x0B' || c == '\x0C'

-- | Short-circuit the current handler with the given response.
-- Throws `ConduitHalt`; the trampoline catches it.
halt :: Word16 -> ByteString -> IO a
halt status body = throwIO (ConduitHalt (respond status body))

-- ── C interop ─────────────────────────────────────────────────────────────────

-- | Materialise a `Response` into a freshly-allocated `Ptr CResponse`.
-- The returned pointer is owned by conduit-capi once returned from a handler;
-- the caller must NOT free it (conduit-capi does so after writing the HTTP
-- response).
pokeResponse :: Response -> IO (Ptr CResponse)
pokeResponse (Response status body headers) =
  BS.useAsCStringLen body $ \(bodyPtr, bodyLen) -> do
    ptr <- conduit_response_new status (castPtr bodyPtr) (fromIntegral bodyLen)
    mapM_ (setHeader ptr) headers
    return ptr
  where
    setHeader ptr (name, value) =
      withCString (T.unpack name) $ \cname ->
        withCString (T.unpack value) $ \cvalue ->
          conduit_response_set_header ptr cname cvalue
