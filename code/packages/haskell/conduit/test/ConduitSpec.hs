-- Unit tests for the Conduit library (no live server needed).
--
-- Tests cover:
--   - Response builder helpers: correct status, body, headers.
--   - redirect: CR/LF rejection.
--   - halt: throws ConduitHalt with the right response.
--   - Application: setSetting/getSetting round-trip.
--   - Application: newApplication is non-null (basic sanity).

module ConduitSpec (spec) where

import           Control.Exception      (evaluate, try)
import qualified Data.ByteString.Char8  as BC
import           Test.Hspec

import           Conduit

spec :: Spec
spec = do
  describe "Response.html" $ do
    it "sets status 200 and Content-Type text/html" $ do
      let r = html 200 "<p>hi</p>"
      respStatus r `shouldBe` 200
      lookup "Content-Type" (respHeaders r)
        `shouldBe` Just "text/html; charset=utf-8"
      respBody r `shouldBe` BC.pack "<p>hi</p>"

  describe "Response.json" $ do
    it "sets status 201 and Content-Type application/json" $ do
      let r = json 201 "{\"ok\":true}"
      respStatus r `shouldBe` 201
      lookup "Content-Type" (respHeaders r)
        `shouldBe` Just "application/json"

  describe "Response.textPlain" $ do
    it "sets status 200 and Content-Type text/plain" $ do
      let r = textPlain 200 "hello"
      respStatus r `shouldBe` 200
      lookup "Content-Type" (respHeaders r)
        `shouldBe` Just "text/plain; charset=utf-8"

  describe "Response.respond" $ do
    it "builds a bare response with no headers" $ do
      let r = respond 204 BC.empty
      respStatus r `shouldBe` 204
      respHeaders r `shouldBe` []
      respBody r `shouldBe` BC.empty

  describe "Response.redirect" $ do
    it "sets Location header" $ do
      let r = redirect 302 "/new-path"
      respStatus r `shouldBe` 302
      lookup "Location" (respHeaders r) `shouldBe` Just "/new-path"

    it "rejects a location containing CR" $ do
      evaluate (redirect 302 "/foo\r/bar") `shouldThrow` anyErrorCall

    it "rejects a location containing LF" $ do
      evaluate (redirect 302 "/foo\n/bar") `shouldThrow` anyErrorCall

  describe "Response.halt" $ do
    it "throws ConduitHalt with the given status" $ do
      result <- try (halt 503 "down") :: IO (Either ConduitHalt ())
      case result of
        Left (ConduitHalt r) -> respStatus r `shouldBe` 503
        Right _              -> expectationFailure "halt should have thrown"

  describe "Application.setSetting / getSetting" $ do
    it "stores and retrieves a value" $ do
      app <- newApplication
      setSetting app "env" "test"
      v   <- getSetting app "env"
      v `shouldBe` Just "test"

    it "returns Nothing for an unset key" $ do
      app <- newApplication
      v   <- getSetting app "no-such-key"
      v `shouldBe` Nothing

    it "last write wins for the same key" $ do
      app <- newApplication
      setSetting app "k" "first"
      setSetting app "k" "second"
      v <- getSetting app "k"
      v `shouldBe` Just "second"
