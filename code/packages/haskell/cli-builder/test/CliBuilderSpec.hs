module CliBuilderSpec (spec) where

import qualified Data.Map.Strict as Map
import CliBuilder
import Test.Hspec

spec :: Spec
spec = do
    describe "spec loading" $ do
        it "applies defaults for a minimal spec" $ do
            let result = loadSpecFromStr "{\"cli_builder_spec_version\":\"1.0\",\"name\":\"minimal\",\"description\":\"Minimal spec\"}"
            case result of
                Right loaded ->
                    do
                        cliParsingMode loaded `shouldBe` "gnu"
                        builtinHelp (cliBuiltinFlags loaded) `shouldBe` True
                        builtinVersion (cliBuiltinFlags loaded) `shouldBe` True
                Left err -> expectationFailure (show err)

        it "rejects unsupported versions" $ do
            loadSpecFromStr "{\"cli_builder_spec_version\":\"2.0\",\"name\":\"x\",\"description\":\"y\"}"
                `shouldSatisfy` isSpecError

        it "rejects flags with no forms" $ do
            loadSpecFromStr "{\"cli_builder_spec_version\":\"1.0\",\"name\":\"x\",\"description\":\"y\",\"flags\":[{\"id\":\"bad\",\"description\":\"bad\",\"type\":\"boolean\"}]}"
                `shouldSatisfy` isSpecError

        it "rejects circular requires dependencies" $ do
            loadSpecFromStr "{\"cli_builder_spec_version\":\"1.0\",\"name\":\"x\",\"description\":\"y\",\"flags\":[{\"id\":\"a\",\"short\":\"a\",\"description\":\"a\",\"type\":\"boolean\",\"requires\":[\"b\"]},{\"id\":\"b\",\"short\":\"b\",\"description\":\"b\",\"type\":\"boolean\",\"requires\":[\"a\"]}]}"
                `shouldSatisfy` isSpecError

        it "rejects multiple variadic arguments" $ do
            loadSpecFromStr "{\"cli_builder_spec_version\":\"1.0\",\"name\":\"x\",\"description\":\"y\",\"arguments\":[{\"id\":\"a\",\"name\":\"A\",\"description\":\"a\",\"type\":\"string\",\"variadic\":true},{\"id\":\"b\",\"name\":\"B\",\"description\":\"b\",\"type\":\"string\",\"variadic\":true}]}"
                `shouldSatisfy` isSpecError

    describe "help generation" $ do
        it "renders root help with options and arguments" $ do
            let helpTextValue = generateRootHelp echoSpec
            helpTextValue `shouldContain` "USAGE"
            helpTextValue `shouldContain` "OPTIONS"
            helpTextValue `shouldContain` "ARGUMENTS"
            helpTextValue `shouldContain` "--help"

        it "renders subcommand help with global options" $ do
            let helpTextValue = generateCommandHelp gitSpec ["git", "add"]
            helpTextValue `shouldContain` "git add"
            helpTextValue `shouldContain` "--no-pager"
            helpTextValue `shouldContain` "PATHSPEC"

    describe "token classification" $ do
        let lsClassifier = map flagInfoFromDef (cliFlags lsSpec)
            grepClassifier = map flagInfoFromDef (cliFlags grepSpec)
            sdlClassifier =
                [ FlagInfo "classpath" Nothing Nothing (Just "classpath") False False False
                , FlagInfo "cp" Nothing Nothing (Just "cp") False False False
                , FlagInfo "c" (Just 'c') Nothing Nothing True False False
                ]
        it "classifies builtins and long values" $ do
            classifyToken lsClassifier "--" `shouldBe` [EndOfFlags]
            classifyToken lsClassifier "--help" `shouldBe` [LongFlag "help"]
            classifyToken grepClassifier "--regexp=foo" `shouldBe` [LongFlagWithValue "regexp" "foo"]

        it "classifies short stacks and inline values" $ do
            classifyToken lsClassifier "-la" `shouldBe` [StackedFlags "la"]
            classifyToken grepClassifier "-efoo" `shouldBe` [ShortFlagWithValue 'e' "foo"]

        it "classifies single-dash-long flags before short stacks" $ do
            classifyToken sdlClassifier "-classpath" `shouldBe` [SingleDashLong "classpath"]

        it "supports traditional-mode stack detection" $ do
            let flagsValue =
                    [ FlagInfo "extract" (Just 'x') Nothing Nothing True False False
                    , FlagInfo "verbose" (Just 'v') Nothing Nothing True False False
                    , FlagInfo "file" (Just 'f') Nothing Nothing False False False
                    ]
            classifyTraditional flagsValue "xvf" [] `shouldBe` [StackedFlags "xvf"]

    describe "parser" $ do
        it "parses variadic arguments and boolean flags" $ do
            case parseArgs (newParser echoSpec) ["echo", "-n", "hello", "world"] of
                Right (ParseOutput result) -> do
                    Map.lookup "no-newline" (resultFlags result) `shouldBe` Just (JsonBool True)
                    Map.lookup "string" (resultArguments result) `shouldBe` Just (JsonArray [JsonString "hello", JsonString "world"])
                    resultCommandPath result `shouldBe` ["echo"]
                other -> expectationFailure (show other)

        it "applies defaults and flag requirements" $ do
            case parseArgs (newParser lsSpec) ["ls"] of
                Right (ParseOutput result) -> do
                    Map.lookup "path" (resultArguments result) `shouldBe` Just (JsonString ".")
                    Map.lookup "all" (resultFlags result) `shouldBe` Just (JsonBool False)
                other -> expectationFailure (show other)

            case parseArgs (newParser lsSpec) ["ls", "-h"] of
                Left (ParseFailure (ParseErrors errs)) ->
                    map parseErrorType errs `shouldContain` ["missing_dependency_flag"]
                other -> expectationFailure (show other)

        it "routes through subcommands while honoring global flags" $ do
            case parseArgs (newParser gitSpec) ["git", "--no-pager", "add", "README.md"] of
                Right (ParseOutput result) -> do
                    resultCommandPath result `shouldBe` ["git", "add"]
                    Map.lookup "no-pager" (resultFlags result) `shouldBe` Just (JsonBool True)
                    Map.lookup "pathspec" (resultArguments result) `shouldBe` Just (JsonArray [JsonString "README.md"])
                other -> expectationFailure (show other)

        it "returns help and version outputs" $ do
            parseArgs (newParser gitSpec) ["git", "add", "--help"]
                `shouldBe` Right (HelpOutput (HelpResult (generateCommandHelp gitSpec ["git", "add"]) ["git", "add"]))
            parseArgs (newParser echoSpec) ["echo", "--version"]
                `shouldBe` Right (VersionOutput (VersionResult "8.32"))

        it "supports variadic source plus trailing destination" $ do
            case parseArgs (newParser cpSpec) ["cp", "a.txt", "b.txt", "/dest"] of
                Right (ParseOutput result) -> do
                    Map.lookup "source" (resultArguments result) `shouldBe` Just (JsonArray [JsonString "a.txt", JsonString "b.txt"])
                    Map.lookup "dest" (resultArguments result) `shouldBe` Just (JsonString "/dest")
                other -> expectationFailure (show other)

        it "reports conflicts" $ do
            case parseArgs (newParser echoSpec) ["echo", "-e", "-E", "hello"] of
                Left (ParseFailure (ParseErrors errs)) ->
                    map parseErrorType errs `shouldContain` ["conflicting_flags"]
                other -> expectationFailure (show other)

