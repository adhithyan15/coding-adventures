module WasmOpcodesSpec (spec) where

import Test.Hspec
import WasmOpcodes

spec :: Spec
spec = describe "WasmOpcodes" $ do
    it "looks up bytes and names consistently" $ do
        opcodeByByte 0x6A `shouldBe` opcodeByName "i32.add"
        opcodeName <$> opcodeByByte 0x20 `shouldBe` Just "local.get"

    it "stores stack effects for representative instructions" $ do
        opcodeStackPop <$> opcodeByName "i32.add" `shouldBe` Just 2
        opcodeStackPush <$> opcodeByName "select" `shouldBe` Just 1
        opcodeImmediates <$> opcodeByName "call" `shouldBe` Just ["funcidx"]

    it "covers the core assembler/runtime instruction set" $ do
        map opcodeByName ["block", "end", "local.get", "local.set", "i32.const", "i32.load", "i32.store", "call"]
            `shouldSatisfy` all isJust

    it "returns Nothing for unknown instructions" $ do
        opcodeByByte 0xFF `shouldBe` Nothing
        opcodeByName "totally.fake" `shouldBe` Nothing

isJust :: Maybe a -> Bool
isJust maybeValue =
    case maybeValue of
        Just _ -> True
        Nothing -> False
