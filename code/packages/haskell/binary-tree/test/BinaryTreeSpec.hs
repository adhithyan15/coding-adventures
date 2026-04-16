module BinaryTreeSpec (spec) where

import Test.Hspec

import BinaryTree

spec :: Spec
spec = do
    describe "traversals" $
        it "walks a tree in all major orders" $ do
            let treeValue =
                    Node 1
                        (Node 2 (leaf 4) (leaf 5))
                        (Node 3 EmptyTree (leaf 6))
            inorder treeValue `shouldBe` [4,2,5,1,3,6]
            preorder treeValue `shouldBe` [1,2,4,5,3,6]
            postorder treeValue `shouldBe` [4,5,2,6,3,1]
            levelOrder treeValue `shouldBe` [1,2,3,4,5,6]

    describe "shape queries" $
        it "computes size, height, and leaves" $ do
            let treeValue = Node "root" (leaf "left") (Node "right" EmptyTree (leaf "right-leaf"))
            size treeValue `shouldBe` 4
            height treeValue `shouldBe` 3
            leaves treeValue `shouldBe` ["left", "right-leaf"]
