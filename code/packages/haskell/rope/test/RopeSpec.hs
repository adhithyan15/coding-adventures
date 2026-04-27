module RopeSpec (spec) where

import Test.Hspec

import Rope

spec :: Spec
spec = do
    describe "append and concatRopes" $
        it "concatenates chunks while preserving text" $ do
            let ropeValue = concatRopes [fromString "hel", fromString "lo", fromString " world"]
            toString ropeValue `shouldBe` "hello world"
            len ropeValue `shouldBe` 11

    describe "splitRope and editing" $
        it "splits, inserts, and deletes ranges" $ do
            let ropeValue = fromString "hello world"
                (left, right) = splitRope 5 ropeValue
            toString left `shouldBe` "hello"
            toString right `shouldBe` " world"
            toString (insert 5 "," ropeValue) `shouldBe` "hello, world"
            toString (deleteRange 5 1 (insert 5 "," ropeValue)) `shouldBe` "hello world"
