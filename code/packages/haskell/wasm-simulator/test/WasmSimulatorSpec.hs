module WasmSimulatorSpec (spec) where

import Test.Hspec
import WasmAssembler
import WasmExecution
import WasmRuntime
import WasmSimulator

spec :: Spec
spec = describe "WasmSimulator" $ do
    it "simulates an assembled WASM export" $ do
        let assembly =
                unlines
                    [ ".type 0 params=none results=i32"
                    , ".export function answer 0"
                    , ".func 0 type=0 locals=none"
                    , "i32.const 42"
                    , "return"
                    , "end"
                    , ".endfunc"
                    ]
            runtime = newRuntime Nothing
        case assembleToBytes assembly of
            Left err -> expectationFailure (show err)
            Right wasmBytes -> do
                result <- simulateWasm runtime wasmBytes "answer" []
                result `shouldBe` Right (WasmState "answer" [WasmI32 42])
