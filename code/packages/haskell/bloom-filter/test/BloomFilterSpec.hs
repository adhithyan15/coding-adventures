module BloomFilterSpec (spec) where

import Test.Hspec

import qualified Bitset
import BloomFilter

spec :: Spec
spec = do
    describe "insert and mightContain" $
        it "marks inserted values as present" $ do
            let filterState = fromList 64 3 ["alpha", "beta", "gamma"]
            mightContain "alpha" filterState `shouldBe` True
            mightContain "beta" filterState `shouldBe` True

    describe "bitset" $
        it "touches multiple underlying bits" $ do
            let filterState = insert "delta" (new 64 4)
            Bitset.popCount (bitset filterState) `shouldSatisfy` (> 1)
