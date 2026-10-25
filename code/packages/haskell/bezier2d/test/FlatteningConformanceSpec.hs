{-# LANGUAGE OverloadedStrings #-}

module FlatteningConformanceSpec (spec) where

import Bezier2D
import Control.Monad (forM_)
import Data.Aeson
import qualified Data.ByteString as ByteString
import Point2D (Point (..))
import System.Directory (doesFileExist, getCurrentDirectory, makeAbsolute)
import System.FilePath ((</>), takeDirectory)
import Test.Hspec

data Fixture = Fixture
    { fixtureVersion :: Int
    , fixtureAbsoluteTolerance :: Double
    , fixtureMaxDepth :: Int
    , fixtureMaxSubdivisions :: Int
    , fixtureCases :: [FixtureCase]
    }

data FixtureCase = FixtureCase
    { caseIdentifier :: String
    , caseDegree :: Int
    , caseControlPoints :: [[Double]]
    , caseTolerance :: Double
    , caseExpectedDisposition :: String
    , caseExpectedTermination :: Maybe String
    , caseWitnessT :: Maybe Double
    , caseExpectedWitness :: Maybe [Double]
    , caseExpectedWitnessDistance :: Maybe Double
    }

instance FromJSON Fixture where
    parseJSON = withObject "flattening fixture" $ \valueObject -> Fixture
        <$> valueObject .: "version"
        <*> valueObject .: "absolute_tolerance"
        <*> valueObject .: "max_depth"
        <*> valueObject .: "max_subdivisions"
        <*> valueObject .: "cases"

instance FromJSON FixtureCase where
    parseJSON = withObject "flattening case" $ \valueObject -> FixtureCase
        <$> valueObject .: "id"
        <*> valueObject .: "degree"
        <*> valueObject .: "control_points"
        <*> valueObject .: "tolerance"
        <*> valueObject .: "expected_disposition"
        <*> valueObject .:? "expected_termination"
        <*> valueObject .:? "witness_t"
        <*> valueObject .:? "expected_witness"
        <*> valueObject .:? "expected_witness_distance"

spec :: Spec
spec = beforeAll loadFixture $ do
    describe "G2D02 neutral flattening corpus" $ do
        it "pins the version and safety budgets" $ \fixture -> do
            fixtureVersion fixture `shouldBe` 1
            fixtureAbsoluteTolerance fixture `shouldBe` 1e-12
            fixtureMaxDepth fixture `shouldBe` 32
            fixtureMaxSubdivisions fixture `shouldBe` 65535
            length (fixtureCases fixture) `shouldBe` 9

        it "consumes every ordinary disposition dynamically" $ \fixture ->
            forM_ (filter ((== Nothing) . caseExpectedTermination) (fixtureCases fixture)) $ \testCase -> do
                let controls = map pointFromCoordinates (caseControlPoints testCase)
                    result = flatten testCase controls
                if caseExpectedDisposition testCase == "invalid-tolerance"
                    then result `shouldBe` Left InvalidTolerance
                    else do
                        points <- expectRight result
                        case (endpoints points, endpoints controls) of
                            (Just actualEndpoints, Just expectedEndpoints) ->
                                actualEndpoints `shouldBe` expectedEndpoints
                            _ -> expectationFailure (caseIdentifier testCase ++ " returned no points")
                        all pointIsFinite points `shouldBe` True
                        if caseExpectedDisposition testCase == "chord"
                            then length points `shouldBe` 2
                            else length points `shouldSatisfy` (> 2)
                        assertWitness testCase controls

        it "fails the pinned exhaustion case atomically" $ \fixture -> do
            let budgetCases = filter ((== Just "budget-error") . caseExpectedTermination) (fixtureCases fixture)
            length budgetCases `shouldBe` 1
            case budgetCases of
                [testCase] -> do
                    let controls = map pointFromCoordinates (caseControlPoints testCase)
                    flatten testCase controls `shouldBe` Left FlatteningBudgetExceeded
                _ -> expectationFailure "expected exactly one budget case"

    describe "fail-closed numeric boundaries" $ do
        it "rejects nonfinite controls before subdivision" $ \_ -> do
            let origin = Point 0.0 0.0
            toPolylineQuadratic
                (QuadraticBezier origin (Point (0.0 / 0.0) 1.0) (Point 1.0 0.0))
                0.1 `shouldBe` Left NonFiniteControlPoint
            toPolylineCubic
                (CubicBezier origin (Point 0.0 1.0) (Point (1.0 / 0.0) (-1.0)) origin)
                0.1 `shouldBe` Left NonFiniteControlPoint

        it "rejects finite inputs whose derived geometry overflows" $ \_ -> do
            let huge = encodeFloat (floatRadix (0.0 :: Double) - 1) (snd (floatRange (0.0 :: Double)) - 1)
                negative = Point (-huge) 0.0
                positive = Point huge 0.0
            toPolylineQuadratic
                (QuadraticBezier negative (Point 0.0 1.0) positive)
                0.1 `shouldBe` Left NonFiniteGeometry
            toPolylineCubic
                (CubicBezier negative (Point 0.0 1.0) (Point 0.0 (-1.0)) positive)
                0.1 `shouldBe` Left NonFiniteGeometry

flatten :: FixtureCase -> [Point] -> Either FlatteningError [Point]
flatten testCase controls = case (caseDegree testCase, controls) of
    (2, [first, control, end]) ->
        toPolylineQuadratic (QuadraticBezier first control end) (caseTolerance testCase)
    (3, [first, firstControl, secondControl, end]) ->
        toPolylineCubic (CubicBezier first firstControl secondControl end) (caseTolerance testCase)
    _ -> error ("unsupported fixture shape " ++ caseIdentifier testCase)

assertWitness :: FixtureCase -> [Point] -> Expectation
assertWitness testCase controls = case
    (caseWitnessT testCase, caseExpectedWitness testCase, caseExpectedWitnessDistance testCase) of
    (Just amount, Just expectedCoordinates, Just expectedDistance) -> do
        let actual = case (caseDegree testCase, controls) of
                (2, [first, control, end]) ->
                    evaluateQuadratic (QuadraticBezier first control end) amount
                (3, [first, firstControl, secondControl, end]) ->
                    evaluateCubic (CubicBezier first firstControl secondControl end) amount
                _ -> error "unsupported witness shape"
            expected = pointFromCoordinates expectedCoordinates
        pointX actual `shouldSatisfy` closeTo (pointX expected)
        pointY actual `shouldSatisfy` closeTo (pointY expected)
        case endpoints controls of
            Just (start, end) -> distanceToSegmentForTest actual start end
                `shouldSatisfy` closeTo expectedDistance
            Nothing -> expectationFailure "fixture controls must not be empty"
    _ -> expectationFailure (caseIdentifier testCase ++ " is missing its witness")

distanceToSegmentForTest :: Point -> Point -> Point -> Double
distanceToSegmentForTest point start end
    | lengthSquared == 0.0 = distance point start
    | otherwise = distance point nearest
  where
    deltaX = pointX end - pointX start
    deltaY = pointY end - pointY start
    lengthSquared = deltaX * deltaX + deltaY * deltaY
    projection = max 0.0 (min 1.0
        (((pointX point - pointX start) * deltaX
            + (pointY point - pointY start) * deltaY) / lengthSquared))
    nearest = Point
        (pointX start + projection * deltaX)
        (pointY start + projection * deltaY)

distance :: Point -> Point -> Double
distance left right = sqrt
    ((pointX left - pointX right) ^ (2 :: Int)
        + (pointY left - pointY right) ^ (2 :: Int))

closeTo :: Double -> Double -> Bool
closeTo expected actual = abs (actual - expected) <= 1e-12

pointFromCoordinates :: [Double] -> Point
pointFromCoordinates [x, y] = Point x y
pointFromCoordinates _ = error "fixture point must have two coordinates"

pointIsFinite :: Point -> Bool
pointIsFinite point = finite (pointX point) && finite (pointY point)

endpoints :: [value] -> Maybe (value, value)
endpoints [] = Nothing
endpoints (first : rest) = Just (first, foldl (\_ value -> value) first rest)

finite :: Double -> Bool
finite value = not (isNaN value || isInfinite value)

expectRight :: (Show error) => Either error value -> IO value
expectRight result = case result of
    Right value -> pure value
    Left failure -> expectationFailure ("expected Right, got Left " ++ show failure) >> fail "unreachable"

loadFixture :: IO Fixture
loadFixture = do
    start <- makeAbsolute =<< getCurrentDirectory
    path <- findFixture start
    bytes <- ByteString.readFile path
    if ByteString.length bytes > 65536
        then fail "G2D02 flattening fixture exceeds the bounded reader limit"
        else case eitherDecodeStrict' bytes of
            Left failure -> fail ("invalid G2D02 flattening fixture: " ++ failure)
            Right fixture -> pure fixture

findFixture :: FilePath -> IO FilePath
findFixture directory = do
    let candidate = directory </> "code" </> "specs" </> "fixtures"
            </> "bezier2d-flattening-v1" </> "cases.json"
    found <- doesFileExist candidate
    if found
        then pure candidate
        else if takeDirectory directory == directory
            then fail "could not locate the checked-in G2D02 flattening corpus"
            else findFixture (takeDirectory directory)
