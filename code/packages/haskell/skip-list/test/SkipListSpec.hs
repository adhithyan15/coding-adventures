module SkipListSpec (spec) where

import SkipList
import Test.Hspec

spec :: Spec
spec = describe "SkipList" $ do
    it "stores entries in sorted key order" $ do
        let valuesList = insert (2 :: Int) "beta" (insert 1 "alpha" new)
        entries valuesList `shouldBe` [(1, "alpha"), (2, "beta")]
        find 2 valuesList `shouldBe` Just "beta"
        member 1 valuesList `shouldBe` True
        keys (delete 1 valuesList) `shouldBe` [2]

    it "exposes a non-empty description" $ do
        description `shouldSatisfy` (not . null)
