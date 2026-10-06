{-# LANGUAGE OverloadedStrings #-}

module GraphDiffSpec (graphDiffSpec) where

import Data.Aeson
    ( FromJSON(..)
    , Object
    , Value
    , eitherDecodeStrict'
    , toJSON
    , withObject
    , (.:)
    )
import Data.Aeson.Types (Parser, parseEither)
import qualified Data.ByteString as BS
import Data.List (isPrefixOf, sort)
import System.Directory (listDirectory)
import System.FilePath ((</>))
import Test.Hspec

import GraphDiffCore
    ( DiffInput(..)
    , Edge(..)
    , GraphInput(..)
    , Package(..)
    , RepositoryBoundary
    , evaluateDiffSelection
    , evaluateGraph
    )

-- The native test loader may read the checked corpus. The production
-- operations receive already-decoded values and have no such authority.
fixtureRoot :: FilePath
fixtureRoot = "../../../specs/fixtures/build-tool-v1"

data Fixture = Fixture
    { fixtureId :: String
    , fixtureDomain :: String
    , fixtureInput :: Value
    , fixtureOutcome :: String
    , fixtureResult :: Value
    , fixtureDiagnosticCodes :: [String]
    }

instance FromJSON Fixture where
    parseJSON = withObject "graph/diff fixture" $ \object -> do
        expected <- object .: "expected" :: Parser Object
        diagnosticValues <- expected .: "diagnostics" :: Parser [Value]
        codes <- mapM parseDiagnosticCode diagnosticValues
        Fixture
            <$> object .: "id"
            <*> object .: "domain"
            <*> object .: "input"
            <*> expected .: "outcome"
            <*> expected .: "result"
            <*> pure codes

parseDiagnosticCode :: Value -> Parser String
parseDiagnosticCode = withObject "diagnostic" (.: "code")

graphDiffSpec :: Spec
graphDiffSpec = describe "process-free graph and diff selection" $ do
    it "dynamically consumes the exact eight graph cases" $ do
        names <- caseNames "graph-"
        names `shouldBe` graphRoster
        length graphIdRoster `shouldBe` length names
        mapM_ (\(name, caseId) -> assertGraphFixture caseId
            (fixtureRoot </> "cases" </> name)) (zip names graphIdRoster)

    it "dynamically consumes the exact twelve diff-selection cases" $ do
        names <- caseNames "diff-selection-"
        names `shouldBe` diffRoster
        length diffIdRoster `shouldBe` length names
        boundaryBytes <- BS.readFile (fixtureRoot </> "repository-source-input-boundary.json")
        boundary <- decodeOrFail boundaryBytes
        mapM_ (\(name, caseId) -> assertDiffFixture boundary caseId
            (fixtureRoot </> "cases" </> name)) (zip names diffIdRoster)

    it "rejects undeclared, duplicate, and cyclic edges before partial output" $ do
        evaluateGraph (GraphInput ["fixture/a"] [Edge "fixture/a" "fixture/b"])
            `shouldBeError` "GRAPH_EDGE_UNKNOWN"
        evaluateGraph (GraphInput ["fixture/a", "fixture/b"]
            [Edge "fixture/a" "fixture/b", Edge "fixture/a" "fixture/b"])
            `shouldBeError` "GRAPH_EDGE_DUPLICATE"
        evaluateGraph (GraphInput ["fixture/a", "fixture/b"]
            [Edge "fixture/a" "fixture/b", Edge "fixture/b" "fixture/a"])
            `shouldBeError` "GRAPH_CYCLE"

    it "validates portable roots, glob syntax, and boundary authority first" $ do
        let base = DiffInput [Package "fixture/a" "p" "strict_globs" ["*.hs"]]
                [] [] "error" ["p/Main.hs"] Nothing Nothing
        evaluateDiffSelection (base {diffPackages =
            [Package "fixture/a" "CON" "strict_globs" ["*.hs"]]})
            `shouldBeError` "DIFF_PATH_INVALID"
        evaluateDiffSelection (base {diffPackages =
            [Package "fixture/a" "p" "strict_globs" ["[z-a].hs"]]})
            `shouldBeError` "DIFF_GLOB_INVALID"
        evaluateDiffSelection (base {diffPackages =
            [Package "fixture/a" "p" "strict_globs" ["[&-&&].hs"]]})
            `shouldBeError` "DIFF_GLOB_INVALID"
        evaluateDiffSelection (base {diffBoundarySha256 = Just (replicate 64 '0')})
            `shouldBeError` "DIFF_BOUNDARY_DIGEST_MISMATCH"

shouldBeError :: Show a => Either String a -> String -> Expectation
shouldBeError actual expected = case actual of
    Left code -> code `shouldBe` expected
    Right value -> expectationFailure ("unexpected success: " ++ show value)

caseNames :: String -> IO [FilePath]
caseNames prefix = do
    entries <- listDirectory (fixtureRoot </> "cases")
    pure (sort [name | name <- entries, prefix `isPrefixOf` name, ".json" `isSuffixOf` name])

isSuffixOf :: Eq a => [a] -> [a] -> Bool
isSuffixOf suffix value = reverse suffix `isPrefixOf` reverse value

decodeOrFail :: FromJSON a => BS.ByteString -> IO a
decodeOrFail bytes = case eitherDecodeStrict' bytes of
    Left message -> expectationFailure message >> fail message
    Right value -> pure value

assertGraphFixture :: String -> FilePath -> Expectation
assertGraphFixture caseId path = do
    fixture <- BS.readFile path >>= decodeOrFail
    fixtureDomain fixture `shouldBe` "graph"
    fixtureId fixture `shouldBe` caseId
    graphInput <- case parseEither parseJSON (fixtureInput fixture) of
        Left message -> expectationFailure message >> fail message
        Right value -> pure (value :: GraphInput)
    assertOutcome fixture (toJSON <$> evaluateGraph graphInput)

assertDiffFixture :: RepositoryBoundary -> String -> FilePath -> Expectation
assertDiffFixture boundary caseId path = do
    fixture <- BS.readFile path >>= decodeOrFail
    fixtureDomain fixture `shouldBe` "diff_selection"
    fixtureId fixture `shouldBe` caseId
    decoded <- case parseEither parseJSON (fixtureInput fixture) of
        Left message -> expectationFailure message >> fail message
        Right value -> pure (value :: DiffInput)
    let input = case diffBoundarySha256 decoded of
            Nothing -> decoded
            Just _ -> decoded {diffBoundary = Just boundary}
    assertOutcome fixture (toJSON <$> evaluateDiffSelection input)

assertOutcome :: Fixture -> Either String Value -> Expectation
assertOutcome fixture actual = case actual of
    Left code -> do
        fixtureOutcome fixture `shouldBe` "error"
        fixtureResult fixture `shouldBe` toJSON (mempty :: Object)
        fixtureDiagnosticCodes fixture `shouldBe` [code]
    Right result -> do
        fixtureOutcome fixture `shouldBe` "ok"
        fixtureDiagnosticCodes fixture `shouldBe` []
        result `shouldBe` fixtureResult fixture

graphRoster :: [FilePath]
graphRoster =
    [ "graph-canonical-edge-order.json"
    , "graph-chain.json"
    , "graph-cycle.json"
    , "graph-diamond.json"
    , "graph-empty.json"
    , "graph-isolated.json"
    , "graph-multiple-components.json"
    , "graph-partial-cycle-no-output.json"
    ]

diffRoster :: [FilePath]
diffRoster =
    [ "diff-selection-exact-build-fronts.json"
    , "diff-selection-forced-package.json"
    , "diff-selection-known-unmatched-near-build.json"
    , "diff-selection-match-work-at-limit.json"
    , "diff-selection-match-work-over-limit.json"
    , "diff-selection-package-prefix.json"
    , "diff-selection-repository-boundary.json"
    , "diff-selection-shared-input-multiconsumer.json"
    , "diff-selection-strict-glob-character-classes.json"
    , "diff-selection-transitive.json"
    , "diff-selection-unknown-all.json"
    , "diff-selection-unknown-error.json"
    ]

graphIdRoster :: [String]
graphIdRoster =
    [ "graph/canonical-edge-order", "graph/chain", "graph/cycle"
    , "graph/diamond", "graph/empty", "graph/isolated"
    , "graph/multiple-components", "graph/partial-cycle-no-output"
    ]

diffIdRoster :: [String]
diffIdRoster =
    [ "diff-selection/exact-build-fronts"
    , "diff-selection/forced-package"
    , "diff-selection/known-unmatched-near-build"
    , "diff-selection/match-work-at-limit"
    , "diff-selection/match-work-over-limit"
    , "diff-selection/package-prefix"
    , "diff-selection/repository-boundary-reverse-index"
    , "diff-selection/shared-input-multiconsumer"
    , "diff-selection/strict-glob-character-classes"
    , "diff-selection/transitive-package-change"
    , "diff-selection/unknown-path-all"
    , "diff-selection/unknown-path-error"
    ]