echoSpec :: CliSpec
echoSpec = expectSpec $
    loadSpecFromStr $
        unlines
            [ "{"
            , "  \"cli_builder_spec_version\": \"1.0\","
            , "  \"name\": \"echo\","
            , "  \"description\": \"Display a line of text\","
            , "  \"version\": \"8.32\","
            , "  \"flags\": ["
            , "    {\"id\":\"no-newline\",\"short\":\"n\",\"description\":\"Do not output trailing newline\",\"type\":\"boolean\"},"
            , "    {\"id\":\"enable-escapes\",\"short\":\"e\",\"description\":\"Enable interpretation of backslash escapes\",\"type\":\"boolean\",\"conflicts_with\":[\"disable-escapes\"]},"
            , "    {\"id\":\"disable-escapes\",\"short\":\"E\",\"description\":\"Disable interpretation of backslash escapes\",\"type\":\"boolean\",\"conflicts_with\":[\"enable-escapes\"]}"
            , "  ],"
            , "  \"arguments\": ["
            , "    {\"id\":\"string\",\"name\":\"STRING\",\"description\":\"Text to print\",\"type\":\"string\",\"required\":false,\"variadic\":true,\"variadic_min\":0}"
            , "  ]"
            , "}"
            ]

lsSpec :: CliSpec
lsSpec = expectSpec $
    loadSpecFromStr $
        unlines
            [ "{"
            , "  \"cli_builder_spec_version\": \"1.0\","
            , "  \"name\": \"ls\","
            , "  \"description\": \"List directory contents\","
            , "  \"version\": \"8.32\","
            , "  \"flags\": ["
            , "    {\"id\":\"long-listing\",\"short\":\"l\",\"description\":\"Use long listing format\",\"type\":\"boolean\",\"conflicts_with\":[\"single-column\"]},"
            , "    {\"id\":\"all\",\"short\":\"a\",\"long\":\"all\",\"description\":\"List hidden entries\",\"type\":\"boolean\"},"
            , "    {\"id\":\"human-readable\",\"short\":\"h\",\"long\":\"human-readable\",\"description\":\"Human readable sizes\",\"type\":\"boolean\",\"requires\":[\"long-listing\"]},"
            , "    {\"id\":\"single-column\",\"short\":\"1\",\"description\":\"One per line\",\"type\":\"boolean\",\"conflicts_with\":[\"long-listing\"]}"
            , "  ],"
            , "  \"arguments\": ["
            , "    {\"id\":\"path\",\"name\":\"PATH\",\"description\":\"Directory or file to list\",\"type\":\"path\",\"required\":false,\"variadic\":true,\"variadic_min\":0,\"default\":\".\"}"
            , "  ]"
            , "}"
            ]

