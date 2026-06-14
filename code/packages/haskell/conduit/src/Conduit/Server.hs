-- Conduit.Server — server lifecycle: serve, serveBackground, stop, query.
--
-- A `Server` wraps a `Ptr CServer` (a Rust-owned box).  Once you call
-- `freeServer`, the pointer is invalid; further calls are undefined behaviour.
--
-- FOREGROUND vs BACKGROUND
-- ------------------------
-- `serve` calls `conduit_server_serve`, which is tagged `safe` in the FFI
-- declaration.  The `safe` annotation releases the GHC capability while C
-- blocks, allowing other Haskell green threads to run.  Without this, a single
-- `serve` call would freeze the entire Haskell program.
--
-- `serveBackground` calls `conduit_server_serve_background`, which starts a new
-- OS thread inside the Rust runtime and returns immediately.  The background OS
-- thread does NOT hold any GHC capability — it only calls back into Haskell via
-- the FunPtr trampolines, which acquire a capability on entry (that is what
-- "wrapper" imports guarantee).
--
-- STOP AND POLL
-- -------------
-- `stop` sends a shutdown signal; `running` polls whether the reactor has
-- actually exited.  If you need to wait until fully stopped, loop on `running`.

module Conduit.Server
  ( Server (..)
  , serve
  , serveBackground
  , stop
  , localPort
  , running
  , freeServer
  ) where

import           Conduit.FFI
import           Foreign.Ptr  (Ptr)

-- ── Server type ───────────────────────────────────────────────────────────────

-- | A handle to a bound Conduit server.
newtype Server = Server (Ptr CServer)

-- ── Server operations ─────────────────────────────────────────────────────────

-- | Start serving on the calling thread.  Blocks until the server is stopped.
-- Other Haskell green threads will continue to run while this blocks (the
-- FFI call uses the `safe` calling convention).
serve :: Server -> IO ()
serve (Server ptr) = do
  rc <- conduit_server_serve ptr
  if rc /= 0
    then ioError (userError "conduit_server_serve returned non-zero")
    else return ()

-- | Start serving on a Rust-managed background OS thread.  Returns immediately.
serveBackground :: Server -> IO ()
serveBackground (Server ptr) = do
  rc <- conduit_server_serve_background ptr
  if rc /= 0
    then ioError (userError "conduit_server_serve_background returned non-zero")
    else return ()

-- | Send a shutdown signal to the server.
-- This returns before the reactor has fully exited; poll `running` if needed.
stop :: Server -> IO ()
stop (Server ptr) = conduit_server_stop ptr

-- | Return the port the server is actually listening on.
-- Useful when you bound port 0 (kernel-assigned ephemeral port).
localPort :: Server -> IO Int
localPort (Server ptr) = fromIntegral <$> conduit_server_local_port ptr

-- | Return `True` if the server is currently running.
running :: Server -> IO Bool
running (Server ptr) = (/= 0) <$> conduit_server_running ptr

-- | Free the server handle and all associated resources.
-- Must be called after `stop` (and after `running` returns False).
--
-- WARNING: the `Server` value is invalid after this call.  Calling `freeServer`
-- twice on the same value is a use-after-free in the Rust layer and is undefined
-- behaviour.  Use `bracket` (bind ...) (\srv -> stop srv >> freeServer srv)
-- to ensure exactly-once cleanup even in the presence of exceptions.
freeServer :: Server -> IO ()
freeServer (Server ptr) = conduit_server_free ptr
