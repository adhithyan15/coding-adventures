module ArithmeticSpec (spec) where

import Test.Hspec
import Arithmetic
import LogicGates

spec :: Spec
spec = do
    describe "Addition Setup" $ do
        it "computes halfAdder correctly" $ do
            halfAdder 1 1 `shouldBe` Right (0, 1)
            halfAdder 1 0 `shouldBe` Right (1, 0)
        it "computes fullAdder correctly" $ do
            fullAdder 1 1 1 `shouldBe` Right (1, 1)
            fullAdder 0 1 1 `shouldBe` Right (0, 1)
        it "computes rippleCarryAdder correctly" $ do
            rippleCarryAdder [0, 0, 1, 1] [0, 0, 0, 1] 0 `shouldBe` Right ([0, 1, 0, 0], 0)
            
    describe "Subtraction" $ do
        it "computes subtractorN correctly" $ do
            subtractorN [0, 1, 0, 0] [0, 0, 0, 1] `shouldBe` Right ([0, 0, 1, 1], 0)

    describe "ALU Control" $ do
        it "processes ALU logic properly" $ do
            let res = alu [0,1,0,1] [0,0,1,1] ADD
            fmap result res `shouldBe` Right [1,0,0,0]
            fmap (zFlag . flags) res `shouldBe` Right 0

    describe "Hardening (LGE-HS-DEPS)" $ do
        it "rejects empty ALU inputs for every op" $ do
            mapM_ (\op -> alu [] [] op `shouldBe` Left (InvalidShape "Data inputs must not be empty"))
                  [ADD, SUB, AND, OR, XOR, SLT]
        it "rejects invalid ALU bits and mismatched lengths" $ do
            alu [2] [0] ADD `shouldBe` Left (InvalidBit "a" 2)
            alu [0] [3] ADD `shouldBe` Left (InvalidBit "b" 3)
            alu [0,1] [0] ADD `shouldBe` Left (InvalidShape "Data inputs must have same length")
        it "SLT still works" $ do
            fmap result (alu [1,1,1,1] [0,0,0,1] SLT) `shouldBe` Right [0,0,0,1]
        it "validates carryIn even with empty operands" $ do
            rippleCarryAdder [] [] 5 `shouldBe` Left (InvalidBit "carryIn" 5)
            rippleCarryAdder [] [] 1 `shouldBe` Right ([], 1)
        it "rejects mismatched adder lengths" $
            rippleCarryAdder [1] [1,0] 0 `shouldBe` Left (InvalidShape "Inputs must have the same length")
        it "isNegative validates all bits and rejects empty" $ do
            isNegative [1,7] `shouldBe` Left (InvalidBit "bits" 7)
            isNegative [0,1] `shouldBe` Right 0
            isNegative [] `shouldBe` Left (InvalidShape "bits must not be empty")
