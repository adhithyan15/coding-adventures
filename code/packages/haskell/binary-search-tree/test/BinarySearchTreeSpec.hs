module BinarySearchTreeSpec (spec) where

import Test.Hspec

import BinarySearchTree

spec :: Spec
spec = do
    describe "insert and member" $
        it "builds an ordered tree from a list" $ do
            let treeValue = fromList [5,1,3,9,7]
            member 3 treeValue `shouldBe` True
            member 4 treeValue `shouldBe` False
            toList treeValue `shouldBe` [1,3,5,7,9]

    describe "delete and rangeQuery" $
        it "deletes values while preserving ordering" $ do
            let treeValue = delete 5 (fromList [5,1,3,9,7])
            toList treeValue `shouldBe` [1,3,7,9]
            rangeQuery 2 8 treeValue `shouldBe` [3,7]
            predecessor 7 treeValue `shouldBe` Just 3
            successor 7 treeValue `shouldBe` Just 9
