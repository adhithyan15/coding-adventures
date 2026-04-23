module TreeSpec (spec) where

import Test.Hspec

import Tree

spec :: Spec
spec = do
    describe "basic structure" $
        it "adds children and computes traversals" $ do
            let Right tree1 = addChild "Program" "Assignment" (new "Program")
                Right tree2 = addChild "Program" "Print" tree1
                Right tree3 = addChild "Assignment" "Name" tree2
                Right tree4 = addChild "Assignment" "BinaryOp" tree3
            children "Program" tree4 `shouldBe` ["Assignment", "Print"]
            preorder tree4 `shouldBe` ["Program", "Assignment", "Name", "BinaryOp", "Print"]
            leaves tree4 `shouldBe` ["Name", "BinaryOp", "Print"]
            lca "Name" "BinaryOp" tree4 `shouldBe` Just "Assignment"

    describe "subtree operations" $
        it "extracts and removes subtrees" $ do
            let Right tree1 = addChild "root" "left" (new "root")
                Right tree2 = addChild "root" "right" tree1
                Right tree3 = addChild "left" "left.left" tree2
            fmap preorder (subtree "left" tree3) `shouldBe` Just ["left", "left.left"]
            fmap preorder (removeSubtree "left" tree3) `shouldBe` Right ["root", "right"]
