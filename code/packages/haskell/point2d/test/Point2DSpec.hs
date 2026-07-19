module Point2DSpec (spec) where

import Point2D
import Prelude hiding (negate, subtract)
import Test.Hspec
import qualified Trig

epsilon :: Double
epsilon = 1e-9

shouldApprox :: Double -> Double -> Expectation
shouldApprox actual expected = abs (actual - expected) `shouldSatisfy` (< epsilon)

shouldBePoint :: Point -> Point -> Expectation
shouldBePoint actual expected = do
    pointX actual `shouldApprox` pointX expected
    pointY actual `shouldApprox` pointY expected

shouldBeRect :: Rect -> Rect -> Expectation
shouldBeRect actual expected = do
    rectX actual `shouldApprox` rectX expected
    rectY actual `shouldApprox` rectY expected
    rectWidth actual `shouldApprox` rectWidth expected
    rectHeight actual `shouldApprox` rectHeight expected

spec :: Spec
spec = do
    describe "Point construction and arithmetic" $ do
        it "constructs the origin" $
            origin `shouldBe` Point 0.0 0.0
        it "stores coordinates" $ do
            pointX (Point 3.0 (-5.0)) `shouldBe` 3.0
            pointY (Point 3.0 (-5.0)) `shouldBe` (-5.0)
        it "adds component by component" $
            add (Point 1.0 2.0) (Point 3.0 4.0) `shouldBe` Point 4.0 6.0
        it "subtracts component by component" $
            subtract (Point 5.0 7.0) (Point 2.0 3.0) `shouldBe` Point 3.0 4.0
        it "scales by positive, zero, and negative factors" $ do
            scale (Point 3.0 4.0) 2.0 `shouldBePoint` Point 6.0 8.0
            scale (Point 3.0 4.0) 0.0 `shouldBePoint` origin
            scale (Point 3.0 4.0) (-1.0) `shouldBePoint` Point (-3.0) (-4.0)
        it "negates both coordinates" $
            negate (Point 3.0 (-4.0)) `shouldBe` Point (-3.0) 4.0

    describe "Point vector geometry" $ do
        it "computes perpendicular and parallel dot products" $ do
            dot (Point 1.0 0.0) (Point 0.0 1.0) `shouldBe` 0.0
            dot (Point 3.0 0.0) (Point 5.0 0.0) `shouldBe` 15.0
        it "computes signed cross products" $ do
            cross (Point 1.0 0.0) (Point 0.0 1.0) `shouldBe` 1.0
            cross (Point 0.0 1.0) (Point 1.0 0.0) `shouldBe` (-1.0)
        it "computes magnitude and squared magnitude" $ do
            magnitude (Point 3.0 4.0) `shouldApprox` 5.0
            magnitude origin `shouldBe` 0.0
            magnitudeSquared (Point 3.0 4.0) `shouldBe` 25.0
        it "normalizes non-zero vectors" $ do
            let unit = normalize (Point 3.0 4.0)
            unit `shouldBePoint` Point 0.6 0.8
            magnitude unit `shouldApprox` 1.0
        it "normalizes zero and near-zero vectors to the origin" $ do
            normalize origin `shouldBe` origin
            normalize (Point 1e-14 0.0) `shouldBe` origin
        it "computes distance and squared distance" $ do
            distance origin (Point 3.0 4.0) `shouldApprox` 5.0
            distanceSquared origin (Point 3.0 4.0) `shouldBe` 25.0
        it "interpolates endpoints, midpoints, and extrapolation" $ do
            let start = Point 1.0 2.0
                end = Point 5.0 6.0
            lerp start end 0.0 `shouldBePoint` start
            lerp start end 1.0 `shouldBePoint` end
            lerp start end 0.5 `shouldBePoint` Point 3.0 4.0
            lerp start end 2.0 `shouldBePoint` Point 9.0 10.0
        it "rotates counterclockwise and negates after two rotations" $ do
            perpendicular (Point 1.0 0.0) `shouldBePoint` Point 0.0 1.0
            perpendicular (Point 0.0 1.0) `shouldBePoint` Point (-1.0) 0.0
            perpendicular (perpendicular (Point 3.0 4.0))
                `shouldBePoint` negate (Point 3.0 4.0)
        it "returns angles on every axis" $ do
            angle (Point 1.0 0.0) `shouldApprox` 0.0
            angle (Point 0.0 1.0) `shouldApprox` Trig.halfPi
            abs (angle (Point (-1.0) 0.0)) `shouldApprox` Trig.piValue
            angle (Point 0.0 (-1.0)) `shouldApprox` (-Trig.halfPi)

    describe "Rect construction and accessors" $ do
        it "stores origin and extent" $
            Rect 1.0 2.0 10.0 5.0 `shouldBe` Rect 1.0 2.0 10.0 5.0
        it "constructs from corner points" $
            rectFromPoints (Point 1.0 2.0) (Point 11.0 7.0)
                `shouldBeRect` Rect 1.0 2.0 10.0 5.0
        it "constructs the zero rectangle" $
            zeroRect `shouldBe` Rect 0.0 0.0 0.0 0.0
        it "returns minimum, maximum, and center points" $ do
            let rect = Rect 2.0 3.0 8.0 4.0
            minPoint rect `shouldBePoint` Point 2.0 3.0
            maxPoint rect `shouldBePoint` Point 10.0 7.0
            center rect `shouldBePoint` Point 6.0 5.0

    describe "Rect predicates" $ do
        it "recognizes zero, negative, and positive areas" $ do
            isEmpty zeroRect `shouldBe` True
            isEmpty (Rect 0.0 0.0 (-1.0) 5.0) `shouldBe` True
            isEmpty (Rect 0.0 0.0 5.0 (-1.0)) `shouldBe` True
            isEmpty (Rect 0.0 0.0 5.0 5.0) `shouldBe` False
        it "contains interior and top-left points" $ do
            let rect = Rect 0.0 0.0 10.0 10.0
            containsPoint rect (Point 5.0 5.0) `shouldBe` True
            containsPoint rect (Point 0.0 0.0) `shouldBe` True
        it "excludes right, bottom, and exterior points" $ do
            let rect = Rect 0.0 0.0 10.0 10.0
            containsPoint rect (Point 10.0 5.0) `shouldBe` False
            containsPoint rect (Point 5.0 10.0) `shouldBe` False
            containsPoint rect (Point (-1.0) 5.0) `shouldBe` False
            containsPoint rect (Point 5.0 (-1.0)) `shouldBe` False

    describe "Rect set operations" $ do
        it "unites non-overlapping rectangles" $
            union (Rect 0.0 0.0 5.0 5.0) (Rect 10.0 10.0 5.0 5.0)
                `shouldBeRect` Rect 0.0 0.0 15.0 15.0
        it "treats an empty rectangle as the union identity on either side" $ do
            let rect = Rect 1.0 2.0 5.0 5.0
            union rect zeroRect `shouldBe` rect
            union zeroRect rect `shouldBe` rect
        it "returns a positive-area intersection" $
            intersection (Rect 0.0 0.0 10.0 10.0) (Rect 5.0 5.0 10.0 10.0)
                `shouldBe` Just (Rect 5.0 5.0 5.0 5.0)
        it "rejects disjoint and edge-touching intersections" $ do
            intersection (Rect 0.0 0.0 5.0 5.0) (Rect 10.0 10.0 5.0 5.0)
                `shouldBe` Nothing
            intersection (Rect 0.0 0.0 5.0 5.0) (Rect 5.0 0.0 5.0 5.0)
                `shouldBe` Nothing
            intersection (Rect 0.0 0.0 5.0 5.0) (Rect 0.0 5.0 5.0 5.0)
                `shouldBe` Nothing
        it "expands and shrinks symmetrically" $ do
            expandBy (Rect 1.0 1.0 8.0 8.0) 1.0
                `shouldBeRect` Rect 0.0 0.0 10.0 10.0
            expandBy (Rect 0.0 0.0 10.0 10.0) (-1.0)
                `shouldBeRect` Rect 1.0 1.0 8.0 8.0
