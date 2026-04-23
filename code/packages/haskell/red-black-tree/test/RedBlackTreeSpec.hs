module RedBlackTreeSpec (spec) where

import Test.Hspec

import RedBlackTree

spec :: Spec
spec = do
    describe "insert" $
        it "maintains red-black invariants after insertion" $ do
            let treeValue = fromList [10,20,30,15,25]
            toList treeValue `shouldBe` [10,15,20,25,30]
            rootColor treeValue `shouldBe` Just Black
            isValid treeValue `shouldBe` True

    describe "delete" $
        it "removes values by rebuilding a valid tree" $ do
            let treeValue = delete 20 (fromList [10,20,30,15,25])
            member 20 treeValue `shouldBe` False
            isValid treeValue `shouldBe` True
