module BPlusTreeSpec (spec) where

import Test.Hspec

import BPlusTree

spec :: Spec
spec = do
    describe "insert and search" $
        it "supports point lookups and ordered full scans" $ do
            let treeValue =
                    insert 15 "fifteen" $
                        insert 20 "twenty" $
                            insert 5 "five" (new 3)
            search 15 treeValue `shouldBe` Just "fifteen"
            fmap fst (fullScan treeValue) `shouldBe` [5,15,20]

    describe "rangeScan and delete" $
        it "supports range scans and removals" $ do
            let treeValue =
                    delete 15 $
                        insert 15 "fifteen" $
                            insert 20 "twenty" $
                                insert 5 "five" (new 3)
            contains 15 treeValue `shouldBe` False
            rangeScan 5 20 treeValue `shouldBe` [(5,"five"),(20,"twenty")]
            isValid treeValue `shouldBe` True
