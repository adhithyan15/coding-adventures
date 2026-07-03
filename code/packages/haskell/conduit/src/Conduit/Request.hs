-- Conduit.Request — Haskell representation of an HTTP request.
--
-- A `Request` is materialized from a `Ptr CRequest` at the start of each
-- handler call.  The fields that are cheap to read (method, path, query string,
-- content-type, remote address, error message, body) are eagerly copied into
-- Haskell values so the handler can be a pure function over `Request` without
-- repeatedly crossing the FFI boundary.
--
-- Dynamic lookups (named path parameters, query parameters, headers) are lazy:
-- `reqParam`, `reqQuery`, and `reqHeader` call back into C using the stored
-- `_reqPtr`.  These are IO actions because they cross the FFI boundary.
-- The `_reqPtr` is only valid for the duration of the handler call — do not
-- store a `Request` and call these functions after the handler returns.

module Conduit.Request
  ( Request (..)
  , peekRequest
  , reqParam
  , reqQuery
  , reqHeader
  ) where

import qualified Data.ByteString         as BS
import           Data.ByteString         (ByteString)
import qualified Data.Text               as T
import           Data.Text               (Text)
import           Foreign.C.String        (CString, peekCString, withCString)
import           Foreign.C.Types         (CSize (..))
import           Foreign.Marshal.Alloc   (alloca)
import           Foreign.Ptr             (Ptr, castPtr, nullPtr)
import           Foreign.Storable        (peek)

import           Conduit.FFI

-- | An immutable snapshot of the current HTTP request.
--
-- Eagerly-read fields are plain Haskell values; dynamic lookups (param, query,
-- header) return IO because they call back into C via `_reqPtr`.
data Request = Request
  { reqMethod      :: !Text        -- ^ HTTP method, e.g. "GET"
  , reqPath        :: !Text        -- ^ URL path, e.g. "/hello/world"
  , reqQueryString :: !Text        -- ^ raw query string, e.g. "q=foo&page=2"
  , reqBody        :: !ByteString  -- ^ raw request body bytes
  , reqContentType :: !Text        -- ^ Content-Type header value ("" if none)
  , reqRemoteAddr  :: !Text        -- ^ client address, e.g. "127.0.0.1:54321"
  , reqError       :: !Text        -- ^ non-empty only inside an on_error handler
  , _reqPtr        :: !(Ptr CRequest)
    -- ^ The raw C pointer; stored for lazy param/query/header lookups.
    -- ONLY valid during the current handler call.
  }

-- | Materialise a `Request` from a raw C pointer.
-- Called at the top of every handler trampoline in `Conduit.App`.
peekRequest :: Ptr CRequest -> IO Request
peekRequest ptr = do
  method      <- readText =<< conduit_request_method       ptr
  path        <- readText =<< conduit_request_path         ptr
  queryStr    <- readText =<< conduit_request_query_string ptr
  contentType <- readText =<< conduit_request_content_type ptr
  remoteAddr  <- readText =<< conduit_request_remote_addr  ptr
  errMsg      <- readText =<< conduit_request_error        ptr
  body        <- alloca $ \lenPtr -> do
                   bodyPtr <- conduit_request_body ptr lenPtr
                   len     <- peek lenPtr
                   if bodyPtr == nullPtr || len == 0
                     then return BS.empty
                     -- castPtr: Ptr Word8 → CString (Ptr CChar) for packCStringLen
                     else BS.packCStringLen (castPtr bodyPtr, fromIntegral len)
  return Request
    { reqMethod      = method
    , reqPath        = path
    , reqQueryString = queryStr
    , reqBody        = body
    , reqContentType = contentType
    , reqRemoteAddr  = remoteAddr
    , reqError       = errMsg
    , _reqPtr        = ptr
    }
  where
    -- Read a C string pointer into Text.
    -- conduit-capi guarantees all strings are valid UTF-8.
    readText :: CString -> IO Text
    readText cs
      | cs == nullPtr = return T.empty
      | otherwise     = T.pack <$> peekCString cs

-- | Look up a named path parameter (e.g. `:name` in `/hello/:name`).
-- Returns `Nothing` if the parameter is absent.
-- Must be called during the handler invocation; the C pointer is invalid after.
reqParam :: Request -> Text -> IO (Maybe Text)
reqParam req name =
  withCString (T.unpack name) $ \cname ->
    readMaybeText =<< conduit_request_param (_reqPtr req) cname

-- | Look up a named query-string parameter.  Returns `Nothing` if absent.
reqQuery :: Request -> Text -> IO (Maybe Text)
reqQuery req name =
  withCString (T.unpack name) $ \cname ->
    readMaybeText =<< conduit_request_query (_reqPtr req) cname

-- | Look up a request header by name (case-insensitive).  Returns `Nothing` if absent.
reqHeader :: Request -> Text -> IO (Maybe Text)
reqHeader req name =
  withCString (T.unpack name) $ \cname ->
    readMaybeText =<< conduit_request_header (_reqPtr req) cname

-- ── Internal helpers ──────────────────────────────────────────────────────────

-- | Convert a nullable CString to `Maybe Text`.
readMaybeText :: CString -> IO (Maybe Text)
readMaybeText cs
  | cs == nullPtr = return Nothing
  | otherwise     = Just . T.pack <$> peekCString cs
