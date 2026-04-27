module SegmentTreeSpec (spec) where

import Data.Monoid (Sum(..))
import Test.Hspec

import SegmentTree

spec :: Spec
spec = do
    describe "rangeQuery" $
        it "aggregates ranges with the monoid instance" $ do
            let treeValue = fromList (map Sum [3,1,4,1,5])
            rangeQuery 1 3 treeValue `shouldBe` Just (Sum 6)

    describe "update" $
        it "replaces a single point" $ do
            let treeValue = fromList (map Sum [3,1,4,1,5])
            fmap (fmap getSum . rangeQuery 0 2) (update 1 (Sum 7) treeValue)
                `shouldBe` Just (Just 14)
