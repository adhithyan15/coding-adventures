-- Conduit.App — Application type and route/filter registration.
--
-- This module provides the Haskell DSL for building a Conduit application.
-- The central challenge is bridging Haskell closures (arbitrary heap objects
-- managed by the GHC garbage collector) to the C callback model (a function
-- pointer + an opaque `void*` context passed at every invocation).
--
-- THE CLOSURE BOXING STRATEGY
-- ---------------------------
-- GHC's garbage collector moves heap objects, so a raw pointer into the GHC
-- heap would become a dangling pointer as soon as the GC runs.  We use two
-- GHC primitives to produce stable C-safe pointers:
--
--  1. `StablePtr a` — pins a Haskell value so the GC will not move or collect
--     it until `freeStablePtr` is called.  `newStablePtr` returns a stable
--     pointer that can be cast to `Ptr ()` and used as a C `void*`.
--
--  2. `FunPtr f` — a C-callable function pointer (an "adjustor" stub allocated
--     by GHC) that, when called from C with the right argument types, enters
--     the GHC runtime and invokes the Haskell IO action.  Produced by
--     `mkHandler` / `mkAfter` / `mkCtxFree` (the "wrapper" imports in FFI.hs).
--
-- TRAMPOLINE PATTERN
-- ------------------
-- Each `addRoute` / `before` / `after` / `notFound` / `onError` call:
--   1. Allocates a StablePtr for the user's Haskell closure.
--   2. Allocates a FunPtr (C adjustor) for the trampoline that reads the
--      closure from the StablePtr and calls it.
--   3. Registers both with conduit-capi; ctx = the StablePtr cast to Ptr ().
--   4. The ctx_free FunPtr calls freeHaskellFunPtr + freeStablePtr, so
--      conduit-capi's cleanup path releases all memory.
--
-- EXCEPTION HANDLING
-- ------------------
-- Every trampoline wraps the user call in `try`:
--   - ConduitHalt   → return the embedded Response directly.
--   - Any other IO exception → conduit_capi_report_error(msg) + return nullPtr
--     (conduit-capi routes the request through the on_error handler).

module Conduit.App
  ( Application (..)
  , newApplication
  , addRoute
  , Conduit.App.get
  , Conduit.App.post
  , Conduit.App.put
  , Conduit.App.delete
  , Conduit.App.patch
  , Conduit.App.options
  , before
  , Conduit.App.after
  , notFound
  , onError
  , setSetting
  , getSetting
  , bind
  ) where

import           Control.Exception         (SomeException, finally, fromException, try)
import           System.IO                 (hPutStrLn, stderr)
import qualified Data.ByteString           as BS
import qualified Data.Text                 as T
import           Data.Text                 (Text)
import           Data.Word                 (Word16)
import           Foreign.C.String          (peekCString, withCString)
import           Foreign.C.Types           (CSize (..))
import           Foreign.Marshal.Alloc     (alloca)
import           Foreign.Ptr               ( Ptr, FunPtr, castPtr, nullPtr
                                           , freeHaskellFunPtr )
import           Foreign.Storable          (peek)
import           Foreign.StablePtr         ( StablePtr, deRefStablePtr
                                           , freeStablePtr, newStablePtr
                                           , castStablePtrToPtr
                                           , castPtrToStablePtr )

import           Conduit.FFI
import           Conduit.Request           (Request, peekRequest)
import           Conduit.Response          (Response (..), ConduitHalt (..),
                                            pokeResponse)
import           Conduit.Server            (Server (..))

-- ── Application type ──────────────────────────────────────────────────────────

-- | An opaque handle to a Conduit application under construction.
-- Once passed to `bind`, it is consumed by conduit-capi and must not be used.
newtype Application = Application (Ptr CApp)

-- ── Trampoline builders ───────────────────────────────────────────────────────

-- | Create a C-callable (FunPtr, ctx, ctxFree) triple for a request handler.
-- The user's closure is pinned with a StablePtr; the ctxFree destructor frees
-- both the FunPtr (adjustor) and the StablePtr.
mkHandlerTriple
  :: (Request -> IO Response)
  -> IO (FunPtr HandlerFn, Ptr (), FunPtr CtxFreeFn)
