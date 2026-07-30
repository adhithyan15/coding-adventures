module Http1Spec (spec) where

import CodingAdventures.Http1
import CodingAdventures.HttpCore hiding (version)
import qualified Data.ByteString.Char8 as Bytes
import Data.Either (isLeft)
import Test.Hspec

spec :: Spec
spec = do
  describe "package metadata" $
    it "reports version 0.1.0" $
      shouldBe version "0.1.0"

  describe "request heads" $ do
    it "parses a simple GET and reports the exact body offset" $ do
      let wire = "GET /devices HTTP/1.0\r\nHost: example.com\r\n\r\nbody"
          headText = "GET /devices HTTP/1.0\r\nHost: example.com\r\n\r\n"
      parsed <- expectRight (parseRequest wire)
      parsedRequest parsed
        `shouldBe`
          RequestHead
            { requestMethod = "GET"
            , requestTarget = "/devices"
            , requestVersion = HttpVersion 1 0
            , requestHeaders = [Header "Host" "example.com"]
            }
      requestBodyOffset parsed `shouldBe` Bytes.length (Bytes.pack headText)
      requestBodyKind parsed `shouldBe` NoBody

    it "skips leading blank lines without losing the final body offset" $ do
      let wire = "\r\n\nPOST /submit HTTP/1.1\nContent-Length: 5\n\nhello"
          headText = "\r\n\nPOST /submit HTTP/1.1\nContent-Length: 5\n\n"
      parsed <- expectRight (parseRequest wire)
      requestMethod (parsedRequest parsed) `shouldBe` "POST"
      requestBodyOffset parsed `shouldBe` Bytes.length (Bytes.pack headText)
      requestBodyKind parsed `shouldBe` ContentLength 5

    it "maps absent and zero Content-Length to no request body" $ do
      absent <- expectRight (parseRequest "GET / HTTP/1.1\r\n\r\n")
      zero <-
        expectRight
          (parseRequest "POST / HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
      requestBodyKind absent `shouldBe` NoBody
      requestBodyKind zero `shouldBe` NoBody

    it "gives chunked transfer encoding precedence over Content-Length" $ do
      parsed <-
        expectRight
          ( parseRequest
              "POST / HTTP/1.1\r\n\
              \Transfer-Encoding: gzip\r\n\
              \Transfer-Encoding: compress, CHUNKED\r\n\
              \Content-Length: invalid\r\n\r\n"
          )
      requestBodyKind parsed `shouldBe` Chunked

  describe "response heads" $ do
    it "parses status, reason, headers, length, and body offset" $ do
      let wire =
            "HTTP/1.1 200 Everything Is Fine\r\nContent-Length: 4\r\n\r\nbody"
          headText =
            "HTTP/1.1 200 Everything Is Fine\r\nContent-Length: 4\r\n\r\n"
      parsed <- expectRight (parseResponse wire)
      parsedResponse parsed
        `shouldBe`
          ResponseHead
            { responseVersion = HttpVersion 1 1
            , responseStatus = 200
            , responseReason = "Everything Is Fine"
            , responseHeaders = [Header "Content-Length" "4"]
            }
      responseBodyOffset parsed `shouldBe` Bytes.length (Bytes.pack headText)
      responseBodyKind parsed `shouldBe` ContentLength 4

    it "uses EOF framing when a body-bearing response omits length" $ do
      parsed <-
        expectRight
          (parseResponse "HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n")
      responseBodyKind parsed `shouldBe` UntilEof

    it "maps zero Content-Length to no response body" $ do
      parsed <-
        expectRight
          (parseResponse "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
      responseBodyKind parsed `shouldBe` NoBody

    it "makes 1xx, 204, and 304 bodyless before inspecting bad headers" $ do
      mapM_
        (\statusLine -> do
          parsed <-
            expectRight
              ( parseResponse
                  (statusLine ++ "\r\nContent-Length: invalid\r\n\r\n")
              )
          responseBodyKind parsed `shouldBe` NoBody
        )
        [ "HTTP/1.1 101 Switching Protocols"
        , "HTTP/1.1 204 No Content"
        , "HTTP/1.1 304 Not Modified"
        ]

    it "gives chunked transfer encoding precedence over Content-Length" $ do
      parsed <-
        expectRight
          ( parseResponse
              "HTTP/1.1 200 OK\r\n\
              \Transfer-Encoding: gzip, chunked\r\n\
              \Content-Length: invalid\r\n\r\n"
          )
      responseBodyKind parsed `shouldBe` Chunked

    it "accepts an empty reason phrase" $ do
      parsed <- expectRight (parseResponse "HTTP/1.1 200\r\n\r\n")
      responseReason (parsedResponse parsed) `shouldBe` ""

  describe "line and header preservation" $ do
    it "accepts LF, preserves duplicates, and trims only header OWS" $ do
      parsed <-
        expectRight
          ( parseResponse
              "\nHTTP/1.1 200 OK\n\
              \Set-Cookie:\t a=1 \t\n\
              \Set-Cookie: b=2\n\npayload"
          )
      responseHeaders (parsedResponse parsed)
        `shouldBe`
          [ Header "Set-Cookie" "a=1"
          , Header "Set-Cookie" "b=2"
          ]

    it "splits a header on only its first colon" $ do
      parsed <-
        expectRight
          (parseRequest "GET / HTTP/1.1\r\nX-Time: 12:34:56\r\n\r\n")
      requestHeaders (parsedRequest parsed)
        `shouldBe` [Header "X-Time" "12:34:56"]

    it "trims optional whitespace around a header name" $ do
      parsed <-
        expectRight
          (parseRequest "GET / HTTP/1.1\r\n  Host \t: example.com\r\n\r\n")
      requestHeaders (parsedRequest parsed)
        `shouldBe` [Header "Host" "example.com"]

  describe "stable parse failures" $ do
    it "rejects incomplete heads, including a final line without LF" $ do
      parseRequest "" `shouldBe` Left IncompleteHead
      parseRequest "GET / HTTP/1.1\r\nHost: example.com"
        `shouldBe` Left IncompleteHead
      parseResponse "HTTP/1.1 200 OK\r\n"
        `shouldBe` Left IncompleteHead

    it "rejects request start lines with missing or extra fields" $ do
      parseRequest "GET /\r\n\r\n"
        `shouldBe` Left (InvalidStartLine "GET /")
      parseRequest "GET / HTTP/1.1 extra\r\n\r\n"
        `shouldBe` Left (InvalidStartLine "GET / HTTP/1.1 extra")
      parseResponse "HTTP/1.1\r\n\r\n"
        `shouldBe` Left (InvalidStartLine "HTTP/1.1")

    it "distinguishes invalid request and response versions" $ do
      parseRequest "GET / HTTP/1\r\n\r\n"
        `shouldBe` Left (InvalidVersion "HTTP/1")
      parseResponse "HTTX/1.1 200 OK\r\n\r\n"
        `shouldBe` Left (InvalidVersion "HTTX/1.1")

    it "rejects malformed, signed, and overflowing status codes" $ do
      mapM_
        (\statusText ->
          parseResponse ("HTTP/1.1 " ++ statusText ++ " Bad\r\n\r\n")
            `shouldBe` Left (InvalidStatusCode statusText)
        )
        [ "-1"
        , "+200"
        , "two-hundred"
        , "65536"
        , replicate 1000 '9'
        ]

    it "rejects missing colons and empty header names" $ do
      parseRequest "GET / HTTP/1.1\r\nHost example.com\r\n\r\n"
        `shouldBe` Left (InvalidHeaderLine "Host example.com")
      parseRequest "GET / HTTP/1.1\r\n : value\r\n\r\n"
        `shouldBe` Left (InvalidHeaderLine " : value")

    it "rejects malformed, signed, and overflowing lengths" $ do
      let overflow = show (toInteger (maxBound :: Int) + 1)
      mapM_
        (\lengthText ->
          parseResponse
            ("HTTP/1.1 200 OK\r\nContent-Length: " ++ lengthText ++ "\r\n\r\n")
            `shouldBe` Left (InvalidContentLength lengthText)
        )
        [ ""
        , "-1"
        , "+1"
        , "1 0"
        , "one"
        , overflow
        , replicate 1000 '9'
        ]

    it "returns the first declared Content-Length error consistently" $ do
      parseRequest
        "POST / HTTP/1.1\r\n\
        \Content-Length: invalid\r\n\
        \Content-Length: 5\r\n\r\n"
        `shouldBe` Left (InvalidContentLength "invalid")

    it "keeps failure construction total for arbitrary bytes" $ do
      parseRequestHead (Bytes.pack "GET / HTTP/\255.1\r\n\r\n")
        `shouldSatisfy` isLeft

parseRequest :: String -> Either Http1ParseError ParsedRequestHead
parseRequest = parseRequestHead . Bytes.pack

parseResponse :: String -> Either Http1ParseError ParsedResponseHead
parseResponse = parseResponseHead . Bytes.pack

expectRight :: (HasCallStack, Show error) => Either error value -> IO value
expectRight result =
  case result of
    Left parseError -> do
      expectationFailure ("unexpected parse error: " ++ show parseError)
      fail "expectRight received Left"
    Right value -> pure value
