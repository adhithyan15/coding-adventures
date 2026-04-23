module WasmSimulator
    ( description
    , WasmState(..)
    , simulateWasm
    ) where

import Data.ByteString (ByteString)
import WasmExecution hiding (description)
import WasmRuntime hiding (description)

description :: String
description = "Haskell WASM simulator facade backed by the runtime package"

data WasmState = WasmState
    { stateEntry :: String
    , stateResult :: [WasmValue]
    }
    deriving (Eq, Show)

simulateWasm :: WasmRuntime -> ByteString -> String -> [WasmValue] -> IO (Either String WasmState)
simulateWasm runtime wasmBytes entryLabel args = do
    result <- loadAndRun runtime wasmBytes entryLabel args
    pure (fmap (\values -> WasmState entryLabel values) result)
