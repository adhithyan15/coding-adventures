module NibWasmCompilerSpec (spec) where

import qualified Data.ByteString as BS
import NibWasmCompiler
import System.Directory
import System.FilePath
import Test.Hspec
import WasmExecution
import WasmRuntime

spec :: Spec
spec = describe "NibWasmCompiler" $ do
    it "compiles a simple function into Wasm bytes" $ do
        result <- unwrapEither (compileSource "fn answer() -> u4 { return 7; }")
        resultBytes result `shouldSatisfy` (not . BS.null)
        length (extractSignatures (resultTypedAst result)) `shouldBe` 2

    it "aliases packSource to compileSource" $ do
        compiled <- unwrapEither (compileSource "fn answer() -> u4 { return 7; }")
        packed <- unwrapEither (packSource "fn answer() -> u4 { return 7; }")
        resultBytes packed `shouldBe` resultBytes compiled

    it "runs exported functions through the local runtime" $ do
        result <- unwrapEither (compileSource "fn answer() -> u4 { return 7; }")
        runResult <- loadAndRun (newRuntime Nothing) (resultBytes result) "answer" []
        runResult `shouldBe` Right [WasmI32 7]

    it "writes Wasm bytes" $ do
        tempDir <- getTemporaryDirectory
        let path = tempDir </> "haskell-nib-smoke.wasm"
        result <- writeWasmFile "fn answer() -> u4 { return 7; }" path >>= unwrapEither
        resultWasmPath result `shouldBe` Just path
        doesFileExist path `shouldReturn` True

    it "reports type-check failures with a stage" $ do
        case compileSource "fn bad() -> u4 { let x: u4 = true; return 1; }" of
            Left err -> packageErrorStage err `shouldBe` "type-check"
            Right _ -> expectationFailure "expected type-check failure"

unwrapEither :: Show err => Either err value -> IO value
unwrapEither result =
    case result of
        Left err -> expectationFailure (show err) >> error "unreachable after expectationFailure"
        Right value -> pure value
