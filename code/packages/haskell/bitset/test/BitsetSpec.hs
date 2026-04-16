module BitsetSpec (spec) where

import Test.Hspec

import Bitset

spec :: Spec
spec = do
    describe "fromIntegerValue" $
        it "converts integers to binary strings" $
            toBinaryString (fromIntegerValue 42) `shouldBe` "101010"

    describe "bit operations" $ do
        it "sets, clears, toggles, and tests bits" $ do
            let bitset0 = new 8
                bitset1 = setBit 2 bitset0
                bitset2 = toggleBit 5 bitset1
                bitset3 = clearBit 2 bitset2
            testBit 2 bitset1 `shouldBe` True
            testBit 5 bitset2 `shouldBe` True
            testBit 2 bitset3 `shouldBe` False

        it "supports bulk boolean operations" $ do
            let left = fromIntegerValue 12
                right = fromIntegerValue 10
            toIntegerValue (andBitset left right) `shouldBe` 8
            toIntegerValue (orBitset left right) `shouldBe` 14
            toIntegerValue (xorBitset left right) `shouldBe` 6
            toIntegerValue (andNotBitset left right) `shouldBe` 4

    describe "queries" $ do
        it "counts and iterates set bits" $ do
            let bitset = fromIntegerValue 165
            popCount bitset `shouldBe` 4
            iterSetBits bitset `shouldBe` [0,2,5,7]

        it "tracks length-aware complements" $ do
            let bitset = fromBinaryString "1010"
            fmap toBinaryString (notBitset <$> bitset) `shouldBe` Right "0101"