mkHandlerTriple userFn = do
  -- Pin the closure.
  stablePtr <- newStablePtr userFn
  let ctx = castStablePtrToPtr stablePtr

  -- The C-callable trampoline.
  let trampoline ctxPtr reqPtr = do
        fn  <- deRefStablePtr
                 (castPtrToStablePtr ctxPtr :: StablePtr (Request -> IO Response))
        req <- peekRequest reqPtr
        result <- try (fn req) :: IO (Either SomeException Response)
        case result of
          Right resp -> pokeResponse resp
          Left ex ->
            case fromException ex of
              Just (ConduitHalt resp) -> pokeResponse resp
              Nothing -> do
                -- Log the full exception to stderr rather than reflecting it to
                -- the HTTP client (Finding #2: prevents information disclosure).
                hPutStrLn stderr ("[conduit] unhandled exception: " ++ show ex)
                withCString "internal server error" conduit_capi_report_error
                return nullPtr

  funPtr <- mkHandler trampoline

  -- The ctx_free destructor: releases the FunPtr adjustor and the StablePtr.
  ctxFreePtr <- mkCtxFree $ \ctxPtr -> do
    freeHaskellFunPtr funPtr
    freeStablePtr
      (castPtrToStablePtr ctxPtr :: StablePtr (Request -> IO Response))

  return (funPtr, ctx, ctxFreePtr)

-- | Same as `mkHandlerTriple` but for a before-filter that returns Maybe Response.
-- `Nothing` → continue; `Just resp` → short-circuit.
mkBeforeTriple
  :: (Request -> IO (Maybe Response))
  -> IO (FunPtr HandlerFn, Ptr (), FunPtr CtxFreeFn)
mkBeforeTriple userFn = do
  stablePtr <- newStablePtr userFn
  let ctx = castStablePtrToPtr stablePtr

  let trampoline ctxPtr reqPtr = do
        fn  <- deRefStablePtr
                 (castPtrToStablePtr ctxPtr
                    :: StablePtr (Request -> IO (Maybe Response)))
        req <- peekRequest reqPtr
        result <- try (fn req) :: IO (Either SomeException (Maybe Response))
        case result of
          Right Nothing    -> return nullPtr
          Right (Just r)   -> pokeResponse r
          Left ex -> do
            hPutStrLn stderr ("[conduit] before-filter exception: " ++ show ex)
            withCString "internal server error" conduit_capi_report_error
            return nullPtr

  funPtr <- mkHandler trampoline

  ctxFreePtr <- mkCtxFree $ \ctxPtr -> do
    freeHaskellFunPtr funPtr
    freeStablePtr
      (castPtrToStablePtr ctxPtr
         :: StablePtr (Request -> IO (Maybe Response)))

  return (funPtr, ctx, ctxFreePtr)

-- | Create a (FunPtr AfterFn, ctx, ctxFree) triple for an after-hook.
-- conduit-capi passes ownership of the current response pointer to us;
-- we read it into a Haskell `Response`, free the C pointer, call the hook,
-- and poke the result into a fresh C pointer.
mkAfterTriple
  :: (Request -> Response -> IO Response)
  -> IO (FunPtr AfterFn, Ptr (), FunPtr CtxFreeFn)
mkAfterTriple userFn = do
  stablePtr <- newStablePtr userFn
  let ctx = castStablePtrToPtr stablePtr

  let trampoline ctxPtr reqPtr curRespPtr = do
        fn      <- deRefStablePtr
                     (castPtrToStablePtr ctxPtr
                        :: StablePtr (Request -> Response -> IO Response))
        req     <- peekRequest reqPtr
        curResp <- readCResponse curRespPtr
        -- conduit-capi transferred ownership of curRespPtr to us; free it.
        conduit_response_free curRespPtr
        result  <- try (fn req curResp) :: IO (Either SomeException Response)
        case result of
          Right r -> pokeResponse r
          Left ex -> do
            hPutStrLn stderr ("[conduit] after-hook exception: " ++ show ex)
            withCString "internal server error" conduit_capi_report_error
            pokeResponse curResp  -- fall back to the original response

  funPtr <- mkAfter trampoline

  ctxFreePtr <- mkCtxFree $ \ctxPtr -> do
    freeHaskellFunPtr funPtr
    freeStablePtr
      (castPtrToStablePtr ctxPtr
         :: StablePtr (Request -> Response -> IO Response))

  return (funPtr, ctx, ctxFreePtr)

-- | Read a ConduitResponse* into a Haskell Response.  The C pointer is NOT
-- freed here; the caller must decide when to free it.
readCResponse :: Ptr CResponse -> IO Response
readCResponse ptr = do
  status  <- conduit_response_status ptr
  body    <- alloca $ \lenPtr -> do
               bPtr <- conduit_response_body ptr lenPtr
               len  <- peek lenPtr
               if bPtr == nullPtr || len == 0
                 then return BS.empty
                 else BS.packCStringLen (castPtr bPtr, fromIntegral len)
  nHdrs   <- (fromIntegral :: CSize -> Int) <$> conduit_response_header_count ptr
  headers <- mapM (readHeader ptr) [0 .. nHdrs - 1]
  return (Response status body headers)
  where
    readHeader p i = do
      nm  <- conduit_response_header_name  p (fromIntegral i)
      val <- conduit_response_header_value p (fromIntegral i)
      n   <- if nm  == nullPtr then return "" else peekCString nm
      v   <- if val == nullPtr then return "" else peekCString val
      return (T.pack n, T.pack v)

-- ── Application lifecycle ─────────────────────────────────────────────────────

-- | Create a new, empty Conduit application.
newApplication :: IO Application
newApplication = Application <$> conduit_app_new

-- | Register a route with an explicit HTTP method string.
addRoute :: Application -> Text -> Text -> (Request -> IO Response) -> IO ()
addRoute (Application appPtr) method pattern handler = do
  (funPtr, ctx, ctxFree) <- mkHandlerTriple handler
  withCString (T.unpack method)  $ \cm ->
    withCString (T.unpack pattern) $ \cp ->
      conduit_app_add_route appPtr cm cp funPtr ctx ctxFree

-- | Convenience wrappers for common HTTP methods.
get, post, put, delete, patch, options
  :: Application -> Text -> (Request -> IO Response) -> IO ()
get     app p h = addRoute app "GET"     p h
post    app p h = addRoute app "POST"    p h
put     app p h = addRoute app "PUT"     p h
delete  app p h = addRoute app "DELETE"  p h
patch   app p h = addRoute app "PATCH"   p h
options app p h = addRoute app "OPTIONS" p h

-- | Register a before-filter.  `Nothing` → continue; `Just resp` → short-circuit.
before :: Application -> (Request -> IO (Maybe Response)) -> IO ()
before (Application appPtr) handler = do
  (funPtr, ctx, ctxFree) <- mkBeforeTriple handler
  conduit_app_add_before appPtr funPtr ctx ctxFree

-- | Register an after-hook.  Receives the current response, returns (possibly
-- modified) response.
after :: Application -> (Request -> Response -> IO Response) -> IO ()
after (Application appPtr) handler = do
  (funPtr, ctx, ctxFree) <- mkAfterTriple handler
  conduit_app_add_after appPtr funPtr ctx ctxFree

-- | Override the 404 not-found handler.
notFound :: Application -> (Request -> IO Response) -> IO ()
notFound (Application appPtr) handler = do
  (funPtr, ctx, ctxFree) <- mkHandlerTriple handler
  conduit_app_set_not_found appPtr funPtr ctx ctxFree

-- | Override the 500 error handler.
onError :: Application -> (Request -> IO Response) -> IO ()
onError (Application appPtr) handler = do
  (funPtr, ctx, ctxFree) <- mkHandlerTriple handler
  conduit_app_set_error_handler appPtr funPtr ctx ctxFree

-- | Store a named key-value setting on the application.
setSetting :: Application -> Text -> Text -> IO ()
setSetting (Application appPtr) key value =
  withCString (T.unpack key)   $ \ck ->
    withCString (T.unpack value) $ \cv ->
      conduit_app_set_setting appPtr ck cv

-- | Retrieve a named setting.  Returns `Nothing` if not set.
getSetting :: Application -> Text -> IO (Maybe Text)
getSetting (Application appPtr) key =
  withCString (T.unpack key) $ \ck -> do
    cs <- conduit_app_get_setting appPtr ck
    if cs == nullPtr
      then return Nothing
      else do
        -- `finally` guarantees conduit_string_free even if peekCString throws,
        -- preventing a memory leak that could be exploited as a DoS vector.
        s <- peekCString cs `finally` conduit_string_free cs
        return (Just (T.pack s))

-- | Bind host:port, consuming the Application, and return a Server handle.
-- Throws an IOError if the bind fails (e.g. port already in use).
bind :: Application -> Text -> Word16 -> IO Server
bind (Application appPtr) host port =
  withCString (T.unpack host) $ \ch -> do
    srvPtr <- conduit_server_bind ch port appPtr
    if srvPtr == nullPtr
      then do
        errCs <- conduit_last_error
        err   <- if errCs == nullPtr then return "unknown error"
                 else peekCString errCs
        ioError (userError ("conduit_server_bind failed: " ++ err))
      else return (Server srvPtr)
