module JsonValueSpec (spec) where

import JsonValue
import Test.Hspec

spec :: Spec
spec = describe "JsonValue" $ do
    it "parses primitives and structures" $ do
        parseJson "{\"name\":\"Ada\",\"active\":true,\"scores\":[1,2.5],\"meta\":null}"
            `shouldBe` Right
                (JsonObject
                    [ ("name", JsonString "Ada")
                    , ("active", JsonBool True)
                    , ("scores", JsonArray [JsonNumber 1, JsonNumber 2.5])
                    , ("meta", JsonNull)
                    ]
                )

    it "parses escaped strings" $ do
        parseJson "\"line\\nfeed\""
            `shouldBe` Right (JsonString "line\nfeed")

    it "rejects invalid json" $ do
        parseJson "{bad"
            `shouldBe` Left "invalid json"
