module BrainfuckIRCompilerSpec (spec) where

import BrainfuckIRCompiler
import CompilerIR
import Test.Hspec

spec :: Spec
spec = describe "BrainfuckIRCompiler" $ do
    it "lowers Brainfuck tape operations into semantic IR" $ do
        result <- unwrapEither (compileSource "++[>+<-]")
        let program = compileResultProgram result
            ops = map irOpcode (irInstructions program)
        irEntryLabel program `shouldBe` "_start"
        irDataDecls program `shouldBe` [IrDataDecl "tape" 30000 0]
        ops `shouldSatisfy` \opcodes ->
            all (`elem` opcodes) [LoadAddr, LoadByte, StoreByte, BranchZ, Jump, Halt]

    it "surfaces parser errors" $ do
        compileSource "[" `shouldSatisfy` either (const True) (const False)

unwrapEither :: Show err => Either err value -> IO value
unwrapEither result =
    case result of
        Left err -> expectationFailure (show err) >> error "unreachable after expectationFailure"
        Right value -> pure value
