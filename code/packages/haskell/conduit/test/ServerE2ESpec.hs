-- End-to-end server tests.
--
-- Each test binds on port 0 (kernel assigns a free port), starts serving in the
-- background, sends a raw HTTP/1.0 request over a TCP socket, checks the
-- response, then stops and frees the server.
--
-- PORT 0 PATTERN
-- --------------
-- Binding on port 0 avoids port conflicts in CI and lets tests run in parallel.
-- After `bind`, call `localPort srv` to learn the assigned port.
--
-- RAW HTTP/1.0 CLIENT
-- --------------------
-- We intentionally use HTTP/1.0 (not http-conduit or similar) so the test has
-- zero dependency on a Haskell HTTP client library.  HTTP/1.0 connections close
-- after the response, so we can read until EOF.
--
-- TEARDOWN
-- --------
-- `bracket` (bind) (\srv -> stop srv >> freeServer srv) guarantees cleanup even
-- if a test assertion throws.

module ServerE2ESpec (spec) where

import           Control.Concurrent      (threadDelay)
import           Control.Exception       (bracket)
import qualified Data.ByteString.Char8   as BC
import           Data.Maybe              (fromMaybe)
import           Network.Socket          ( AddrInfo (..)
                                         , SocketType (..), ShutdownCmd (..)
                                         , defaultHints
                                         , getAddrInfo, socket, connect
                                         , shutdown, close )
import qualified Network.Socket.ByteString as NSB
import           Test.Hspec

import           Conduit hiding (before, after)
import qualified Conduit

-- ── Helpers ───────────────────────────────────────────────────────────────────

-- | Allocate a server, run the action, then stop + free.
withServer :: (Application -> IO ()) -> (Server -> IO a) -> IO a
withServer setup action = do
  app <- newApplication
  setup app
  bracket
    (bind app "127.0.0.1" 0)
    (\srv -> stop srv >> threadDelay 50000 >> freeServer srv)
    (\srv -> do
      serveBackground srv
      threadDelay 20000  -- give the server a moment to start accepting
      action srv)

-- | Send a raw HTTP/1.0 request string and return the raw response bytes.
rawHttp :: Int -> String -> IO BC.ByteString
rawHttp port reqStr = do
  let hints = defaultHints { addrSocketType = Stream }
  addr:_ <- getAddrInfo (Just hints) (Just "127.0.0.1") (Just (show port))
  sock   <- socket (addrFamily addr) Stream (addrProtocol addr)
  connect sock (addrAddress addr)
  NSB.sendAll sock (BC.pack reqStr)
  -- Shutdown the write half so the server sees EOF and closes the connection
  -- after sending its response. Without this, an HTTP/1.1 server keeps the
  -- connection open for keep-alive and recvAll blocks forever.
  shutdown sock ShutdownSend
  recvAll sock
  where
    recvAll s = do
      chunk <- NSB.recv s 4096
      if BC.null chunk
        then close s >> return BC.empty
        else (chunk <>) <$> recvAll s

-- | Extract the status line (first line) from a raw HTTP response.
statusLine :: BC.ByteString -> BC.ByteString
statusLine = fst . BC.break (== '\n') . fst . BC.break (== '\r')

-- ── Tests ─────────────────────────────────────────────────────────────────────

spec :: Spec
spec = describe "Server E2E" $ do

  it "GET / returns 200 Hello, Haskell!" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 200"
      resp `shouldSatisfy` BC.isInfixOf "Hello, Haskell!"

  it "GET /hello/world returns 200 with name" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /hello/world HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 200"
      resp `shouldSatisfy` BC.isInfixOf "Hello, world!"

  it "GET /hello/ (no name) returns 404" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /hello/ HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 404"

  it "POST /echo echoes the request body" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      let body = "hello echo"
          req  = "POST /echo HTTP/1.0\r\nHost: localhost\r\nContent-Length: "
                 ++ show (BC.length (BC.pack body))
                 ++ "\r\n\r\n" ++ body
      resp <- rawHttp port req
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 200"
      resp `shouldSatisfy` BC.isInfixOf (BC.pack body)

  it "GET /search?q=haskell returns 200 with query value" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port
        "GET /search?q=haskell HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 200"
      resp `shouldSatisfy` BC.isInfixOf "haskell"

  it "GET /redirect returns 302 with Location header" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /redirect HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 302"
      resp `shouldSatisfy` BC.isInfixOf "Location:"

  it "GET /halt returns 503" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /halt HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 503"

  it "GET /down returns 503 from before-filter" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /down HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 503"

  it "GET /error returns 500 from on_error handler" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /error HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 500"

  it "GET /missing returns 404 from custom notFound" $ do
    withServer setupApp $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /missing HTTP/1.0\r\nHost: localhost\r\n\r\n"
      statusLine resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 404"
      resp `shouldSatisfy` BC.isInfixOf "Not found"

-- ── Test application ──────────────────────────────────────────────────────────

-- | Build the test application with all 8 routes and filters.
setupApp :: Application -> IO ()
setupApp app = do
  -- Routes
  Conduit.get app "/" $ \_ ->
    return (html 200 "<h1>Hello, Haskell!</h1>")

  Conduit.get app "/hello/:name" $ \req -> do
    mname <- reqParam req "name"
    case mname of
      Nothing   -> return (html 404 "No name given")
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

  Conduit.get app "/down" $ \_ ->
    return (html 200 "This route is blocked by the before-filter")

  Conduit.get app "/error" $ \_ ->
    ioError (userError "deliberate error")

  -- Before-filter: block /down
  Conduit.before app $ \req ->
    if reqPath req == "/down"
      then return (Just (html 503 "Service temporarily unavailable"))
      else return Nothing

  -- Custom notFound and onError handlers
  notFound app $ \_ ->
    return (html 404 "Not found — Haskell Conduit")

  onError app $ \_ ->
    return (html 500 "Internal server error")
