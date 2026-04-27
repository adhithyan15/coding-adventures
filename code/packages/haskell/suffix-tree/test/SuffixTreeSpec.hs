module SuffixTreeSpec (spec) where

import SuffixTree
import Test.Hspec

spec :: Spec
spec = describe "SuffixTree" $ do
    it "searches and counts matches" $ do
        let tree = build "banana"
        search tree "ana" `shouldBe` [1, 3]
        countOccurrences tree "ana" `shouldBe` 2
        nodeCount tree `shouldBe` 7

    it "returns every suffix" $ do
        let tree = build "banana"
        allSuffixes tree
            `shouldBe` ["banana", "anana", "nana", "ana", "na", "a"]

    it "finds longest repeated and common substrings" $ do
        let tree = buildUkkonen "banana"
        longestRepeatedSubstring tree `shouldBe` "ana"
        longestCommonSubstring "xabxac" "abcabxabcd" `shouldBe` "abxa"

    it "treats empty patterns as matching every boundary" $ do
        let tree = build "abc"
        search tree "" `shouldBe` [0, 1, 2, 3]
