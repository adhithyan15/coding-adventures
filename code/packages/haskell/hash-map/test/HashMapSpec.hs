module HashMapSpec (spec) where

import HashMap
import Test.Hspec

spec :: Spec
spec = describe "HashMap" $ do
    it "stores, updates, and deletes entries" $ do
        let valuesMap = set "beta" (2 :: Int) (set "alpha" 1 empty)
        get "alpha" valuesMap `shouldBe` Just 1
        has "beta" valuesMap `shouldBe` True
        size valuesMap `shouldBe` 2
        keys valuesMap `shouldBe` ["alpha", "beta"]
        entries (delete "alpha" valuesMap) `shouldBe` [("beta", 2)]

    it "builds from ascending entries" $ do
        toList (fromList [("b", 2 :: Int), ("a", 1)]) `shouldBe` [("a", 1), ("b", 2)]

    it "exposes a non-empty description" $ do
        description `shouldSatisfy` (not . null)
