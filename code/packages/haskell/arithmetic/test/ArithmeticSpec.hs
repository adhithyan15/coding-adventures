module ArithmeticSpec (spec) where

import Test.Hspec
import Arithmetic

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
