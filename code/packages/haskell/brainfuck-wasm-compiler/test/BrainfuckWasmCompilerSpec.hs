module BrainfuckWasmCompilerSpec (spec) where

import qualified Data.ByteString as BS
import BrainfuckWasmCompiler
import System.Directory
import System.FilePath
import Test.Hspec
import WasmRuntime

spec :: Spec
spec = describe "BrainfuckWasmCompiler" $ do
    it "compiles source into Wasm bytes" $ do
        result <- unwrapEither (compileSource "++>+")
        resultBytes result `shouldSatisfy` (not . BS.null)

    it "lowers structured Brainfuck loops into Wasm bytes" $ do
        result <- unwrapEither (compileSource "++[>+<-]")
        resultBytes result `shouldSatisfy` (not . BS.null)

    it "aliases packSource to compileSource" $ do
        compiled <- unwrapEither (compileSource "+")
        packed <- unwrapEither (packSource "+")
        resultBytes packed `shouldBe` resultBytes compiled

    it "writes Wasm bytes" $ do
        tempDir <- getTemporaryDirectory
        let path = tempDir </> "haskell-brainfuck-smoke.wasm"
        result <- writeWasmFile "+" path >>= unwrapEither
        resultWasmPath result `shouldBe` Just path
        doesFileExist path `shouldReturn` True

    it "runs emitted tape-mutating code through the local runtime" $ do
        result <- unwrapEither (compileSource "+")
        runResult <- loadAndRun (newRuntime Nothing) (resultBytes result) "_start" []
        runResult `shouldBe` Right []

unwrapEither :: Show err => Either err value -> IO value
unwrapEither result =
    case result of
        Left err -> expectationFailure (show err) >> error "unreachable after expectationFailure"
        Right value -> pure value
