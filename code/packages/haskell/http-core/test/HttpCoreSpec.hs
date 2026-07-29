module HttpCoreSpec (spec) where

import CodingAdventures.HttpCore
import Data.Either (isLeft)
import Test.Hspec

spec :: Spec
spec = do
  describe "package metadata and HTTP versions" $ do
    it "reports version 0.1.0" $
      version `shouldBe` "0.1.0"

    it "parses and renders bounded HTTP versions" $ do
      let expected = HttpVersion {versionMajor = 1, versionMinor = 1}
      parseHttpVersion "HTTP/1.1" `shouldBe` Right expected
      renderHttpVersion expected `shouldBe` "HTTP/1.1"

    it "rejects malformed and overflowing HTTP versions" $ do
      mapM_
        (\input -> parseHttpVersion input `shouldSatisfy` isLeft)
        [ "http/1.1"
        , "HTTP/1"
        , "HTTP/.1"
        , "HTTP/1."
        , "HTTP/a.1"
        , "HTTP/1.2.3"
        , "HTTP/65536.1"
        ]

  describe "ordered HTTP headers" $ do
    let headers =
          [ Header "Set-Cookie" "session=abc"
          , Header "set-cookie" "theme=dark"
          , Header "Content-Type" "text/plain"
          ]

    it "looks up names with ASCII case-insensitive comparison" $ do
      findHeader headers "CONTENT-TYPE" `shouldBe` Just "text/plain"
      findHeader headers "missing" `shouldBe` Nothing
      findHeader [Header "Ä-Test" "value"] "ä-test" `shouldBe` Nothing

    it "preserves duplicates and returns the first matching value" $ do
      map headerValue headers
        `shouldBe` ["session=abc", "theme=dark", "text/plain"]
      findHeader headers "set-cookie" `shouldBe` Just "session=abc"

  describe "semantic header helpers" $ do
    it "parses zero and positive content lengths" $ do
      parseContentLength [Header "Content-Length" "0"] `shouldBe` Just 0
      parseContentLength [Header "content-length" "42"] `shouldBe` Just 42

    it "rejects malformed, signed, padded, and overflowing lengths" $ do
      let overflow = show (toInteger (maxBound :: Int) + 1)
      mapM_
        (\value ->
          parseContentLength [Header "Content-Length" value]
            `shouldBe` Nothing)
        ["", "-1", "+1", " 1", "1 ", "forty-two", overflow]

    it "parses media types with and without charsets" $ do
      parseContentType [Header "Content-Type" "application/json"]
        `shouldBe` Just ("application/json", Nothing)
      parseContentType
        [Header "Content-Type" "text/html; level=1; CHARSET=\"utf-8\"; q=1"]
        `shouldBe` Just ("text/html", Just "utf-8")

    it "uses the first charset and rejects an empty media type" $ do
      parseContentType
        [Header "Content-Type" "text/plain; charset=ascii; charset=utf-8"]
        `shouldBe` Just ("text/plain", Just "ascii")
      parseContentType [Header "Content-Type" " ; charset=utf-8"]
        `shouldBe` Nothing
      parseContentType [] `shouldBe` Nothing

  describe "body framing and message heads" $ do
    it "compares all body framing constructors by value" $ do
      NoBody `shouldBe` NoBody
      ContentLength 7 `shouldBe` ContentLength 7
      UntilEof `shouldBe` UntilEof
      Chunked `shouldBe` Chunked

    it "delegates request helpers to the ordered header list" $ do
      let request =
            RequestHead
              { requestMethod = "POST"
              , requestTarget = "/submit?mode=fast"
              , requestVersion = HttpVersion 1 1
              , requestHeaders =
                  [ Header "Content-Length" "5"
                  , Header "Content-Type" "text/plain; charset=utf-8"
                  ]
              }
      requestHeader request "content-length" `shouldBe` Just "5"
      requestContentLength request `shouldBe` Just 5
      requestContentType request
        `shouldBe` Just ("text/plain", Just "utf-8")
      requestPath request `shouldBe` "/submit"
      requestQueryValue request "mode" `shouldBe` Just "fast"

    it "delegates response helpers to the ordered header list" $ do
      let response =
            ResponseHead
              { responseVersion = HttpVersion 1 0
              , responseStatus = 200
              , responseReason = "OK"
              , responseHeaders = [Header "Content-Type" "application/json"]
              }
      responseHeader response "content-type" `shouldBe` Just "application/json"
      responseContentLength response `shouldBe` Nothing
      responseContentType response
        `shouldBe` Just ("application/json", Nothing)

  describe "request targets and raw query strings" $ do
    it "splits path, query, and fragment without percent-decoding" $ do
      let target =
            parseRequestTarget
              "/clip/v2/resource?id=abc%20123&limit=10#ignored"
      targetPath target `shouldBe` "/clip/v2/resource"
      targetQuery target `shouldBe` Just "id=abc%20123&limit=10"
      targetFragment target `shouldBe` Just "ignored"
      queryValue target "id" `shouldBe` Just "abc%20123"

    it "normalizes an empty path while preserving empty suffixes" $ do
      parseRequestTarget "?flag#"
        `shouldBe` RequestTarget "/" (Just "flag") (Just "")
      parseRequestTarget ""
        `shouldBe` RequestTarget "/" Nothing Nothing

    it "splits on the first equals, skips empty pieces, and keeps duplicates" $ do
      let target = parseRequestTarget "/?&&flag&name=first=rest&name=second"
      queryPairs target
        `shouldBe`
          [ ("flag", "")
          , ("name", "first=rest")
          , ("name", "second")
          ]
      queryValue target "name" `shouldBe` Just "first=rest"
      queryValue target "missing" `shouldBe` Nothing

    it "splits paths consistently across repeated and trailing slashes" $ do
      splitPathSegments "/" `shouldBe` []
      splitPathSegments "/api//devices/" `shouldBe` ["api", "devices"]
      splitPathSegments "api/devices" `shouldBe` ["api", "devices"]

  describe "path-only route matching" $ do
    it "matches literals and returns ordered named captures" $ do
      let pattern = parseRoutePattern "/clip/:kind/:id"
      matchPath pattern "/clip/light/abc"
        `shouldBe` Just [("kind", "light"), ("id", "abc")]

    it "matches roots and rejects literal or arity mismatches" $ do
      matchPath (parseRoutePattern "/") "/" `shouldBe` Just []
      matchPath (parseRoutePattern "/hello/:name") "/goodbye/Ada"
        `shouldBe` Nothing
      matchPath (parseRoutePattern "/hello/:name") "/hello"
        `shouldBe` Nothing

    it "ignores query strings and fragments when matching a target" $ do
      matchTarget
        (parseRoutePattern "/devices/:id")
        "/devices/abc?verbose=true#client"
        `shouldBe` Just [("id", "abc")]
