module AvlTreeSpec (spec) where

import Test.Hspec

import AvlTree

spec :: Spec
spec = do
    describe "insert" $
        it "keeps the tree balanced while inserting values" $ do
            let treeValue = fromList [10,20,30,40,50,25]
            toList treeValue `shouldBe` [10,20,25,30,40,50]
            isBalanced treeValue `shouldBe` True

    describe "delete" $
        it "removes values by rebuilding a balanced tree" $ do
            let treeValue = delete 30 (fromList [10,20,30,40,50,25])
            member 30 treeValue `shouldBe` False
            isBalanced treeValue `shouldBe` True
