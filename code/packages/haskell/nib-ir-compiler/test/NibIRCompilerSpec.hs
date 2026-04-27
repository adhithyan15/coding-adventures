module NibIRCompilerSpec (spec) where

import CompilerIR
import NibIRCompiler
import NibTypeChecker
import Test.Hspec

spec :: Spec
spec = describe "NibIRCompiler" $ do
    it "emits entrypoint and function labels" $ do
        let checked = checkSource "fn answer() -> u4 { return 7; }"
            program = compileResultProgram (compileNib (typeCheckTypedAst checked) releaseConfig)
        map irOpcode (irInstructions program) `shouldSatisfy` \ops ->
            Label `elem` ops && Halt `elem` ops && Ret `elem` ops
        irInstructions program `shouldSatisfy` any (hasLabel "_fn_answer")

    it "places calls in the IR" $ do
        let checked = checkSource "fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }"
            program = compileResultProgram (compileNib (typeCheckTypedAst checked) releaseConfig)
        map irOpcode (irInstructions program) `shouldContain` [Call]

hasLabel :: String -> IrInstruction -> Bool
hasLabel expected inst =
    irOpcode inst == Label && irOperands inst == [LabelRef expected]
