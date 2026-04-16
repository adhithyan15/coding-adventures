module ScaffoldGeneratorSpec (scaffoldGeneratorSpec) where

import Test.Hspec

import ScaffoldGenerator

scaffoldGeneratorSpec :: Spec
scaffoldGeneratorSpec = do
    describe "isKebabCase" $ do
        it "accepts valid names" $ do
            isKebabCase "logic-gates" `shouldBe` True

        it "rejects invalid names" $ do
            isKebabCase "LogicGates" `shouldBe` False
            isKebabCase "bad--name" `shouldBe` False

    describe "toModuleName" $ do
        it "converts kebab case to module case" $ do
            toModuleName "logic-gates" `shouldBe` "LogicGates"

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
