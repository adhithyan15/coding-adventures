module WasmAssemblerSpec (spec) where

import Data.IORef
import Test.Hspec
import WasmAssembler
import WasmExecution
import WasmRuntime

spec :: Spec
spec = describe "WasmAssembler" $ do
    it "assembles a simple function and runs it" $ do
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
            Right bytesValue -> do
                result <- loadAndRun runtime bytesValue "answer" []
                result `shouldBe` Right [WasmI32 42]

    it "assembles a memory load with block syntax" $ do
        let assembly =
                unlines
                    [ ".type 0 params=none results=i32"
                    , ".memory 0 min=1 max=none"
                    , ".export function read_answer 0"
                    , ".func 0 type=0 locals=none"
                    , "block void"
                    , "end"
                    , "i32.const 0"
                    , "i32.load align=2 offset=4"
                    , "return"
                    , "end"
                    , ".endfunc"
                    , ".data 0 offset=4 bytes=2A,00,00,00"
                    ]
            runtime = newRuntime Nothing
        case assembleToBytes assembly of
            Left err -> expectationFailure (show err)
            Right bytesValue -> do
                result <- loadAndRun runtime bytesValue "read_answer" []
                result `shouldBe` Right [WasmI32 42]

    it "supports a minimal WASI fd_write host call" $ do
        outputRef <- newIORef []
        host <- newWasiHost WasiConfig {wasiStdout = \chunk -> modifyIORef outputRef (++ [chunk])}
        let runtime = newRuntime (Just host)
            assembly =
                unlines
                    [ ".type 0 params=i32,i32,i32,i32 results=i32"
                    , ".type 1 params=none results=i32"
                    , ".import function wasi_snapshot_preview1 fd_write type=0"
                    , ".memory 0 min=1 max=none"
                    , ".export function _start 1"
                    , ".func 0 type=1 locals=i32"
                    , "i32.const 12"
                    , "i32.const 67"
                    , "i32.store8 align=0 offset=0"
                    , "i32.const 0"
                    , "i32.const 12"
                    , "i32.store align=2 offset=0"
                    , "i32.const 4"
                    , "i32.const 1"
                    , "i32.store align=2 offset=0"
                    , "i32.const 1"
                    , "i32.const 0"
                    , "i32.const 1"
                    , "i32.const 8"
                    , "call 0"
                    , "return"
                    , "end"
                    , ".endfunc"
                    ]
        case assembleToBytes assembly of
            Left err -> expectationFailure (show err)
            Right bytesValue -> do
                result <- loadAndRun runtime bytesValue "_start" []
                output <- readIORef outputRef
                result `shouldBe` Right [WasmI32 0]
                output `shouldBe` ["C"]

    it "reports unterminated function blocks" $ do
        parseAssembly ".func 0 type=0 locals=none" `shouldSatisfy` isLeft

    it "reports unknown instructions" $ do
        assembleToBytes (unlines [".type 0 params=none results=none", ".func 0 type=0 locals=none", "totally.fake", ".endfunc"])
            `shouldSatisfy` isLeft

isLeft :: Either a b -> Bool
isLeft eitherValue =
    case eitherValue of
        Left _ -> True
        Right _ -> False
