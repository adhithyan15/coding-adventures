module HashSetSpec (spec) where

import HashSet
import Test.Hspec

spec :: Spec
spec = describe "HashSet" $ do
    it "adds, removes, and combines members" $ do
        let leftSet = add "beta" (add "alpha" empty)
            rightSet = add "gamma" (add "beta" empty)
        toList leftSet `shouldBe` ["alpha", "beta"]
        contains "alpha" leftSet `shouldBe` True
        toList (difference leftSet rightSet) `shouldBe` ["alpha"]
        toList (intersection leftSet rightSet) `shouldBe` ["beta"]
        toList (union leftSet rightSet) `shouldBe` ["alpha", "beta", "gamma"]
        size (remove "alpha" leftSet) `shouldBe` 1

    it "exposes a non-empty description" $ do
        description `shouldSatisfy` (not . null)
