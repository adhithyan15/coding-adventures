-- conduit-hello — a Haskell demo application for the Conduit web framework.
--
-- This program demonstrates the full Conduit DSL: route registration (GET, POST),
-- path parameters, query parameters, before-filters, after-hooks, custom
-- not-found and error handlers, redirects, and halt.
--
-- RUNNING
-- -------
-- Build conduit-capi first (the tools/run-tests.sh does this automatically):
--
--   cargo build -p conduit-capi --release
--   CONDUIT_CAPI_PATH=$(pwd)/target/release/libconduit_capi.dylib \
--     cabal run conduit-hello
--
-- The server listens on port 8080 unless PORT is set.
--
-- ROUTES
-- ------
--   GET  /                 → 200  "Hello, World from Haskell Conduit!"
--   GET  /hello/:name      → 200  "Hello, <name>!"
--   POST /echo             → 200  echoes the request body
--   GET  /search?q=<term>  → 200  "You searched for: <term>"
--   GET  /redirect         → 302  Location: /
--   GET  /halt             → 503  "Service halted"
--   GET  /down             → 503  (from before-filter, never reaches handler)
--   GET  /error            → 500  (handler throws, routed through on_error)
--   *    (anything else)   → 404  custom not-found handler

module Main (main) where

import           Control.Exception  (ioError)
import qualified Data.Text          as T
import           Data.Maybe         (fromMaybe)
import           System.Environment (lookupEnv)
import           System.IO          (hPutStrLn, stderr)

import           Conduit

main :: IO ()
main = do
  -- Read port from the environment; default to 8080.
  portStr <- fromMaybe "8080" <$> lookupEnv "PORT"
  let port = read portStr :: Int

  app <- newApplication

  -- Store the environment name as a setting (just to demonstrate setSetting).
  env <- fromMaybe "production" <$> lookupEnv "APP_ENV"
  setSetting app "environment" (T.pack env)

  -- ── Routes ─────────────────────────────────────────────────────────────────

  Conduit.get app "/" $ \_ ->
    return (html 200 "<h1>Hello, World from Haskell Conduit!</h1>")

  Conduit.get app "/hello/:name" $ \req -> do
    mname <- reqParam req "name"
    case mname of
      Nothing   -> return (html 400 "Missing :name parameter")
      Just name -> return (html 200 ("<p>Hello, " <> name <> "!</p>"))

  Conduit.post app "/echo" $ \req ->
    return (respond 200 (reqBody req))

  Conduit.get app "/search" $ \req -> do
    mq <- reqQuery req "q"
    let q = fromMaybe "(none)" mq
    return (textPlain 200 ("You searched for: " <> q))

  Conduit.get app "/redirect" $ \_ ->
    return (redirect 302 "/")

  Conduit.get app "/halt" $ \_ ->
    halt 503 "Service halted"

  -- This route is blocked by the before-filter below.
  Conduit.get app "/down" $ \_ ->
    return (html 200 "You should not see this — the before-filter intercepted /down")

  -- This handler throws, so on_error receives the request.
  Conduit.get app "/error" $ \_ ->
    ioError (userError "deliberate handler error")

  -- ── Filters ────────────────────────────────────────────────────────────────

  -- Before-filter: block /down with a 503.
  before app $ \req ->
    if reqPath req == "/down"
      then return (Just (html 503 "<p>Service temporarily unavailable.</p>"))
      else return Nothing

  -- After-hook: add an X-Powered-By header to every response.
  Conduit.after app $ \_ resp ->
    let newHeaders = ("X-Powered-By", "Haskell Conduit") : respHeaders resp
    in  return (resp { respHeaders = newHeaders })

  -- ── Error and not-found handlers ───────────────────────────────────────────

  notFound app $ \req ->
    return (html 404
      ("<p>Not found: <code>" <> reqPath req <> "</code></p>"))

  -- Note: reqError carries the message passed to conduit_capi_report_error,
  -- which the library normalises to "internal server error" — never a raw
  -- exception message — so reflecting it here is safe.
  onError app $ \_ ->
    return (html 500 "<p>Internal server error.</p>")

  -- ── Bind and serve ─────────────────────────────────────────────────────────

  srv <- bind app "0.0.0.0" (fromIntegral port)
  actual <- localPort srv
  hPutStrLn stderr ("Conduit (Haskell) listening on port " ++ show actual)
  serve srv
