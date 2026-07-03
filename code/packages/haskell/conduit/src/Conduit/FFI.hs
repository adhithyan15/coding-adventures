-- Conduit.FFI — raw GHC C FFI declarations for conduit-capi.
--
-- This module is the single boundary between Haskell and the Rust cdylib.
-- Every `foreign import ccall` here maps 1-to-1 to a symbol in conduit_capi.h.
--
-- CALLING CONVENTIONS
-- -------------------
-- We use `unsafe` for all quick-return C functions: the call is a direct C call
-- that does not block and does not re-enter Haskell, so there is no need to
-- release the GHC capability (token).  We use `safe` for conduit_server_serve,
-- which blocks indefinitely: the `safe` annotation releases the GHC capability
-- so the RTS can schedule other green threads while C blocks.
--
-- PHANTOM TYPES
-- -------------
-- CApp / CServer / CRequest / CResponse are empty Haskell types used purely as
-- phantom type tags.  A `Ptr CApp` in Haskell corresponds to `ConduitApp*` in C.
-- GHC never allocates values of these types; they exist only to make the pointer
-- types distinct and type-check.
--
-- CALLBACK WRAPPING
-- -----------------
-- `foreign import ccall "wrapper"` produces a function that converts a Haskell
-- IO action into a C-callable function pointer (FunPtr).  The GHC runtime
-- allocates a small C stub ("adjustor"); you must call `freeHaskellFunPtr` when
-- the pointer is no longer needed (conduit-capi calls our ctx_free destructor
-- which does this).

module Conduit.FFI
  ( -- * Phantom handle types
    CApp
  , CServer
  , CRequest
  , CResponse

    -- * C function pointer types
  , HandlerFn
  , AfterFn
  , CtxFreeFn

    -- * FunPtr wrapper constructors ("adjustors")
  , mkHandler
  , mkAfter
  , mkCtxFree

    -- * Error channel
  , conduit_capi_report_error
  , conduit_last_error

    -- * App lifecycle
  , conduit_app_new
  , conduit_app_free
  , conduit_app_set_setting
  , conduit_app_get_setting
  , conduit_app_add_route
  , conduit_app_add_before
  , conduit_app_add_after
  , conduit_app_set_not_found
  , conduit_app_set_error_handler

    -- * Server
  , conduit_server_bind
  , conduit_server_serve
  , conduit_server_serve_background
  , conduit_server_stop
  , conduit_server_local_port
  , conduit_server_running
  , conduit_server_free

    -- * Request accessors
  , conduit_request_method
  , conduit_request_path
  , conduit_request_query_string
  , conduit_request_content_type
  , conduit_request_remote_addr
  , conduit_request_error
  , conduit_request_body
  , conduit_request_param
  , conduit_request_query
  , conduit_request_header

    -- * Response builder / reader
  , conduit_response_new
  , conduit_response_set_header
  , conduit_response_status
  , conduit_response_body
  , conduit_response_header_count
  , conduit_response_header_name
  , conduit_response_header_value
  , conduit_response_free
  , conduit_string_free
  ) where

import Foreign.C.String (CString)
import Foreign.C.Types  (CInt (..), CSize (..))
import Foreign.Ptr      (FunPtr, Ptr)
import Data.Word        (Word16, Word8)

-- ── Phantom handle types ──────────────────────────────────────────────────────

-- | Phantom type tag for ConduitApp*.
data CApp

-- | Phantom type tag for ConduitServer*.
data CServer

-- | Phantom type tag for const ConduitRequest*.
data CRequest

-- | Phantom type tag for ConduitResponse*.
data CResponse

-- ── C function pointer types ──────────────────────────────────────────────────

-- | ConduitHandler: (void* ctx, const ConduitRequest* req) -> ConduitResponse*
type HandlerFn
  =  Ptr ()        -- ^ opaque ctx (a StablePtr to the Haskell handler)
  -> Ptr CRequest  -- ^ borrowed request view (valid only during this call)
  -> IO (Ptr CResponse)

-- | ConduitAfter: (void* ctx, const ConduitRequest* req, ConduitResponse* current)
--   -> ConduitResponse*
--   Receives and owns `current`; must return a response (never NULL).
type AfterFn
  =  Ptr ()        -- ^ opaque ctx
  -> Ptr CRequest  -- ^ borrowed request
  -> Ptr CResponse -- ^ owned current response
  -> IO (Ptr CResponse)

