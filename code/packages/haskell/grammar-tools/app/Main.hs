module Main (main) where

import GrammarTools (parseParserGrammar, parseTokenGrammar)
import GrammarTools.Compiler (compileParserGrammar, compileTokenGrammar)
import System.Environment (getArgs)
import System.Exit (die)

main :: IO ()
main = do
    args <- getArgs
    case args of
        ["compile-tokens", inputPath, moduleName, outputPath] ->
            compileTokens inputPath moduleName outputPath
        ["compile-grammar", inputPath, moduleName, outputPath] ->
            compileGrammar inputPath moduleName outputPath
        _ ->
            die
                ( unlines
                    [ "Usage:"
                    , "  grammar-tools-cli compile-tokens <input.tokens> <module-name> <output.hs>"
                    , "  grammar-tools-cli compile-grammar <input.grammar> <module-name> <output.hs>"
                    ]
                )

compileTokens :: FilePath -> String -> FilePath -> IO ()
compileTokens inputPath moduleName outputPath = do
    source <- readFile inputPath
    grammar <-
        case parseTokenGrammar source of
            Left err -> die (show err)
            Right value -> pure value
    writeFile outputPath (compileTokenGrammar grammar inputPath moduleName)

compileGrammar :: FilePath -> String -> FilePath -> IO ()
compileGrammar inputPath moduleName outputPath = do
    source <- readFile inputPath
    grammar <-
        case parseParserGrammar source of
            Left err -> die (show err)
            Right value -> pure value
    writeFile outputPath (compileParserGrammar grammar inputPath moduleName)
