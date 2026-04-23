module JsonSerializerSpec (spec) where

import JsonSerializer
import JsonValue
import Test.Hspec

spec :: Spec
spec = describe "JsonSerializer" $ do
    it "renders compact json" $ do
        renderJson (JsonObject [("name", JsonString "Ada"), ("active", JsonBool True)])
            `shouldBe` "{\"name\":\"Ada\",\"active\":true}"

    it "renders pretty json with sorting" $ do
        renderPrettyJson
            (SerializerConfig 2 ' ' True False)
            (JsonObject [("z", JsonNumber 1), ("a", JsonArray [JsonBool False])])
            `shouldBe` "{\n  \"a\": [\n    false\n  ],\n  \"z\": 1\n}"

    it "escapes control characters" $ do
        renderJson (JsonString "line\nfeed")
            `shouldBe` "\"line\\nfeed\""
