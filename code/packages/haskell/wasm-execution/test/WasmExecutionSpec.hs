module WasmExecutionSpec (spec) where

import qualified Data.ByteString as BS
import Data.IORef
import Test.Hspec
import WasmExecution
import WasmTypes

spec :: Spec
spec = describe "WasmExecution" $ do
    it "evaluates simple const expressions" $ do
        evaluateConstExpr (BS.pack [0x41, 0x2A, 0x0B]) [] `shouldBe` Right (WasmI32 42)

    it "supports linear memory reads and writes" $ do
        memory <- newLinearMemory 1 Nothing
        storeI32 memory 4 42
        loadI32 memory 4 `shouldReturn` 42
        storeI32_8 memory 0 65
        loadI32_8u memory 0 `shouldReturn` 65

    it "executes a small i32 function body" $ do
        globalsRef <- newIORef []
        let engine =
                WasmExecutionEngine
                    { engineMemory = Nothing
                    , engineTables = []
                    , engineGlobals = globalsRef
                    , engineGlobalTypes = []
                    , engineFuncTypes = [FuncType [I32] [I32]]
                    , engineFuncBodies = [Just (FunctionBody [] (BS.pack [0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B]))]
                    , engineHostFunctions = [Nothing]
                    }
        callFunction engine 0 [WasmI32 5] `shouldReturn` Right [WasmI32 25]
