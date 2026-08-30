{-# LANGUAGE OverloadedStrings #-}

module ToolchainDetectionSpec (toolchainDetectionSpec) where

import Control.Exception (evaluate)
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
import Data.List (isPrefixOf, sort)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Maybe (fromMaybe)
import System.Directory (listDirectory)
import System.FilePath ((</>))
import Test.Hspec

import ToolchainDetection

fixtureRoot :: FilePath
fixtureRoot = "../../../specs/fixtures/build-tool-v1/cases"

data Fixture = Fixture
    { fixtureId :: String
    , fixturePlatform :: String
    , fixtureForceFull :: Bool
    , fixturePackages :: [ToolchainPackage]
    , fixtureScheduled :: Maybe [String]
    , fixtureForced :: [String]
    , fixtureOutcome :: String
    , fixtureFlags :: Map String Bool
    , fixtureDiagnostics :: [ToolchainDiagnostic]
    }

instance FromJSON Fixture where
    parseJSON = withObject "toolchain fixture" $ \object -> do
        input <- object .: "input" :: Parser Object
        options <- input .: "options" :: Parser Object
        expected <- object .: "expected" :: Parser Object
        result <- expected .: "result" :: Parser Object
        packageValues <- options .: "packages" :: Parser [Value]
        diagnosticValues <- expected .: "diagnostics" :: Parser [Value]
        Fixture
            <$> object .: "id"
            <*> options .: "platform"
            <*> options .: "force_full"
            <*> mapM parsePackage packageValues
            <*> options .:? "scheduled_packages"
            <*> options .: "forced_toolchains"
            <*> expected .: "outcome"
            <*> (fromMaybe Map.empty <$> result .:? "toolchains")
            <*> mapM parseDiagnostic diagnosticValues

parsePackage :: Value -> Parser ToolchainPackage
parsePackage = withObject "toolchain package" $ \object ->
    ToolchainPackage
        <$> object .: "name"
        <*> object .: "language"
        <*> object .: "build_files"

parseDiagnostic :: Value -> Parser ToolchainDiagnostic
parseDiagnostic = withObject "toolchain diagnostic" $ \object ->
    ToolchainDiagnostic
        <$> object .: "code"
        <*> object .: "severity"
        <*> object .:? "package"

toolchainDetectionSpec :: Spec
toolchainDetectionSpec = describe "toolchain declaration detection" $ do
    it "independently consumes every neutral toolchain fixture" $ do
        entries <- listDirectory fixtureRoot
        let fixtures = sort (filter isToolchainFixture entries)
        fixtures `shouldBe` expectedFixtureNames
        mapM_ assertFixture fixtures

    it "unions selected declarations and forced workflow toolchains" $ do
        let packages =
                [ ToolchainPackage
                    "rust/selected"
                    "rust"
                    (Map.fromList [("BUILD", "# needs-toolchain: python\r\n")])
                , ToolchainPackage
                    "go/unscheduled"
                    "go"
                    (Map.fromList [("BUILD", "# needs-toolchain: java\n")])
                ]
            result =
                evaluateToolchainSnapshot
                    "linux"
                    False
                    packages
                    (Just ["rust/selected"])
                    ["kotlin"]
        toolchainOutcome result `shouldBe` "ok"
        Map.lookup "rust" (toolchainFlags result) `shouldBe` Just True
        Map.lookup "python" (toolchainFlags result) `shouldBe` Just True
        Map.lookup "kotlin" (toolchainFlags result) `shouldBe` Just True
        Map.lookup "go" (toolchainFlags result) `shouldBe` Just False
        Map.lookup "java" (toolchainFlags result) `shouldBe` Just False

    it "strips carriage return only from a CRLF terminator" $ do
        parseExtraToolchains "# needs-toolchain: python\r\n" `shouldBe` ["python"]
        parseExtraToolchains "# needs-toolchain: ruby\r" `shouldBe` []
        parseExtraToolchains "# needs-toolchain: lua\r  " `shouldBe` []
        parseExtraToolchains "# needs-toolchain: perl\r\t\n" `shouldBe` []
        parseExtraToolchains "# needs-toolchain: swift\r\r\n" `shouldBe` []

    it "rejects per-file byte and logical-line limit overruns" $ do
        let byteOversized = replicate 32769 '\233'
            lineOversized = replicate 4096 '\n'
        parseExtraToolchains byteOversized `shouldBe` []
        parseExtraToolchains lineOversized `shouldBe` []
        evaluateSnapshotWith [singleBuild byteOversized]
            `shouldThrow` errorCall "toolchain BUILD snapshot exceeds its per-file resource ceiling"
        evaluateSnapshotWith [singleBuild lineOversized]
            `shouldThrow` errorCall "toolchain BUILD snapshot exceeds its per-file resource ceiling"

    it "rejects aggregate snapshot limit overruns" $ do
        let buildFiles = Map.fromList [("BUILD_" ++ show index, replicate 65536 'x') | index <- [0 .. 16]]
            package = ToolchainPackage "rust/app" "rust" buildFiles
        evaluateSnapshotWith [package]
            `shouldThrow` errorCall "toolchain BUILD snapshot exceeds its aggregate resource ceiling"

evaluateSnapshotWith :: [ToolchainPackage] -> IO ToolchainResult
evaluateSnapshotWith packages =
    evaluate (evaluateToolchainSnapshot "linux" False packages Nothing [])

singleBuild :: String -> ToolchainPackage
singleBuild content =
    ToolchainPackage "rust/app" "rust" (Map.singleton "BUILD" content)

assertFixture :: FilePath -> Expectation
assertFixture filename = do
    bytes <- BS.readFile (fixtureRoot </> filename)
    fixture <- case eitherDecodeStrict' bytes of
        Left err -> expectationFailure err >> fail err
        Right parsed -> pure parsed
    let actual =
            evaluateToolchainSnapshot
                (fixturePlatform fixture)
                (fixtureForceFull fixture)
                (fixturePackages fixture)
                (fixtureScheduled fixture)
                (fixtureForced fixture)
    expectFixtureField fixture "outcome" (toolchainOutcome actual) (fixtureOutcome fixture)
    expectFixtureField fixture "toolchains" (toolchainFlags actual) (fixtureFlags fixture)
    expectFixtureField fixture "diagnostics" (toolchainDiagnostics actual) (fixtureDiagnostics fixture)

expectFixtureField :: (Eq value, Show value) => Fixture -> String -> value -> value -> Expectation
expectFixtureField fixture field actual expected
    | actual == expected = pure ()
    | otherwise =
        expectationFailure
            (fixtureId fixture ++ " " ++ field ++ ": expected " ++ show expected ++ ", got " ++ show actual)

isToolchainFixture :: FilePath -> Bool
isToolchainFixture filename =
    "toolchain-detection-" `isPrefixOf` filename && reverse ".json" `isPrefixOf` reverse filename

expectedFixtureNames :: [FilePath]
expectedFixtureNames =
    [ "toolchain-detection-affected-only.json"
    , "toolchain-detection-crlf-grammar.json"
    , "toolchain-detection-declarations.json"
    , "toolchain-detection-empty.json"
    , "toolchain-detection-force-full.json"
    , "toolchain-detection-null-all.json"
    , "toolchain-detection-platform-darwin.json"
    , "toolchain-detection-platform-linux.json"
    , "toolchain-detection-platform-windows.json"
    , "toolchain-detection-shared.json"
    , "toolchain-detection-unsupported.json"
    ]
