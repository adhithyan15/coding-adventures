module TreeSetSpec (spec) where

import Test.Hspec

import TreeSet

spec :: Spec
spec = do
    describe "fromList and contains" $
        it "deduplicates and sorts values" $ do
            let setValue = fromList [5,1,3,3,9]
            toSortedList setValue `shouldBe` [1,3,5,9]
            contains 3 setValue `shouldBe` True

    describe "set algebra and order queries" $
        it "supports union, intersection, range, rank, predecessor, and successor" $ do
            let left = fromList [1,3,5,9]
                right = fromList [3,4,5,8]
            toSortedList (union left right) `shouldBe` [1,3,4,5,8,9]
            toSortedList (intersection left right) `shouldBe` [3,5]
            range 3 8 left `shouldBe` [3,5]
            rank 5 left `shouldBe` 2
            predecessor 5 left `shouldBe` Just 3
            successor 5 left `shouldBe` Just 9
