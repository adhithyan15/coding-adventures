module LogicGatesSpec (spec) where

import Test.Hspec
import LogicGates

spec :: Spec
spec = do
    describe "Basic logic gates" $ do
        it "computes AND correctly" $ do
            andGate 1 1 `shouldBe` Right 1
            andGate 1 0 `shouldBe` Right 0
            
        it "computes OR correctly" $ do
            orGate 0 1 `shouldBe` Right 1
            orGate 0 0 `shouldBe` Right 0
            
    describe "Combinational Logic" $ do
        it "multiplexes correctly" $ do
            mux2 0 1 1 `shouldBe` Right 1
            mux2 1 0 0 `shouldBe` Right 1
            
        it "decodes correctly" $ do
            decoder [1, 0] `shouldBe` Right [0, 1, 0, 0]
            
    describe "Sequential Logic" $ do
        it "latches correctly" $ do
            let dl = dLatch 1 1 0 1
            fmap qOut dl `shouldBe` Right 1
            
        it "flipflops correctly" $ do
            let ff = dFlipFlop 1 1 0 1 1 0
            fmap ffQ ff `shouldBe` Right 1
