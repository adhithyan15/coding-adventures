{-# LANGUAGE OverloadedStrings #-}

module BuildToolSpec (buildToolSpec) where

import Control.Monad (forM_)
import Data.Aeson
    ( FromJSON(..)
    , Object
    , Value
    , eitherDecodeStrict'
    , withObject
    , (.:)
    , (.:?)
    )
import Data.Aeson.Types (Parser)
import qualified Data.ByteString as BS
import System.FilePath ((</>))

import Test.Hspec

import BuildTool

data TrackedArtifactFixture = TrackedArtifactFixture
    { fixtureUnicodeVersion :: String
    , fixtureEntries :: [TrackedArtifactEntry]
    , fixtureDiagnostics :: [TrackedArtifactDiagnostic]
    }

instance FromJSON TrackedArtifactFixture where
    parseJSON = withObject "tracked artifact fixture" $ \object -> do
        input <- object .: "input" :: Parser Object
        options <- input .: "options" :: Parser Object
        snapshot <- options .: "tracked_artifact_snapshot" :: Parser Object
        expected <- object .: "expected" :: Parser Object
        entryValues <- snapshot .: "entries" :: Parser [Value]
        diagnosticValues <- expected .: "diagnostics" :: Parser [Value]
        TrackedArtifactFixture
            <$> snapshot .: "unicode_version"
            <*> mapM parseTrackedEntry entryValues
            <*> mapM parseTrackedDiagnostic diagnosticValues

parseTrackedEntry :: Value -> Parser TrackedArtifactEntry
parseTrackedEntry = withObject "tracked artifact entry" $ \object ->
    TrackedArtifactEntry
        <$> object .: "ordinal"
        <*> object .: "path"
        <*> object .: "entry_kind"

parseTrackedDiagnostic :: Value -> Parser TrackedArtifactDiagnostic
parseTrackedDiagnostic = withObject "tracked artifact diagnostic" $ \object -> do
    details <- object .: "details" :: Parser Object
    TrackedArtifactDiagnostic
        <$> object .: "code"
        <*> object .: "severity"
        <*> object .: "path"
        <*> ( TrackedArtifactDiagnosticDetails
                <$> details .: "ordinal"
                <*> details .: "entry_kind"
                <*> details .:? "problem"
            )

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

        it "detects Dart package paths" $ do
            inferLanguage "/repo/code/packages/dart/logic-gates" `shouldBe` "dart"

    describe "tracked artifact validation" $ do
        forM_ trackedArtifactFixtureNames $ \fixtureName ->
            it ("consumes shared fixture " ++ fixtureName) $ do
                fixture <- loadTrackedArtifactFixture fixtureName
                validateTrackedArtifactSnapshot
                    (fixtureUnicodeVersion fixture)
                    (fixtureEntries fixture)
                    `shouldBe` Right (fixtureDiagnostics fixture)

        it "rejects Unicode-version drift before forcing entries" $ do
            validateTrackedArtifactSnapshot
                "15.1.0"
                (error "entries were forced")
                `shouldBe`
                Left "tracked artifact Unicode version must be 17.0.0"

        it "counts Unicode scalars at the 512 boundary" $ do
            let valid = TrackedArtifactEntry 1 (replicate 512 '\x1F600') "regular"
                tooLong = TrackedArtifactEntry 2 (replicate 513 '\x1F600') "regular"
            validateTrackedArtifactSnapshot trackedArtifactUnicodeVersion [valid]
                `shouldBe` Right []
            validateTrackedArtifactSnapshot trackedArtifactUnicodeVersion [tooLong]
                `shouldBe`
                Right
                    [ invalidDiagnostic 2 "regular" "TOO_LONG"
                    ]

        it "redacts hostile paths and preserves exact problem precedence" $ do
            let entries =
                    [ TrackedArtifactEntry 1 "../bad<" "regular"
                    , TrackedArtifactEntry 2 "safe/space /file" "symlink"
                    , TrackedArtifactEntry 3 "safe/bad<name" "reparse"
                    ]
            validateTrackedArtifactSnapshot trackedArtifactUnicodeVersion entries
                `shouldBe`
                Right
                    [ invalidDiagnostic 1 "regular" "UNSAFE_CHARACTER"
                    , invalidDiagnostic 3 "reparse" "UNSAFE_CHARACTER"
                    , invalidDiagnostic 2 "symlink" "TRAILING_DOT_OR_SPACE"
                    ]

        it "uses pinned Unicode 17 behavior for aliases and reserved basenames" $ do
            let outlinedNodeModules =
                    map
                        toEnum
                        [ 0x1CCE3, 0x1CCE4, 0x1CCD9, 0x1CCDA, 0x5F, 0x1CCE2
                        , 0x1CCE4, 0x1CCD9, 0x1CCEA, 0x1CCE1, 0x1CCDA, 0x1CCE8
                        ]
                entries =
                    [ TrackedArtifactEntry 1 (outlinedNodeModules ++ "/version.txt") "regular"
                    , TrackedArtifactEntry 2 "code/con\x0131n$.txt/file.cs" "reparse"
                    , TrackedArtifactEntry 3 "code/\x105D2\x0307/file.rs" "regular"
                    ]
            validateTrackedArtifactSnapshot trackedArtifactUnicodeVersion entries
                `shouldBe`
                Right
                    [ TrackedArtifactDiagnostic
                        "TRACKED_ARTIFACT_FORBIDDEN"
                        "error"
                        (outlinedNodeModules ++ "/version.txt")
                        (TrackedArtifactDiagnosticDetails 1 "regular" Nothing)
                    , invalidDiagnostic 3 "regular" "NON_NFC"
                    , invalidDiagnostic 2 "reparse" "RESERVED_BASENAME"
                    ]

        it "sorts diagnostic details canonically as strings" $ do
            let entries =
                    [ TrackedArtifactEntry 2 "bad<path" "regular"
                    , TrackedArtifactEntry 10 "bad<path" "regular"
                    ]
            validateTrackedArtifactSnapshot trackedArtifactUnicodeVersion entries
                `shouldBe`
                Right
                    [ invalidDiagnostic 10 "regular" "UNSAFE_CHARACTER"
                    , invalidDiagnostic 2 "regular" "UNSAFE_CHARACTER"
                    ]

trackedArtifactFixtureNames :: [FilePath]
trackedArtifactFixtureNames =
    [ "validation-tracked-artifacts-clean.json"
    , "validation-tracked-artifacts-forbidden.json"
    , "validation-tracked-artifacts-aliases.json"
    , "validation-tracked-artifacts-invalid.json"
    , "validation-tracked-artifacts-unicode-boundaries.json"
    ]

loadTrackedArtifactFixture :: FilePath -> IO TrackedArtifactFixture
loadTrackedArtifactFixture fixtureName = do
    maybeRoot <- findRepoRoot Nothing
    repoRoot <- maybe (fail "could not locate repository root for fixtures") pure maybeRoot
    let fixturePath =
            repoRoot
                </> "code"
                </> "specs"
                </> "fixtures"
                </> "build-tool-v1"
                </> "cases"
                </> fixtureName
    bytes <- BS.readFile fixturePath
    either (fail . ("invalid shared fixture: " ++)) pure (eitherDecodeStrict' bytes)

invalidDiagnostic :: Int -> String -> String -> TrackedArtifactDiagnostic
invalidDiagnostic ordinal entryKind problem =
    TrackedArtifactDiagnostic
        "TRACKED_ARTIFACT_PATH_INVALID"
        "error"
        "repository"
        (TrackedArtifactDiagnosticDetails ordinal entryKind (Just problem))
