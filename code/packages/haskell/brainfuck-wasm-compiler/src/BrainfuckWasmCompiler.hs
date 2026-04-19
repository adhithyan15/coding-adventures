module BrainfuckWasmCompiler
    ( description
    , PackageError(..)
    , PackageResult(..)
    , compileSource
    , packSource
    , writeWasmFile
    ) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Brainfuck (BrainfuckOp)
import qualified BrainfuckIRCompiler
import CompilerIR hiding (description)
import IRToWasmCompiler hiding (description)
import IRToWasmValidator hiding (description)
import IROptimizer hiding (description)
import WasmModuleEncoder hiding (description)
import WasmTypes hiding (description)
import WasmValidator hiding (description)

description :: String
description = "Haskell Brainfuck to WebAssembly orchestration package"

data PackageError = PackageError
    { packageErrorStage :: String
    , packageErrorMessage :: String
    }
    deriving (Eq, Show)

data PackageResult = PackageResult
    { resultSource :: String
    , resultAst :: [BrainfuckOp]
    , resultRawIr :: IrProgram
    , resultOptimizedIr :: IrProgram
    , resultModule :: WasmModule
    , resultValidatedModule :: ValidatedModule
    , resultBytes :: ByteString
    , resultWasmPath :: Maybe FilePath
    }
    deriving (Eq, Show)

compileSource :: String -> Either PackageError PackageResult
compileSource source = do
    compiled <- mapLeft "parse" (BrainfuckIRCompiler.compileSource source)
    let rawIr = BrainfuckIRCompiler.compileResultProgram compiled
        optimizedIr = optimizationProgram (optimizeProgram rawIr)
        signatures = [FunctionSignature "_start" 0 (Just "_start")]
    case validateProgram optimizedIr of
        [] -> pure ()
        errors -> Left (PackageError "validate-ir" (show errors))
    moduleValue <- mapLeft "lower" (compileProgram optimizedIr signatures)
    validated <- mapLeft "validate-wasm" (validateModule moduleValue)
    bytesValue <- mapLeft "encode" (encodeModule moduleValue)
    Right
        PackageResult
            { resultSource = source
            , resultAst = BrainfuckIRCompiler.compileResultAst compiled
            , resultRawIr = rawIr
            , resultOptimizedIr = optimizedIr
            , resultModule = moduleValue
            , resultValidatedModule = validated
            , resultBytes = bytesValue
            , resultWasmPath = Nothing
            }

packSource :: String -> Either PackageError PackageResult
packSource = compileSource

writeWasmFile :: String -> FilePath -> IO (Either PackageError PackageResult)
writeWasmFile source path =
    case compileSource source of
        Left err -> pure (Left err)
        Right result -> do
            BS.writeFile path (resultBytes result)
            pure (Right result {resultWasmPath = Just path})

mapLeft :: Show err => String -> Either err value -> Either PackageError value
mapLeft stage result =
    case result of
        Left err -> Left (PackageError stage (show err))
        Right value -> Right value
