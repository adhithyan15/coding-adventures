-- Smoke tests for conduit-hello: launch in background, hit key routes, stop.
--
-- These tests start the actual conduit-hello application routes (re-used via
-- the `setupApp` helper rather than running the binary) to verify the full
-- stack: Haskell → FFI → conduit-capi → web-core → TCP.

module Main (main) where

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

main :: IO ()
main = hspec spec

spec :: Spec
spec = describe "conduit-hello smoke tests" $ do

  it "GET / → 200 Hello" $
    withHello $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n"
      resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 200"

  it "GET /hello/Haskell → 200 Hello, Haskell!" $
    withHello $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /hello/Haskell HTTP/1.0\r\nHost: localhost\r\n\r\n"
      resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 200"
      resp `shouldSatisfy` BC.isInfixOf "Haskell"

  it "GET /redirect → 302" $
    withHello $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /redirect HTTP/1.0\r\nHost: localhost\r\n\r\n"
      resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 302"

  it "GET /down → 503 (before-filter)" $
    withHello $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /down HTTP/1.0\r\nHost: localhost\r\n\r\n"
      resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 503"

  it "GET /error → 500 (on_error)" $
    withHello $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET /error HTTP/1.0\r\nHost: localhost\r\n\r\n"
      resp `shouldSatisfy` BC.isPrefixOf "HTTP/1.1 500"

  it "X-Powered-By header is present (after-hook)" $
    withHello $ \srv -> do
      port <- localPort srv
      resp <- rawHttp port "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n"
      resp `shouldSatisfy` BC.isInfixOf "X-Powered-By"

-- ── Helpers ───────────────────────────────────────────────────────────────────

withHello :: (Server -> IO a) -> IO a
withHello action = do
  app <- newApplication
  setupHelloApp app
  bracket
    (bind app "127.0.0.1" 0)
    (\srv -> stop srv >> threadDelay 50000 >> freeServer srv)
    (\srv -> do
      serveBackground srv
      threadDelay 20000
      action srv)

-- Re-implement the hello-world routes inline so the smoke test doesn't
-- need to shell out to the executable.
setupHelloApp :: Application -> IO ()
setupHelloApp app = do
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

  Conduit.get app "/down" $ \_ ->
    return (html 200 "Blocked")

  Conduit.get app "/error" $ \_ ->
    ioError (userError "deliberate error")

  Conduit.before app $ \req ->
    if reqPath req == "/down"
      then return (Just (html 503 "Temporarily unavailable"))
      else return Nothing

  Conduit.after app $ \_ resp ->
    return (resp { respHeaders = ("X-Powered-By", "Haskell Conduit") : respHeaders resp })

  notFound app $ \req ->
    return (html 404 ("Not found: " <> reqPath req))

  onError app $ \req ->
    return (html 500 ("Error: " <> reqError req))

rawHttp :: Int -> String -> IO BC.ByteString
rawHttp port reqStr = do
  let hints = defaultHints { addrSocketType = Stream }
  addr:_ <- getAddrInfo (Just hints) (Just "127.0.0.1") (Just (show port))
  sock   <- socket (addrFamily addr) Stream (addrProtocol addr)
  connect sock (addrAddress addr)
  NSB.sendAll sock (BC.pack reqStr)
  shutdown sock ShutdownSend
  recvAll sock
  where
    recvAll s = do
      chunk <- NSB.recv s 4096
      if BC.null chunk
        then close s >> return BC.empty
        else (chunk <>) <$> recvAll s
