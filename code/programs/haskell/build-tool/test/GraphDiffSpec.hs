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
    , GraphInput
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
        mapM_ (assertGraphFixture . (fixtureRoot </> "cases" </>)) names

    it "dynamically consumes the exact twelve diff-selection cases" $ do
        names <- caseNames "diff-selection-"
        names `shouldBe` diffRoster
        boundaryBytes <- BS.readFile (fixtureRoot </> "repository-source-input-boundary.json")
        boundary <- decodeOrFail boundaryBytes
        mapM_ (assertDiffFixture boundary . (fixtureRoot </> "cases" </>)) names

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

assertGraphFixture :: FilePath -> Expectation
assertGraphFixture path = do
    fixture <- BS.readFile path >>= decodeOrFail
    fixtureDomain fixture `shouldBe` "graph"
    graphInput <- case parseEither parseJSON (fixtureInput fixture) of
        Left message -> expectationFailure message >> fail message
        Right value -> pure (value :: GraphInput)
    assertOutcome fixture (toJSON <$> evaluateGraph graphInput)

assertDiffFixture :: RepositoryBoundary -> FilePath -> Expectation
assertDiffFixture boundary path = do
    fixture <- BS.readFile path >>= decodeOrFail
    fixtureDomain fixture `shouldBe` "diff_selection"
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
