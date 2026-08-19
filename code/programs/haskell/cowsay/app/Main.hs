-- | cowsay (Haskell) — entry point.
--
-- Thin CLI wiring: load and parse @code\/specs\/cowsay.json@ via
-- CliBuilder, resolve the parsed flags\/arguments into a
-- 'Cowsay.CowsayInvocation', and hand off to 'Cowsay.render' for the
-- actual formatting + paint-vm-ascii render. See
-- @code\/specs\/cowsay-paintvm-pipeline.md@ for the design.
--
-- CliBuilder's 'parseArgs' follows the C\/Go argv convention where index 0
-- is the program name ('parseArgs' pattern-matches @program : argv@);
-- 'System.Environment.getArgs' here does NOT include the program name —
-- passing it straight through would silently drop the first real CLI
-- token, the same pitfall documented for the C#\/F# ports (see
-- lessons.md, \"C#\" section).
module Main (main) where

import Data.List (intercalate)
import Data.Map.Strict (Map)
import System.Directory (getCurrentDirectory)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.FilePath ((</>))
import System.IO
  ( hIsTerminalDevice
  , hPutStrLn
  , hSetEncoding
  , hSetNewlineMode
  , noNewlineTranslation
  , stderr
  , stdin
  , stdout
  , utf8
  )

import CliBuilder
  ( CliBuilderError (..)
  , HelpResult (..)
  , ParseError (..)
  , ParseErrors (..)
  , ParseResult (..)
  , ParserOutput (..)
  , VersionResult (..)
  , loadSpecFromFile
  , newParser
  , parseArgs
  )
import Cowsay
import JsonValue (JsonValue)

main :: IO ()
main = do
  hSetEncoding stdout utf8
  hSetEncoding stderr utf8
  -- Match the LF-only output every other cowsay port produces on every
  -- platform; without this, GHC's default host-native newline translation
  -- rewrites "\n" to "\r\n" on Windows.
  hSetNewlineMode stdout noNewlineTranslation
  hSetNewlineMode stderr noNewlineTranslation
  cwd <- getCurrentDirectory
  repoRoot <- findRepoRoot cwd
  let specPath = repoRoot </> "code" </> "specs" </> "cowsay.json"
      cowsDir = repoRoot </> "code" </> "specs" </> "cows"
  specResult <- loadSpecFromFile specPath
  case specResult of
    Left err -> failWith err
    Right spec -> do
      args <- getArgs
      case parseArgs (newParser spec) ("cowsay" : args) of
        Left err -> failWith err
        Right (HelpOutput helpResult) -> putStrLn (helpText helpResult)
        Right (VersionOutput versionResult) -> putStrLn (versionText versionResult)
        Right (ParseOutput parseResult) -> runCowsay cowsDir parseResult

runCowsay :: FilePath -> ParseResult -> IO ()
runCowsay cowsDir parseResult = do
  let flags = resultFlags parseResult
      arguments = resultArguments parseResult
  if isListRequested flags
    then mapM_ putStrLn =<< listCowFiles cowsDir
    else do
      messageMaybe <- resolveMessage arguments
      case messageMaybe of
        Just msg | not (null msg) -> do
          let invocation = buildInvocation msg flags
          result <- Cowsay.render invocation cowsDir
          case result of
            Left err -> hPutStrLn stderr (show err) >> exitFailure
            Right output -> putStrLn output
        _ -> pure ()

resolveMessage :: Map String JsonValue -> IO (Maybe String)
resolveMessage arguments = case resolveMessageFromArguments arguments of
  Just msg -> pure (Just msg)
  Nothing -> do
    isTerminal <- hIsTerminalDevice stdin
    if isTerminal
      then pure Nothing
      else Just . trim <$> getContents

trim :: String -> String
trim = f . f
  where
    f = reverse . dropWhile (`elem` (" \t\r\n" :: String))

failWith :: CliBuilderError -> IO ()
failWith err = hPutStrLn stderr (formatCliBuilderError err) >> exitFailure

formatCliBuilderError :: CliBuilderError -> String
formatCliBuilderError (SpecError msg) = msg
formatCliBuilderError (JsonError msg) = msg
formatCliBuilderError (IoError msg) = msg
formatCliBuilderError (ParseFailure (ParseErrors errs)) =
  intercalate "\n" [parseErrorType e ++ ": " ++ parseErrorMessage e | e <- errs]
