module TreapSpec (spec) where

import Test.Hspec

import Treap

spec :: Spec
spec = do
    describe "insert" $
        it "maintains BST and heap ordering" $ do
            let treeValue = fromList [(5, 50), (3, 30), (7, 70), (1, 10)]
            toList treeValue `shouldBe` [1,3,5,7]
            rootPriority treeValue `shouldBe` Just 10
            isValid treeValue `shouldBe` True

    describe "delete" $
        it "removes keys while preserving treap invariants" $ do
            let treeValue = delete 3 (fromList [(5, 50), (3, 30), (7, 70), (1, 10)])
            member 3 treeValue `shouldBe` False
            isValid treeValue `shouldBe` True
