module LsmTreeSpec (spec) where

import Test.Hspec

import LsmTree

spec :: Spec
spec = do
    describe "put and get" $
        it "reads from the memtable and flushed segments" $ do
            let treeValue =
                    put "c" 3 $
                        put "b" 2 $
                            put "a" 1 (new 2)
            get "a" treeValue `shouldBe` Just 1
            get "b" treeValue `shouldBe` Just 2

    describe "delete, compact, and rangeQuery" $
        it "honors tombstones and compaction" $ do
            let treeValue0 =
                    put "c" 3 $
                        put "b" 2 $
                            put "a" 1 (new 2)
                treeValue1 = delete "b" treeValue0
                treeValue2 = compact (flush treeValue1)
            contains "b" treeValue2 `shouldBe` False
            rangeQuery "a" "z" treeValue2 `shouldBe` [("a",1),("c",3)]
