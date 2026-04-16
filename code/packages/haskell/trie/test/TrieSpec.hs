module TrieSpec (spec) where

import Test.Hspec

import Trie

spec :: Spec
spec = do
    describe "insert and lookupValue" $
        it "stores exact keys" $ do
            let trieValue =
                    insert "cat" 1 $
                        insert "car" 2 $
                            insert "dog" 3 empty
            lookupValue "cat" trieValue `shouldBe` Just 1
            lookupValue "car" trieValue `shouldBe` Just 2
            lookupValue "cow" trieValue `shouldBe` Nothing

    describe "prefix operations" $
        it "supports autocomplete and longest-prefix matching" $ do
            let trieValue =
                    insert "hello" "word" $
                        insert "hell" "prefix" $
                            insert "helium" "element" empty
            startsWith "hel" trieValue `shouldBe` True
            map fst (wordsWithPrefix "hel" trieValue) `shouldBe` ["helium", "hell", "hello"]
            longestPrefixMatch "hello!" trieValue `shouldBe` Just ("hello", "word")
