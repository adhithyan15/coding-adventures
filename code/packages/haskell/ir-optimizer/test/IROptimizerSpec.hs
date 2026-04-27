module IROptimizerSpec (spec) where

import CompilerIR
import IROptimizer
import Test.Hspec

spec :: Spec
spec = describe "IROptimizer" $ do
    it "preserves programs for the initial convergence pass" $ do
        let program = appendInstruction (emptyProgram "_start") (instruction Halt [] 0)
            result = optimizeProgram program
        optimizationProgram result `shouldBe` program
        optimizationChanged result `shouldBe` False