-- | ConduitCtxFree: (void* ctx) -> void
--   Called by conduit-capi when the owning app/server is freed.
type CtxFreeFn = Ptr () -> IO ()

-- ── FunPtr wrapper constructors ───────────────────────────────────────────────
--
-- `foreign import ccall "wrapper"` tells GHC to generate a C-callable stub
-- (an "adjustor") that, when called with the right C arguments, dispatches
-- into the Haskell runtime to invoke the provided IO action.  The resulting
-- FunPtr is what we pass to conduit-capi as the `ConduitHandler`.

foreign import ccall "wrapper"
  mkHandler :: HandlerFn -> IO (FunPtr HandlerFn)

foreign import ccall "wrapper"
  mkAfter :: AfterFn -> IO (FunPtr AfterFn)

foreign import ccall "wrapper"
  mkCtxFree :: CtxFreeFn -> IO (FunPtr CtxFreeFn)

-- ── Error channel ─────────────────────────────────────────────────────────────

-- | Record an error message so conduit-capi can pass it to the on_error handler.
-- Call this from inside a handler before returning nullPtr.
-- `safe` because this is called from within a re-entrant Haskell callback
-- (the trampoline), and conduit-capi may invoke additional Haskell FunPtrs
-- (e.g. the on_error handler) synchronously as a result.
foreign import ccall safe "conduit_capi_report_error"
  conduit_capi_report_error :: CString -> IO ()

-- | Retrieve the thread-local last error (e.g. after conduit_server_bind fails).
-- The returned pointer is valid until the next conduit-capi call on this thread.
foreign import ccall unsafe "conduit_last_error"
  conduit_last_error :: IO CString

-- ── App lifecycle ─────────────────────────────────────────────────────────────

foreign import ccall unsafe "conduit_app_new"
  conduit_app_new :: IO (Ptr CApp)

-- | Free an app that was never passed to conduit_server_bind.
-- Uses `safe` because conduit-capi calls back into Haskell via the registered
-- ctx_free FunPtrs while freeing.  An `unsafe` call holds the GHC capability
-- and re-entering Haskell from within it deadlocks.
foreign import ccall safe "conduit_app_free"
  conduit_app_free :: Ptr CApp -> IO ()

foreign import ccall unsafe "conduit_app_set_setting"
  conduit_app_set_setting :: Ptr CApp -> CString -> CString -> IO ()

-- | Returns an owned CString; free with conduit_string_free.
foreign import ccall unsafe "conduit_app_get_setting"
  conduit_app_get_setting :: Ptr CApp -> CString -> IO CString

foreign import ccall unsafe "conduit_app_add_route"
  conduit_app_add_route
    :: Ptr CApp
    -> CString           -- ^ HTTP method (e.g. "GET")
    -> CString           -- ^ path pattern (e.g. "/hello/:name")
    -> FunPtr HandlerFn  -- ^ C-callable handler
    -> Ptr ()            -- ^ opaque ctx
    -> FunPtr CtxFreeFn  -- ^ ctx destructor
    -> IO ()

foreign import ccall unsafe "conduit_app_add_before"
  conduit_app_add_before
    :: Ptr CApp
    -> FunPtr HandlerFn
    -> Ptr ()
    -> FunPtr CtxFreeFn
    -> IO ()

foreign import ccall unsafe "conduit_app_add_after"
  conduit_app_add_after
    :: Ptr CApp
    -> FunPtr AfterFn
    -> Ptr ()
    -> FunPtr CtxFreeFn
    -> IO ()

foreign import ccall unsafe "conduit_app_set_not_found"
  conduit_app_set_not_found
    :: Ptr CApp
    -> FunPtr HandlerFn
    -> Ptr ()
    -> FunPtr CtxFreeFn
    -> IO ()

foreign import ccall unsafe "conduit_app_set_error_handler"
  conduit_app_set_error_handler
    :: Ptr CApp
    -> FunPtr HandlerFn
    -> Ptr ()
    -> FunPtr CtxFreeFn
    -> IO ()

-- ── Server ────────────────────────────────────────────────────────────────────

-- | Bind host:port and CONSUME `app`.  Returns NULL on error; check
-- conduit_last_error for the reason.
foreign import ccall unsafe "conduit_server_bind"
  conduit_server_bind :: CString -> Word16 -> Ptr CApp -> IO (Ptr CServer)

-- | Start serving requests on the calling thread.  Blocks until the server is
-- stopped.  Uses `safe` so the GHC RTS can schedule other Haskell threads
-- while this call blocks in C.
foreign import ccall safe "conduit_server_serve"
  conduit_server_serve :: Ptr CServer -> IO CInt

