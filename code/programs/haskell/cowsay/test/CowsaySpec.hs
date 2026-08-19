module CowsaySpec (spec) where

import Control.Exception (finally)
import qualified Data.Map.Strict as Map
import System.Directory
  ( createDirectory
  , getCurrentDirectory
  , getTemporaryDirectory
  , removeDirectoryRecursive
  , removeFile
  )
import System.FilePath ((</>))
import System.IO (hClose, openTempFile)
import Test.Hspec

import CliBuilder
  ( ParseResult (..)
  , ParserOutput (..)
  , loadSpecFromFile
  , newParser
  , parseArgs
  )
import CodingAdventures.PaintInstructions
  ( PaintGlyphPlacement (..)
  , PaintInstruction (..)
  , PaintScene (..)
  )
import qualified CodingAdventures.PaintVmAscii as PaintVmAscii
import Cowsay
import JsonValue (JsonValue (..))

withTempDir :: (FilePath -> IO a) -> IO a
withTempDir action = do
  tmpRoot <- getTemporaryDirectory
  (path, handle) <- openTempFile tmpRoot "cowsay-test-"
  hClose handle
  removeFile path
  createDirectory path
  action path `finally` removeDirectoryRecursive path

writeCow :: FilePath -> String -> String -> IO ()
writeCow dir name contents = writeFile (dir </> (name ++ ".cow")) contents

-- 'findRepoRoot' walks UP from its argument looking for CLAUDE.md, so it
-- needs an absolute starting directory — "." can't walk further up itself.
resolveRepoRoot :: IO FilePath
resolveRepoRoot = getCurrentDirectory >>= findRepoRoot

