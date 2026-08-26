{-# LANGUAGE OverloadedStrings #-}

module BuildToolSpec (buildToolSpec) where

import Control.Monad (forM_)
import Data.Char (chr)
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

data OrphanFixture = OrphanFixture
    { orphanFixtureSnapshot :: OrphanSnapshot
    , orphanFixtureResult :: OrphanValidationResult
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

instance FromJSON OrphanFixture where
    parseJSON = withObject "orphan fixture" $ \object -> do
        input <- object .: "input" :: Parser Object
        options <- input .: "options" :: Parser Object
        snapshot <- options .: "orphan_snapshot" :: Parser Object
        expected <- object .: "expected" :: Parser Object
        expectedResult <- expected .: "result" :: Parser Object
        manifestValues <- snapshot .: "manifests" :: Parser [Value]
        buildFileValues <- snapshot .: "build_files" :: Parser [Value]
        exemptionValues <- snapshot .: "exemptions" :: Parser [Value]
        diagnosticValues <- expected .: "diagnostics" :: Parser [Value]
        parsedSnapshot <-
            OrphanSnapshot
                <$> snapshot .: "directories"
                <*> mapM parseOrphanManifest manifestValues
                <*> mapM parseOrphanBuildFile buildFileValues
                <*> mapM parseOrphanExemption exemptionValues
        parsedResult <-
            OrphanValidationResult
                <$> expectedResult .: "valid"
                <*> expectedResult .: "diagnostic_codes"
                <*> expectedResult .: "pending_exemption_count"
                <*> mapM parseOrphanDiagnostic diagnosticValues
        pure (OrphanFixture parsedSnapshot parsedResult)

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

parseOrphanManifest :: Value -> Parser OrphanManifest
parseOrphanManifest = withObject "orphan manifest" $ \object ->
    OrphanManifest
        <$> object .: "path"
        <*> object .: "kind"

parseOrphanBuildFile :: Value -> Parser OrphanBuildFile
parseOrphanBuildFile = withObject "orphan BUILD" $ \object ->
    OrphanBuildFile
        <$> object .: "path"
        <*> object .: "state"

parseOrphanExemption :: Value -> Parser OrphanExemption
parseOrphanExemption = withObject "orphan exemption" $ \object ->
    OrphanExemption
        <$> object .: "line"
        <*> object .: "kind"
        <*> object .: "path"
        <*> object .:? "reason"

parseOrphanDiagnostic :: Value -> Parser OrphanDiagnostic
parseOrphanDiagnostic = withObject "orphan diagnostic" $ \object -> do
    code <- object .: "code"
    severity <- object .: "severity"
    path <- object .: "path"
    details <- object .: "details" :: Parser Object
    parsedDetails <-
        case code of
            "ORPHAN_CRATE_EMPTY_BUILD" ->
                OrphanCrateDiagnosticDetails
                    <$> details .:? "build_path"
                    <*> details .: "manifest_kind"
            "ORPHAN_CRATE_UNLISTED" ->
                OrphanCrateDiagnosticDetails
                    <$> details .:? "build_path"
                    <*> details .: "manifest_kind"
            "ORPHAN_EXEMPTION_INVALID" ->
                OrphanInvalidExemptionDetails
                    <$> details .: "line"
                    <*> details .: "problem"
            "ORPHAN_EXEMPTION_STALE" ->
                OrphanStaleExemptionDetails
                    <$> details .: "entry_path"
                    <*> details .: "kind"
                    <*> details .: "line"
                    <*> details .: "problem"
            _ -> fail ("unknown orphan diagnostic code: " ++ code)
    pure (OrphanDiagnostic code severity path parsedDetails)

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

    describe "orphan crate validation" $ do
        forM_ orphanFixtureNames $ \fixtureName ->
            it ("consumes shared fixture " ++ fixtureName) $ do
                fixture <- loadOrphanFixture fixtureName
                validateOrphanCrateSnapshot (orphanFixtureSnapshot fixture)
                    `shouldBe` orphanFixtureResult fixture

        it "redacts hostile exemption paths, including surrogate Chars" $ do
            let unsafePaths =
                    [ ""
                    , replicate 513 '\x1F600'
                    , "/absolute/secret-project"
                    , "C:/host/secret-project"
                    , "code/packages/rust/bad<name>"
                    , "code/packages/rust/trailing."
                    , "code/packages/rust/CON"
                    , "code/packages/rust/" ++ [chr 0xD800]
                    ]
            forM_ unsafePaths $ \unsafePath -> do
                let result =
                        validateOrphanCrateSnapshot
                            ( OrphanSnapshot
                                ["code/packages/rust/demo"]
                                [OrphanManifest "code/packages/rust/demo" "package"]
                                []
                                [OrphanExemption 7 "PENDING" unsafePath (Just "not allowed")]
                            )
                    invalidDiagnostics =
                        [ diagnostic
                        | diagnostic <- orphanResultDiagnostics result
                        , orphanDiagnosticCode diagnostic == "ORPHAN_EXEMPTION_INVALID"
                        ]
                invalidDiagnostics
                    `shouldBe`
                    [ OrphanDiagnostic
                        "ORPHAN_EXEMPTION_INVALID"
                        "error"
                        "code/BUILD-EXEMPTIONS"
                        (OrphanInvalidExemptionDetails 7 "PATH_UNSAFE")
                    ]

        it "uses the exact Python blank-reason set" $ do
            let result =
                    validateOrphanCrateSnapshot
                        ( OrphanSnapshot
                            ["code/packages/rust/blank", "code/packages/rust/bom"]
                            [ OrphanManifest "code/packages/rust/blank" "package"
                            , OrphanManifest "code/packages/rust/bom" "package"
                            ]
                            []
                            [ OrphanExemption 7 "PENDING" "code/packages/rust/blank" (Just "\x1C")
                            , OrphanExemption 8 "PENDING" "code/packages/rust/bom" (Just "\xFEFF")
                            ]
                        )
            orphanResultPendingExemptionCount result `shouldBe` 1
            orphanResultDiagnosticCodes result
                `shouldBe` ["ORPHAN_CRATE_UNLISTED", "ORPHAN_EXEMPTION_INVALID"]
            orphanResultDiagnostics result
                `shouldContain`
                [ OrphanDiagnostic
                    "ORPHAN_EXEMPTION_INVALID"
                    "error"
                    "code/BUILD-EXEMPTIONS"
                    (OrphanInvalidExemptionDetails 7 "REASON_MISSING")
                ]

        it "rejects missing, oversized, and non-scalar exemption reasons" $ do
            let invalidReasons = [Nothing, Just (replicate 4097 'x'), Just [chr 0xD800]]
            forM_ invalidReasons $ \invalidReason -> do
                let result =
                        validateOrphanCrateSnapshot
                            ( OrphanSnapshot
                                ["code/packages/rust/demo"]
                                [OrphanManifest "code/packages/rust/demo" "package"]
                                []
                                [OrphanExemption 7 "PENDING" "code/packages/rust/demo" invalidReason]
                            )
                orphanResultPendingExemptionCount result `shouldBe` 0
                orphanResultDiagnosticCodes result
                    `shouldBe` ["ORPHAN_CRATE_UNLISTED", "ORPHAN_EXEMPTION_INVALID"]

        it "chooses the closest empty BUILD and fixed filename rank" $ do
            let result =
                    validateOrphanCrateSnapshot
                        ( OrphanSnapshot
                            ["code/packages/rust/demo/child"]
                            [OrphanManifest "code/packages/rust/demo/child" "package"]
                            [ OrphanBuildFile "code/packages/rust/BUILD" "empty"
                            , OrphanBuildFile "code/packages/rust/demo/BUILD_linux" "empty"
                            , OrphanBuildFile "code/packages/rust/demo/BUILD" "empty"
                            , OrphanBuildFile "code/packages/rust/demo2/BUILD" "runnable"
                            ]
                            []
                        )
            orphanResultDiagnostics result
                `shouldBe`
                [ OrphanDiagnostic
                    "ORPHAN_CRATE_EMPTY_BUILD"
                    "error"
                    "code/packages/rust/demo/child"
                    (OrphanCrateDiagnosticDetails (Just "code/packages/rust/demo/BUILD") "package")
                ]

        it "reserves NFC full-fold identities before field precedence" $ do
            let result =
                    validateOrphanCrateSnapshot
                        ( OrphanSnapshot
                            ["code/packages/rust/Stra\x00DF\&e"]
                            [OrphanManifest "code/packages/rust/Stra\x00DF\&e" "package"]
                            []
                            [ OrphanExemption 7 "UNKNOWN" "code/packages/rust/Stra\x00DF\&e" (Just "first")
                            , OrphanExemption 8 "PENDING" "CODE/PACKAGES/RUST/STRASSE" (Just "duplicate")
                            ]
                        )
                invalidDetails =
                    [ details
                    | OrphanDiagnostic
                        { orphanDiagnosticCode = "ORPHAN_EXEMPTION_INVALID"
                        , orphanDiagnosticDetails = details
                        } <- orphanResultDiagnostics result
                    ]
            invalidDetails
                `shouldBe`
                [ OrphanInvalidExemptionDetails 7 "UNKNOWN_KIND"
                , OrphanInvalidExemptionDetails 8 "DUPLICATE_PATH"
                ]

        it "uses canonical ASCII JSON ordering for Unicode details" $ do
            let accented = "code/packages/rust/\x00E9"
                emoji = "code/packages/rust/\x1F600"
                result =
                    validateOrphanCrateSnapshot
                        ( OrphanSnapshot
                            []
                            []
                            []
                            [ OrphanExemption 9 "EXCLUDED" "code/packages/rust/z" (Just "removed")
                            , OrphanExemption 8 "EXCLUDED" emoji (Just "removed")
                            , OrphanExemption 7 "EXCLUDED" accented (Just "removed")
                            ]
                        )
                stalePaths =
                    [ entryPath
                    | OrphanDiagnostic
                        { orphanDiagnosticDetails =
                            OrphanStaleExemptionDetails entryPath _ _ _
                        } <- orphanResultDiagnostics result
                    ]
            stalePaths `shouldBe` [accented, emoji, "code/packages/rust/z"]

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

orphanFixtureNames :: [FilePath]
orphanFixtureNames =
    [ "validation-orphan-crates-clean.json"
    , "validation-orphan-crates-unlisted.json"
    , "validation-orphan-exemptions-invalid.json"
    , "validation-orphan-exemptions-stale.json"
    ]

loadOrphanFixture :: FilePath -> IO OrphanFixture
loadOrphanFixture fixtureName = do
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
