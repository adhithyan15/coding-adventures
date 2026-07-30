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

    it "uses chunked framing when chunked is the final transfer coding" $ do
      parsed <-
        expectRight
          ( parseRequest
              "POST / HTTP/1.1\r\n\
              \Transfer-Encoding: gzip\r\n\
              \Transfer-Encoding: compress, CHUNKED\r\n\r\n"
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

    it "makes 1xx, 204, and 304 responses bodyless" $ do
      mapM_
        (\statusLine -> do
          parsed <-
            expectRight
              ( parseResponse
                  (statusLine ++ "\r\nContent-Length: 12\r\n\r\n")
              )
          responseBodyKind parsed `shouldBe` NoBody
          responseSwitchesProtocol parsed `shouldBe` False
        )
        [ "HTTP/1.1 101 Switching Protocols"
        , "HTTP/1.1 204 No Content"
        , "HTTP/1.1 304 Not Modified"
        ]

    it "uses chunked framing when chunked is the final response coding" $ do
      parsed <-
        expectRight
          ( parseResponse
              "HTTP/1.1 200 OK\r\n\
              \Transfer-Encoding: gzip, chunked\r\n\r\n"
          )
      responseBodyKind parsed `shouldBe` Chunked

    it "uses request context for HEAD and successful CONNECT responses" $ do
      headResponse <-
        expectRight
          (parseResponseFor "HEAD" "HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n")
      connectResponse <-
        expectRight
          (parseResponseFor "CONNECT" "HTTP/1.1 200 Established\r\n\r\ntunnel")
      responseBodyKind headResponse `shouldBe` NoBody
      responseSwitchesProtocol headResponse `shouldBe` False
      responseBodyKind connectResponse `shouldBe` NoBody
      responseSwitchesProtocol connectResponse `shouldBe` True

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

    it "rejects whitespace before a field colon and obsolete folding" $ do
      parseRequest "GET / HTTP/1.1\r\nHost \t: example.com\r\n\r\n"
        `shouldBe` Left InvalidHeaderLine
      parseRequest "GET / HTTP/1.1\r\nHost: example.com\r\n continued\r\n\r\n"
        `shouldBe` Left InvalidHeaderLine

  describe "stable parse failures" $ do
    it "rejects incomplete heads, including a final line without LF" $ do
      parseRequest "" `shouldBe` Left IncompleteHead
      parseRequest "GET / HTTP/1.1\r\nHost: example.com"
        `shouldBe` Left IncompleteHead
      parseResponse "HTTP/1.1 200 OK\r\n"
        `shouldBe` Left IncompleteHead

    it "rejects request start lines with missing or extra fields" $ do
      parseRequest "GET /\r\n\r\n"
        `shouldBe` Left InvalidStartLine
      parseRequest "GET / HTTP/1.1 extra\r\n\r\n"
        `shouldBe` Left InvalidStartLine
      parseRequest "GET\t/ HTTP/1.1\r\n\r\n"
        `shouldBe` Left InvalidStartLine
      parseRequest "GET  / HTTP/1.1\r\n\r\n"
        `shouldBe` Left InvalidStartLine
      parseResponse "HTTP/1.1\r\n\r\n"
        `shouldBe` Left InvalidStartLine

    it "distinguishes invalid request and response versions" $ do
      parseRequest "GET / HTTP/1\r\n\r\n"
        `shouldBe` Left InvalidVersion
      parseResponse "HTTX/1.1 200 OK\r\n\r\n"
        `shouldBe` Left InvalidVersion

    it "rejects non-three-digit, signed, and overflowing status codes" $ do
      mapM_
        (\statusText ->
          parseResponse ("HTTP/1.1 " ++ statusText ++ " Bad\r\n\r\n")
            `shouldBe` Left InvalidStatusCode
        )
        [ "99"
        , "1000"
        , "-1"
        , "+200"
        , "two-hundred"
        , "65536"
        , replicate 1000 '9'
        ]

    it "rejects missing colons and empty header names" $ do
      parseRequest "GET / HTTP/1.1\r\nHost example.com\r\n\r\n"
        `shouldBe` Left InvalidHeaderLine
      parseRequest "GET / HTTP/1.1\r\n : value\r\n\r\n"
        `shouldBe` Left InvalidHeaderLine
      parseRequest "GET / HTTP/1.1\r\nBad(Name): value\r\n\r\n"
        `shouldBe` Left InvalidHeaderLine
      parseRequest "GET / HTTP/1.1\r\nX-Test: value\NULsuffix\r\n\r\n"
        `shouldBe` Left InvalidHeaderLine

    it "rejects malformed, signed, and overflowing lengths" $ do
      let overflow = show (toInteger (maxBound :: Int) + 1)
      mapM_
        (\lengthText ->
          parseResponse
            ("HTTP/1.1 200 OK\r\nContent-Length: " ++ lengthText ++ "\r\n\r\n")
            `shouldBe` Left InvalidContentLength
        )
        [ ""
        , "-1"
        , "+1"
        , "1 0"
        , "one"
        , overflow
        , replicate 1000 '9'
        ]

    it "accepts identical coalesced lengths and rejects conflicting duplicates" $ do
      parsed <-
        expectRight
          ( parseRequest
              "POST / HTTP/1.1\r\n\
              \Content-Length: 5, 5\r\n\
              \Content-Length: 5\r\n\r\nhello"
          )
      requestBodyKind parsed `shouldBe` ContentLength 5
      parseRequest
        "POST / HTTP/1.1\r\n\
        \Content-Length: 4\r\n\
        \Content-Length: 5\r\n\r\n"
        `shouldBe` Left InvalidContentLength

    it "rejects ambiguous and unsafe transfer framing" $ do
      parseRequest
        "POST / HTTP/1.1\r\n\
        \Transfer-Encoding: chunked\r\n\
        \Content-Length: 5\r\n\r\n"
        `shouldBe` Left AmbiguousFraming
      parseRequest
        "POST / HTTP/1.1\r\nTransfer-Encoding: chunked, gzip\r\n\r\n"
        `shouldBe` Left InvalidTransferEncoding
      parseRequest
        "POST / HTTP/1.1\r\nTransfer-Encoding: chunked, chunked\r\n\r\n"
        `shouldBe` Left InvalidTransferEncoding
      parseRequest
        "POST / HTTP/1.0\r\nTransfer-Encoding: chunked\r\n\r\n"
        `shouldBe` Left InvalidTransferEncoding
      parseResponse
        "HTTP/1.1 200 OK\r\n\
        \Transfer-Encoding: chunked\r\n\
        \Content-Length: 5\r\n\r\n"
        `shouldBe` Left AmbiguousFraming

    it "uses EOF framing for a response with a non-chunked transfer coding" $ do
      parsed <-
        expectRight
          (parseResponse "HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\n")
      responseBodyKind parsed `shouldBe` UntilEof

    it "enforces head, line, header-count, and transfer-coding limits" $ do
      let longLine = "GET /" ++ replicate 8192 'a' ++ " HTTP/1.1\r\n\r\n"
          manyHeaders =
            "GET / HTTP/1.1\r\n"
              ++ concat (replicate 101 "X-Test: value\r\n")
              ++ "\r\n"
          manyCodings =
            "POST / HTTP/1.1\r\nTransfer-Encoding: "
              ++ concat (replicate 16 "gzip,")
              ++ "chunked\r\n\r\n"
          oversizedIncomplete = concat (replicate 32769 "\r\n")
      parseRequest longLine `shouldBe` Left LineTooLong
      parseRequest manyHeaders `shouldBe` Left TooManyHeaders
      parseRequest manyCodings `shouldBe` Left TooManyTransferCodings
      parseRequest oversizedIncomplete `shouldBe` Left HeadTooLarge

    it "never retains raw request targets or field values in errors" $ do
      let targetSecret =
            show (parseRequest "GET /pair?token=secret HTTP/1.1 extra\r\n\r\n")
          headerSecret =
            show
              ( parseRequest
                  "GET / HTTP/1.1\r\n\
                  \Authorization secret-value\r\n\r\n"
              )
      targetSecret `shouldNotContain` "secret"
      headerSecret `shouldNotContain` "secret-value"

    it "keeps failure construction total for arbitrary bytes" $ do
      parseRequestHead (Bytes.pack "GET / HTTP/\255.1\r\n\r\n")
        `shouldSatisfy` isLeft

parseRequest :: String -> Either Http1ParseError ParsedRequestHead
parseRequest = parseRequestHead . Bytes.pack

parseResponse :: String -> Either Http1ParseError ParsedResponseHead
parseResponse = parseResponseFor "GET"

parseResponseFor :: String -> String -> Either Http1ParseError ParsedResponseHead
parseResponseFor method = parseResponseHead method . Bytes.pack

expectRight :: (HasCallStack, Show error) => Either error value -> IO value
expectRight result =
  case result of
    Left parseError -> do
      expectationFailure ("unexpected parse error: " ++ show parseError)
      fail "expectRight received Left"
    Right value -> pure value