spec :: Spec
spec = do
  describe "wrapText" $ do
    it "does not wrap short text" $
      wrapText "hello" 40 `shouldBe` ["hello"]

    it "wraps long text at word boundaries" $
      wrapText "the quick brown fox jumps over" 10
        `shouldBe` ["the quick", "brown fox", "jumps over"]

    it "returns an empty line for empty text" $
      wrapText "" 40 `shouldBe` [""]

    it "keeps a single word longer than the width whole" $
      wrapText "supercalifragilisticexpialidocious" 5
        `shouldBe` ["supercalifragilisticexpialidocious"]

  describe "formatBubble" $ do
    it "returns empty string for no lines" $
      formatBubble [] False `shouldBe` ""

    it "draws a single-line speech bubble" $
      formatBubble ["hi"] False `shouldBe` " ____\n< hi >\n ----"

    it "draws a single-line thought bubble" $
      formatBubble ["hi"] True `shouldBe` " ____\n( hi )\n ----"

    it "draws a multi-line speech bubble with slash/pipe/backslash borders" $
      formatBubble ["one", "two", "three"] False
        `shouldBe` " _______\n/ one   \\\n| two   |\n\\ three /\n -------"

    it "draws a multi-line thought bubble with parens on every line" $
      formatBubble ["one", "two"] True
        `shouldBe` " _____\n( one )\n( two )\n -----"

  describe "normalizeTwoChars" $ do
    it "pads a one-character value" $ normalizeTwoChars "o" `shouldBe` "o "
    it "pads an empty value" $ normalizeTwoChars "" `shouldBe` "  "
    it "leaves a two-character value unchanged" $ normalizeTwoChars "oo" `shouldBe` "oo"
    it "truncates a longer value" $ normalizeTwoChars "ooo" `shouldBe` "oo"

  describe "resolveEyesAndTongue" $ do
    it "keeps base values when no modes are active" $
      resolveEyesAndTongue "oo" "  " [] `shouldBe` ("oo", "  ")

    it "borg overrides eyes only" $
      resolveEyesAndTongue "oo" "  " ["borg"] `shouldBe` ("==", "  ")

    it "dead overrides both eyes and tongue" $
      resolveEyesAndTongue "oo" "  " ["dead"] `shouldBe` ("XX", "U ")

    it "stoned overrides both eyes and tongue" $
      resolveEyesAndTongue "oo" "  " ["stoned"] `shouldBe` ("xx", "U ")

    it "ignores an unknown mode" $
      resolveEyesAndTongue "oo" "  " ["not-a-real-mode"] `shouldBe` ("oo", "  ")

  describe "loadCow" $ do
    it "loads the body between heredoc markers" $ withTempDir $ \dir -> do
      writeCow dir "default" "$the_cow = <<EOC;\n  $thoughts   ^__^\n   ($eyes)\nEOC\n"
      body <- loadCow "default" dir
      body `shouldBe` "  $thoughts   ^__^\n   ($eyes)\n"

    it "falls back to default.cow when the named cow is missing" $ withTempDir $ \dir -> do
      writeCow dir "default" "$the_cow = <<EOC;\nfallback\nEOC\n"
      body <- loadCow "does-not-exist" dir
      body `shouldBe` "fallback\n"

    it "falls back to default.cow instead of escaping via traversal" $ withTempDir $ \dir ->
      withTempDir $ \outsideDir -> do
        writeCow dir "default" "$the_cow = <<EOC;\nfallback\nEOC\n"
        writeCow outsideDir "secret" "$the_cow = <<EOC;\nSECRET\nEOC\n"
        writeCow outsideDir "outside" "$the_cow = <<EOC;\nSECRET\nEOC\n"
        mapM_
          (\malicious -> do
            body <- loadCow malicious dir
            body `shouldBe` "fallback\n")
          [ "../../../../../../etc/passwd"
          , "..\\..\\..\\secret"
          , "../outside"
          ]

    it "falls back to default.cow instead of following a rooted path override" $ withTempDir $ \dir ->
      withTempDir $ \outsideDir -> do
        writeCow dir "default" "$the_cow = <<EOC;\nfallback\nEOC\n"
        let rootedTarget = outsideDir </> "win"
        writeFile (rootedTarget ++ ".cow") "$the_cow = <<EOC;\nSECRET\nEOC\n"
        body <- loadCow rootedTarget dir
        body `shouldBe` "fallback\n"

  describe "composeContent" $ do
    let baseInvocation = CowsayInvocation
          { ciMessage = "hi"
          , ciEyes = "oo"
          , ciTongue = "  "
          , ciActiveModes = []
          , ciNoWrap = False
          , ciWidth = 40
          , ciThink = False
          , ciCowFile = "default"
          }

    it "composes bubble and cow with substitutions" $ withTempDir $ \dir -> do
      writeCow dir "default" "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n"
      content <- composeContent baseInvocation dir
      content `shouldBe` " ____\n< hi >\n ----\n\\ oo   \n"

    it "think mode uses o for thoughts and a paren bubble" $ withTempDir $ \dir -> do
      writeCow dir "default" "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n"
      content <- composeContent (baseInvocation { ciThink = True }) dir
      content `shouldBe` " ____\n( hi )\n ----\no oo   \n"

    it "a mode flag overrides eyes (and tongue) in the cow template" $ withTempDir $ \dir -> do
      writeCow dir "default" "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n"
      content <- composeContent (baseInvocation { ciActiveModes = ["dead"] }) dir
      content `shouldBe` " ____\n< hi >\n ----\n\\ XX U \n"

  describe "buildScene" $ do
    it "creates one glyph_run per non-blank line with correct placements" $ do
      let scene = buildScene "hi\n\nyo"
          glyphRuns = [i | i@PaintGlyphRun {} <- psInstructions scene]
      length glyphRuns `shouldBe` 2
      map pgpGlyphId (pgGlyphs (glyphRuns !! 0)) `shouldBe` [fromEnum 'h', fromEnum 'i']
      map pgpX (pgGlyphs (glyphRuns !! 0)) `shouldBe` [0, fromIntegral scaleXConst]
      map pgpGlyphId (pgGlyphs (glyphRuns !! 1)) `shouldBe` [fromEnum 'y', fromEnum 'o']
      map pgpY (pgGlyphs (glyphRuns !! 1)) `shouldBe` [fromIntegral (2 * scaleYConst), fromIntegral (2 * scaleYConst)]

    it "skips spaces rather than placing them" $ do
      let scene = buildScene "a b"
          glyphRuns = [i | i@PaintGlyphRun {} <- psInstructions scene]
      length glyphRuns `shouldBe` 1
      length (pgGlyphs (head glyphRuns)) `shouldBe` 2

    it "covers all lines in the scene dimensions" $ do
      let scene = buildScene "abc\nde"
      psWidth scene `shouldBe` fromIntegral (3 * scaleXConst)
      psHeight scene `shouldBe` fromIntegral (2 * scaleYConst)

  describe "render round trip" $
    mapM_
      (\(content, expected) -> it ("round-trips " ++ show content) $ do
        let scene = buildScene content
            output = PaintVmAscii.render scene (PaintVmAscii.AsciiOptions scaleXConst scaleYConst)
        output `shouldBe` Right expected)
      [ ("hi", "hi")
      , ("hello\nworld", "hello\nworld")
      , (" ____\n< hi >\n ----\n\\   ^__^\n", " ____\n< hi >\n ----\n\\   ^__^")
      ]

  describe "CLI glue" $ do
    describe "isListRequested" $ do
      it "is true when the flag is present" $
        isListRequested (Map.fromList [("list", JsonBool True)]) `shouldBe` True
      it "is false when the flag is absent" $
        isListRequested Map.empty `shouldBe` False
      it "is false when the flag is explicitly false" $
        isListRequested (Map.fromList [("list", JsonBool False)]) `shouldBe` False

    describe "resolveMessageFromArguments" $ do
      it "joins positional words" $
        resolveMessageFromArguments (Map.fromList [("message", JsonArray [JsonString "hello", JsonString "there"])])
          `shouldBe` Just "hello there"
      it "returns Nothing when arguments is empty" $
        resolveMessageFromArguments Map.empty `shouldBe` Nothing
      it "returns Nothing when the message list is empty" $
        resolveMessageFromArguments (Map.fromList [("message", JsonArray [])]) `shouldBe` Nothing

    describe "buildInvocation" $ do
      it "uses defaults when no flags are set" $ do
        let invocation = buildInvocation "hi" Map.empty
        ciMessage invocation `shouldBe` "hi"
        ciEyes invocation `shouldBe` "oo"
        ciTongue invocation `shouldBe` "  "
        ciCowFile invocation `shouldBe` "default"
        ciNoWrap invocation `shouldBe` False
        ciThink invocation `shouldBe` False
        ciWidth invocation `shouldBe` 40
        ciActiveModes invocation `shouldBe` []

      it "honors explicit flags" $ do
        let flags = Map.fromList
              [ ("eyes", JsonString "^^")
              , ("tongue", JsonString "vv")
              , ("cowfile", JsonString "dragon")
              , ("nowrap", JsonBool True)
              , ("think", JsonBool True)
              , ("width", JsonNumber 20)
              , ("borg", JsonBool True)
              ]
            invocation = buildInvocation "hi" flags
        ciEyes invocation `shouldBe` "^^"
        ciTongue invocation `shouldBe` "vv"
        ciCowFile invocation `shouldBe` "dragon"
        ciNoWrap invocation `shouldBe` True
        ciThink invocation `shouldBe` True
        ciWidth invocation `shouldBe` 20
        ciActiveModes invocation `shouldBe` ["borg"]

      it "clamps a very large width and rejects a negative width" $ do
        ciWidth (buildInvocation "hi" (Map.fromList [("width", JsonNumber 99999999999)])) `shouldBe` 2147483647
        ciWidth (buildInvocation "hi" (Map.fromList [("width", JsonNumber (-5))])) `shouldBe` 1

    describe "listCowFiles" $
      it "returns sorted basenames" $ withTempDir $ \dir -> do
        writeCow dir "tux" ""
        writeCow dir "default" ""
        writeCow dir "dragon" ""
        names <- listCowFiles dir
        names `shouldBe` ["default", "dragon", "tux"]

  describe "CliBuilder argv convention" $ do
    -- Regression test: unlike the C#/F# ports, this Haskell CliBuilder's
    -- parseArgs DOES expect a leading program-name placeholder (it
    -- pattern-matches "program : argv" and errors if argv is empty),
    -- matching the C/Go convention. Verified against the real Parser, not
    -- just hand-built flags/arguments maps.
    it "does not drop the first token when a program-name placeholder is prepended" $ do
      repoRoot <- resolveRepoRoot
      let specPath = repoRoot </> "code" </> "specs" </> "cowsay.json"
      specResult <- loadSpecFromFile specPath
      case specResult of
        Left err -> expectationFailure ("failed to load spec: " ++ show err)
        Right cliSpec -> do
          case parseArgs (newParser cliSpec) ["cowsay", "hello"] of
            Right (ParseOutput result) ->
              resolveMessageFromArguments (resultArguments result) `shouldBe` Just "hello"
            other -> expectationFailure ("unexpected result: " ++ show other)

          case parseArgs (newParser cliSpec) ["cowsay", "hello", "world"] of
            Right (ParseOutput result) ->
              resolveMessageFromArguments (resultArguments result) `shouldBe` Just "hello world"
            other -> expectationFailure ("unexpected result: " ++ show other)

  describe "end-to-end golden output" $ do
    it "resolves the real cows directory" $ do
      repoRoot <- resolveRepoRoot
      let cowsDir = repoRoot </> "code" </> "specs" </> "cows"
      names <- listCowFiles cowsDir
      names `shouldContain` ["default"]

    it "default cow speaking Hello, World!" $ do
      repoRoot <- resolveRepoRoot
      let cowsDir = repoRoot </> "code" </> "specs" </> "cows"
          invocation = CowsayInvocation
            { ciMessage = "Hello, World!"
            , ciEyes = "oo"
            , ciTongue = "  "
            , ciActiveModes = []
            , ciNoWrap = False
            , ciWidth = 40
            , ciThink = False
            , ciCowFile = "default"
            }
      result <- Cowsay.render invocation cowsDir
      result `shouldBe` Right (intercalateLines
        [ " _______________"
        , "< Hello, World! >"
        , " ---------------"
        , "        \\   ^__^"
        , "         \\  (oo)\\_______"
        , "            (__)\\       )\\/\\"
        , "                ||----w |"
        , "                ||     ||"
        ])

    it "borg mode thinking with the default cow" $ do
      repoRoot <- resolveRepoRoot
      let cowsDir = repoRoot </> "code" </> "specs" </> "cows"
          invocation = CowsayInvocation
            { ciMessage = "beep"
            , ciEyes = "oo"
            , ciTongue = "  "
            , ciActiveModes = ["borg"]
            , ciNoWrap = False
            , ciWidth = 40
            , ciThink = True
            , ciCowFile = "default"
            }
      result <- Cowsay.render invocation cowsDir
      result `shouldBe` Right (intercalateLines
        [ " ______"
        , "( beep )"
        , " ------"
        , "        o   ^__^"
        , "         o  (==)\\_______"
        , "            (__)\\       )\\/\\"
        , "                ||----w |"
        , "                ||     ||"
        ])

-- Small local helper so the golden end-to-end tests don't need an extra dep.

intercalateLines :: [String] -> String
intercalateLines [] = ""
intercalateLines ls = foldr1 (\a b -> a ++ "\n" ++ b) ls
