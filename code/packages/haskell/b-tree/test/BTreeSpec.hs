module BTreeSpec (spec) where

import Test.Hspec

import BTree

spec :: Spec
spec = do
    describe "insert and search" $
        it "stores key-value pairs in sorted order" $ do
            let treeValue =
                    insert 15 "fifteen" $
                        insert 20 "twenty" $
                            insert 5 "five" (new 3)
            search 15 treeValue `shouldBe` Just "fifteen"
            fmap fst (inorder treeValue) `shouldBe` [5,15,20]

    describe "delete and rangeQuery" $
        it "supports point deletion and bounded scans" $ do
            let treeValue =
                    delete 15 $
                        insert 15 "fifteen" $
                            insert 20 "twenty" $
                                insert 5 "five" (new 3)
            contains 15 treeValue `shouldBe` False
            rangeQuery 5 20 treeValue `shouldBe` [(5,"five"),(20,"twenty")]
            isValid treeValue `shouldBe` True
