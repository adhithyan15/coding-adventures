module HeapSpec (spec) where

import Heap
import Test.Hspec

spec :: Spec
spec = describe "Heap" $ do
    it "keeps values in ascending order" $ do
        let valuesHeap = fromList [3 :: Int, 1, 2]
        peek valuesHeap `shouldBe` Just 1
        fmap fst (minView valuesHeap) `shouldBe` Just 1
        toAscList (pop valuesHeap) `shouldBe` [2, 3]

    it "exposes a non-empty description" $ do
        description `shouldSatisfy` (not . null)
