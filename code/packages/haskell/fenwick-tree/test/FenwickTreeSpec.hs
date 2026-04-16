module FenwickTreeSpec (spec) where

import Test.Hspec

import FenwickTree

spec :: Spec
spec = do
    describe "prefixSum and rangeSum" $
        it "computes prefix and range totals" $ do
            let treeState = fromList [3, 2, 1, 7, 4]
            prefixSum 3 treeState `shouldBe` Right 6
            rangeSum 2 4 treeState `shouldBe` Right 10

    describe "update and pointQuery" $
        it "applies point updates" $ do
            let treeState = fromList [3, 2, 1, 7, 4]
            updated <- pure (update 3 5 treeState)
            case updated of
                Left err -> expectationFailure (show err)
                Right treeState' -> do
                    pointQuery 3 treeState' `shouldBe` Right 6
                    findKth 11 treeState' `shouldBe` Right 3
