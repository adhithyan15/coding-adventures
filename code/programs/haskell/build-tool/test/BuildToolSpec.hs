module BuildToolSpec (buildToolSpec) where

import Test.Hspec

import BuildTool

buildToolSpec :: Spec
buildToolSpec = do
    describe "parseArgs" $ do
        it "parses the default run configuration" $ do
            parseArgs [] `shouldBe` Right (ParsedRun defaultConfig)

        it "parses supported flags" $ do
            parseArgs
                [ "--language"
                , "haskell"
                , "--force"
                , "--jobs"
                , "4"
                , "--emit-plan"
                ]
                `shouldBe`
                Right
                    ( ParsedRun
                        defaultConfig
                            { configLanguage = "haskell"
                            , configForce = True
                            , configJobs = Just 4
                            , configEmitPlan = True
                            }
                    )

        it "rejects unexpected positional args" $ do
            parseArgs ["oops"] `shouldBe` Left "unexpected positional argument: oops"

    describe "inferLanguage" $ do
        it "detects haskell package paths" $ do
            inferLanguage "/repo/code/packages/haskell/logic-gates" `shouldBe` "haskell"

        it "detects rust program paths" $ do
            inferLanguage "/repo/code/programs/rust/build-tool" `shouldBe` "rust"