-- | Start serving on a background OS thread.  Returns immediately.
foreign import ccall unsafe "conduit_server_serve_background"
  conduit_server_serve_background :: Ptr CServer -> IO CInt

-- | Signal the server to stop.  Returns before the background thread has
-- fully exited; poll conduit_server_running if you need confirmation.
foreign import ccall unsafe "conduit_server_stop"
  conduit_server_stop :: Ptr CServer -> IO ()

-- | The port the server is actually listening on (useful when you bound port 0).
foreign import ccall unsafe "conduit_server_local_port"
  conduit_server_local_port :: Ptr CServer -> IO Word16

-- | 1 if the server is running, 0 otherwise.
foreign import ccall unsafe "conduit_server_running"
  conduit_server_running :: Ptr CServer -> IO CInt

-- Uses `safe`: freeing the server calls all registered ctx_free FunPtrs back
-- into the Haskell runtime.  An `unsafe` call holds the GHC capability, so
-- those re-entrant callbacks would deadlock waiting to acquire it.
foreign import ccall safe "conduit_server_free"
  conduit_server_free :: Ptr CServer -> IO ()

-- ── Request accessors ─────────────────────────────────────────────────────────
-- All return borrowed CStrings valid only for the duration of the handler call.

foreign import ccall unsafe "conduit_request_method"
  conduit_request_method :: Ptr CRequest -> IO CString

foreign import ccall unsafe "conduit_request_path"
  conduit_request_path :: Ptr CRequest -> IO CString

foreign import ccall unsafe "conduit_request_query_string"
  conduit_request_query_string :: Ptr CRequest -> IO CString

foreign import ccall unsafe "conduit_request_content_type"
  conduit_request_content_type :: Ptr CRequest -> IO CString

foreign import ccall unsafe "conduit_request_remote_addr"
  conduit_request_remote_addr :: Ptr CRequest -> IO CString

-- | The error message; non-empty only inside an on_error handler.
foreign import ccall unsafe "conduit_request_error"
  conduit_request_error :: Ptr CRequest -> IO CString

-- | The raw request body; `out_len` receives the byte count.
foreign import ccall unsafe "conduit_request_body"
  conduit_request_body :: Ptr CRequest -> Ptr CSize -> IO (Ptr Word8)

-- | Named path parameter (e.g. `:name` in `/hello/:name`).  NULL if absent.
foreign import ccall unsafe "conduit_request_param"
  conduit_request_param :: Ptr CRequest -> CString -> IO CString

-- | Named query-string parameter.  NULL if absent.
foreign import ccall unsafe "conduit_request_query"
  conduit_request_query :: Ptr CRequest -> CString -> IO CString

-- | Request header by name (case-insensitive).  NULL if absent.
foreign import ccall unsafe "conduit_request_header"
  conduit_request_header :: Ptr CRequest -> CString -> IO CString

-- ── Response builder / reader ─────────────────────────────────────────────────

-- | Build a new response with the given status and body.
-- Status is clamped to 100–599 by conduit-capi.
foreign import ccall unsafe "conduit_response_new"
  conduit_response_new :: Word16 -> Ptr Word8 -> CSize -> IO (Ptr CResponse)

-- | Append a header.  CR/LF/CTL/':'-in-name are stripped by conduit-capi.
foreign import ccall unsafe "conduit_response_set_header"
  conduit_response_set_header :: Ptr CResponse -> CString -> CString -> IO ()

foreign import ccall unsafe "conduit_response_status"
  conduit_response_status :: Ptr CResponse -> IO Word16

foreign import ccall unsafe "conduit_response_body"
  conduit_response_body :: Ptr CResponse -> Ptr CSize -> IO (Ptr Word8)

foreign import ccall unsafe "conduit_response_header_count"
  conduit_response_header_count :: Ptr CResponse -> IO CSize

foreign import ccall unsafe "conduit_response_header_name"
  conduit_response_header_name :: Ptr CResponse -> CSize -> IO CString

foreign import ccall unsafe "conduit_response_header_value"
  conduit_response_header_value :: Ptr CResponse -> CSize -> IO CString

-- | Free a response you built but decided not to return from a handler.
foreign import ccall unsafe "conduit_response_free"
  conduit_response_free :: Ptr CResponse -> IO ()

-- | Free a string returned by conduit_app_get_setting.
foreign import ccall unsafe "conduit_string_free"
  conduit_string_free :: CString -> IO ()
