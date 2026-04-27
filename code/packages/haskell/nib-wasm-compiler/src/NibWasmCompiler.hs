module NibWasmCompiler
    ( description
    , PackageError(..)
    , PackageResult(..)
    , compileSource
    , packSource
    , writeWasmFile
    , extractSignatures
    ) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import CompilerIR hiding (description)
import IRToWasmCompiler hiding (description)
import IRToWasmValidator hiding (description)
import IROptimizer hiding (description)
import NibIRCompiler hiding (description)
import NibParser hiding (description)
import NibTypeChecker hiding (description)
import Parser.AST
import WasmModuleEncoder hiding (description)
import WasmTypes hiding (description)
import WasmValidator hiding (description)

description :: String
description = "Haskell Nib to WebAssembly orchestration package"

data PackageError = PackageError
    { packageErrorStage :: String
    , packageErrorMessage :: String
    }
    deriving (Eq, Show)

data PackageResult = PackageResult
    { resultSource :: String
    , resultAst :: ASTNode
    , resultTypedAst :: TypedAst
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
    ast <- mapLeft "parse" (tokenizeAndParseNib source)
    let checked = checkAst ast
    if typeCheckOk checked
        then pure ()
        else Left (PackageError "type-check" (show (typeCheckErrors checked)))
    let typedAst = typeCheckTypedAst checked
        rawIr = compileResultProgram (compileNib typedAst releaseConfig)
        optimizedIr = optimizationProgram (optimizeProgram rawIr)
        signatures = extractSignatures typedAst
    case validateProgram optimizedIr of
        [] -> pure ()
        errors -> Left (PackageError "validate-ir" (show errors))
    moduleValue <- mapLeft "lower" (compileProgram optimizedIr signatures)
    validated <- mapLeft "validate-wasm" (validateModule moduleValue)
    bytesValue <- mapLeft "encode" (encodeModule moduleValue)
    Right
        PackageResult
            { resultSource = source
            , resultAst = ast
            , resultTypedAst = typedAst
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

extractSignatures :: TypedAst -> [FunctionSignature]
extractSignatures typedAst =
    FunctionSignature "_start" 0 (Just "_start")
        : [ FunctionSignature ("_fn_" ++ name) (countParams fn) (Just name)
          | fn <- functionNodes (typedAstRoot typedAst)
          , Just name <- [firstName fn]
          ]

mapLeft :: Show err => String -> Either err value -> Either PackageError value
mapLeft stage result =
    case result of
        Left err -> Left (PackageError stage (show err))
        Right value -> Right value