grepSpec :: CliSpec
grepSpec = expectSpec $
    loadSpecFromStr $
        unlines
            [ "{"
            , "  \"cli_builder_spec_version\": \"1.0\","
            , "  \"name\": \"grep\","
            , "  \"description\": \"Print lines that match patterns\","
            , "  \"flags\": ["
            , "    {\"id\":\"ignore-case\",\"short\":\"i\",\"long\":\"ignore-case\",\"description\":\"Ignore case\",\"type\":\"boolean\"},"
            , "    {\"id\":\"regexp\",\"short\":\"e\",\"long\":\"regexp\",\"description\":\"Pattern\",\"type\":\"string\",\"value_name\":\"PATTERN\"}"
            , "  ]"
            , "}"
            ]

gitSpec :: CliSpec
gitSpec = expectSpec $
    loadSpecFromStr $
        unlines
            [ "{"
            , "  \"cli_builder_spec_version\": \"1.0\","
            , "  \"name\": \"git\","
            , "  \"description\": \"The stupid content tracker\","
            , "  \"version\": \"2.43.0\","
            , "  \"global_flags\": ["
            , "    {\"id\":\"no-pager\",\"long\":\"no-pager\",\"description\":\"Disable pager\",\"type\":\"boolean\"}"
            , "  ],"
            , "  \"commands\": ["
            , "    {"
            , "      \"id\": \"cmd-add\","
            , "      \"name\": \"add\","
            , "      \"description\": \"Add file contents to the index\","
            , "      \"flags\": ["
            , "        {\"id\":\"dry-run\",\"short\":\"n\",\"long\":\"dry-run\",\"description\":\"Dry run\",\"type\":\"boolean\"},"
            , "        {\"id\":\"verbose\",\"short\":\"v\",\"long\":\"verbose\",\"description\":\"Verbose\",\"type\":\"boolean\"}"
            , "      ],"
            , "      \"arguments\": ["
            , "        {\"id\":\"pathspec\",\"name\":\"PATHSPEC\",\"description\":\"Files to add\",\"type\":\"path\",\"required\":false,\"variadic\":true,\"variadic_min\":0}"
            , "      ]"
            , "    }"
            , "  ]"
            , "}"
            ]

cpSpec :: CliSpec
cpSpec = expectSpec $
    loadSpecFromStr $
        unlines
            [ "{"
            , "  \"cli_builder_spec_version\": \"1.0\","
            , "  \"name\": \"cp\","
            , "  \"description\": \"Copy files and directories\","
            , "  \"flags\": ["
            , "    {\"id\":\"recursive\",\"short\":\"r\",\"long\":\"recursive\",\"description\":\"Recursive\",\"type\":\"boolean\"},"
            , "    {\"id\":\"force\",\"short\":\"f\",\"long\":\"force\",\"description\":\"Force\",\"type\":\"boolean\",\"conflicts_with\":[\"interactive\",\"no-clobber\"]},"
            , "    {\"id\":\"interactive\",\"short\":\"i\",\"long\":\"interactive\",\"description\":\"Interactive\",\"type\":\"boolean\",\"conflicts_with\":[\"force\",\"no-clobber\"]},"
            , "    {\"id\":\"no-clobber\",\"short\":\"n\",\"long\":\"no-clobber\",\"description\":\"No clobber\",\"type\":\"boolean\",\"conflicts_with\":[\"force\",\"interactive\"]}"
            , "  ],"
            , "  \"arguments\": ["
            , "    {\"id\":\"source\",\"name\":\"SOURCE\",\"description\":\"Source files\",\"type\":\"path\",\"required\":true,\"variadic\":true,\"variadic_min\":1},"
            , "    {\"id\":\"dest\",\"name\":\"DEST\",\"description\":\"Destination\",\"type\":\"path\",\"required\":true}"
            , "  ]"
            , "}"
            ]

expectSpec :: Either CliBuilderError CliSpec -> CliSpec
expectSpec result =
    case result of
        Right specValue -> specValue
        Left err -> error (show err)

isSpecError :: Either CliBuilderError a -> Bool
isSpecError result =
    case result of
        Left (SpecError _) -> True
        _ -> False
