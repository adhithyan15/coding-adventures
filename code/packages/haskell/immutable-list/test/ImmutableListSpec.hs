module ImmutableListSpec (spec) where

import Test.Hspec

import ImmutableList

spec :: Spec
spec = do
    describe "persistent operations" $
        it "keeps previous versions intact" $ do
            let emptyList = empty
                one = push "hello" emptyList
                two = push "world" one
            len emptyList `shouldBe` 0
            toList one `shouldBe` ["hello"]
            toList two `shouldBe` ["hello", "world"]

    describe "get, set, and pop" $
        it "updates by returning new values" $ do
            let original = fromList ["a", "b", "c"]
            get 1 original `shouldBe` Just "b"
            fmap toList (set 1 "B" original) `shouldBe` Just ["a", "B", "c"]
            fmap (\(rest, value) -> (toList rest, value)) (pop original)
                `shouldBe` Just (["a", "b"], "c")
