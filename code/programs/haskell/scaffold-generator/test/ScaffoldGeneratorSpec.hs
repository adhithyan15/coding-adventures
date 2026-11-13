module ScaffoldGeneratorSpec (scaffoldGeneratorSpec) where

import Test.Hspec

import Data.List (isInfixOf)

import ScaffoldGenerator

-- | Whether a needle occurs anywhere in a haystack.
containsText :: String -> String -> Bool
containsText = isInfixOf

scaffoldGeneratorSpec :: Spec
scaffoldGeneratorSpec = do
    describe "isKebabCase" $ do
        it "accepts valid names" $ do
            isKebabCase "logic-gates" `shouldBe` True

        it "rejects invalid names" $ do
            isKebabCase "LogicGates" `shouldBe` False
            isKebabCase "logic-Gates" `shouldBe` False
            isKebabCase "bad--name" `shouldBe` False
            isKebabCase "café" `shouldBe` False
            isKebabCase "logic-١" `shouldBe` False

    describe "isSafeDescription" $ do
        it "accepts a single printable line" $ do
            isSafeDescription "A package for in-memory values." `shouldBe` True

        it "rejects metadata-breaking control characters" $ do
            isSafeDescription "safe\nbuild-type: Custom" `shouldBe` False
            isSafeDescription "safe\rlicense: NONE" `shouldBe` False
            isSafeDescription "safe\tfield" `shouldBe` False
            isSafeDescription "safe\x85next-field" `shouldBe` False
            isSafeDescription "safe\x2028next-line" `shouldBe` False

    describe "toModuleName" $ do
        it "converts kebab case to module case" $ do
            toModuleName "logic-gates" `shouldBe` "LogicGates"

    describe "capabilityManifestContents" $ do
        it "renders the schema-v1 pure-library golden document" $ do
            expected <-
                readFile
                    "../../../specs/fixtures/scaffold-generator/haskell_library_required_capabilities.json"
            capabilityManifestContents "library" "my-pkg" `shouldBe` expected

        it "renders the schema-v1 stdout program golden document" $ do
            expected <-
                readFile
                    "../../../specs/fixtures/scaffold-generator/haskell_program_required_capabilities.json"
            capabilityManifestContents "program" "build-helper" `shouldBe` expected

    describe "buildFileContents" $ do
        it "selects the Darwin CLT linker in the shape the validator accepts" $ do
            buildFileContents
                `shouldBe` unlines
                    [ "# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker."
                    , "# Scope the selection to Cabal and its children, keeping other packages intact."
                    , "if [ \"$(uname -s)\" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; if command -v cabal >/dev/null 2>&1; then cabal test all; else echo 'cabal not found -- skipping'; fi"
                    ]

    describe "buildWindowsFileContents" $ do
        it "is the real Cabal command, not a skip record, without the POSIX prefix" $ do
            buildWindowsFileContents `shouldBe` "cabal test all\n"
            buildWindowsFileContents `shouldNotSatisfy` containsText "echo"
            buildWindowsFileContents `shouldNotSatisfy` containsText "uname"
            buildWindowsFileContents `shouldNotSatisfy` containsText "DEVELOPER_DIR"

    describe "parseArgs" $ do
        it "parses the basic invocation" $ do
            parseArgs ["logic-wizard"]
                `shouldBe`
                Right
                    ( ParsedRun
                        defaultConfig
                            { configPackageName = Just "logic-wizard"
                            }
                    )

        it "parses optional flags" $ do
            parseArgs ["--type", "program", "--depends-on", "logic-gates,arithmetic", "build-helper"]
                `shouldBe`
                Right
                    ( ParsedRun
                        defaultConfig
                            { configPackageType = "program"
                            , configDependsOn = ["logic-gates", "arithmetic"]
                            , configPackageName = Just "build-helper"
                            }
                    )
