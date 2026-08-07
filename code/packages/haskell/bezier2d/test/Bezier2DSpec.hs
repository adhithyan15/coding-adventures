module Bezier2DSpec (spec) where

import Bezier2D
import Point2D (Point (..), Rect (..))
import Test.Hspec

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

quadratic :: QuadraticBezier
quadratic = QuadraticBezier (Point 0.0 0.0) (Point 1.0 2.0) (Point 2.0 0.0)

cubic :: CubicBezier
cubic = CubicBezier (Point 0.0 0.0) (Point 1.0 2.0) (Point 3.0 2.0) (Point 4.0 0.0)

spec :: Spec
spec = do
    describe "QuadraticBezier" $ do
        it "stores immutable control points" $ do
            quadraticP0 quadratic `shouldBe` Point 0.0 0.0
            quadraticP1 quadratic `shouldBe` Point 1.0 2.0
            quadraticP2 quadratic `shouldBe` Point 2.0 0.0
        it "evaluates both endpoints" $ do
            evaluateQuadratic quadratic 0.0 `shouldBePoint` quadraticP0 quadratic
            evaluateQuadratic quadratic 1.0 `shouldBePoint` quadraticP2 quadratic
        it "evaluates the known midpoint" $
            evaluateQuadratic quadratic 0.5 `shouldBePoint` Point 1.0 1.0
        it "returns the endpoint derivatives" $ do
            derivativeQuadratic quadratic 0.0 `shouldBePoint` Point 2.0 4.0
            derivativeQuadratic quadratic 1.0 `shouldBePoint` Point 2.0 (-4.0)
        it "splits into exact reparameterized halves" $ do
            let amount = 0.25
                (left, right) = splitQuadratic quadratic amount
            quadraticP0 left `shouldBePoint` quadraticP0 quadratic
            quadraticP2 left `shouldBePoint` evaluateQuadratic quadratic amount
            quadraticP0 right `shouldBePoint` evaluateQuadratic quadratic amount
            quadraticP2 right `shouldBePoint` quadraticP2 quadratic
            evaluateQuadratic left 0.5 `shouldBePoint` evaluateQuadratic quadratic (amount * 0.5)
            evaluateQuadratic right 0.5 `shouldBePoint` evaluateQuadratic quadratic (amount + (1.0 - amount) * 0.5)
        it "flattens a straight quadratic to its endpoints" $ do
            let straight = QuadraticBezier (Point 0.0 0.0) (Point 1.0 0.0) (Point 2.0 0.0)
            toPolylineQuadratic straight 0.1 `shouldBe` [Point 0.0 0.0, Point 2.0 0.0]
        it "subdivides a curved quadratic without duplicate joins" $ do
            let coarse = toPolylineQuadratic quadratic 2.0
                fine = toPolylineQuadratic quadratic 0.01
            length coarse `shouldBe` 2
            length fine `shouldSatisfy` (> length coarse)
            head fine `shouldBePoint` quadraticP0 quadratic
            last fine `shouldBePoint` quadraticP2 quadratic
            and (zipWith (/=) fine (drop 1 fine)) `shouldBe` True
        it "computes a tight interior-extremum bounding box" $
            boundingBoxQuadratic quadratic `shouldBeRect` Rect 0.0 0.0 2.0 1.0
        it "ignores roots outside the parameter interval" $ do
            let monotone = QuadraticBezier (Point 0.0 0.0) (Point 2.0 2.0) (Point 3.0 3.0)
            boundingBoxQuadratic monotone `shouldBeRect` Rect 0.0 0.0 3.0 3.0
        it "elevates exactly to a cubic" $ do
            let elevated = elevate quadratic
            cubicP0 elevated `shouldBePoint` quadraticP0 quadratic
            cubicP3 elevated `shouldBePoint` quadraticP2 quadratic
            mapM_ (\amount -> evaluateCubic elevated amount `shouldBePoint` evaluateQuadratic quadratic amount)
                [0.0, 0.25, 0.5, 0.75, 1.0]

    describe "CubicBezier" $ do
        it "stores immutable control points" $ do
            cubicP0 cubic `shouldBe` Point 0.0 0.0
            cubicP1 cubic `shouldBe` Point 1.0 2.0
            cubicP2 cubic `shouldBe` Point 3.0 2.0
            cubicP3 cubic `shouldBe` Point 4.0 0.0
        it "evaluates both endpoints" $ do
            evaluateCubic cubic 0.0 `shouldBePoint` cubicP0 cubic
            evaluateCubic cubic 1.0 `shouldBePoint` cubicP3 cubic
        it "matches the worked symmetric midpoint" $
            evaluateCubic cubic 0.5 `shouldBePoint` Point 2.0 1.5
        it "returns the endpoint derivatives" $ do
            derivativeCubic cubic 0.0 `shouldBePoint` Point 3.0 6.0
            derivativeCubic cubic 1.0 `shouldBePoint` Point 3.0 (-6.0)
        it "splits into exact reparameterized halves" $ do
            let amount = 0.4
                (left, right) = splitCubic cubic amount
            cubicP0 left `shouldBePoint` cubicP0 cubic
            cubicP3 left `shouldBePoint` evaluateCubic cubic amount
            cubicP0 right `shouldBePoint` evaluateCubic cubic amount
            cubicP3 right `shouldBePoint` cubicP3 cubic
            evaluateCubic left 0.5 `shouldBePoint` evaluateCubic cubic (amount * 0.5)
            evaluateCubic right 0.5 `shouldBePoint` evaluateCubic cubic (amount + (1.0 - amount) * 0.5)
        it "flattens a straight cubic to its endpoints" $ do
            let straight = CubicBezier (Point 0.0 0.0) (Point 1.0 0.0) (Point 2.0 0.0) (Point 3.0 0.0)
            toPolylineCubic straight 0.1 `shouldBe` [Point 0.0 0.0, Point 3.0 0.0]
        it "uses tighter tolerances for finer curved polylines" $ do
            let arch = CubicBezier (Point 0.0 0.0) (Point 0.0 10.0) (Point 10.0 10.0) (Point 10.0 0.0)
                coarse = toPolylineCubic arch 8.0
                fine = toPolylineCubic arch 0.05
            length coarse `shouldBe` 2
            length fine `shouldSatisfy` (> length coarse)
            head fine `shouldBePoint` Point 0.0 0.0
            last fine `shouldBePoint` Point 10.0 0.0
        it "computes exact symmetric arch bounds through a linear derivative root" $ do
            let arch = CubicBezier (Point 0.0 0.0) (Point 0.0 10.0) (Point 10.0 10.0) (Point 10.0 0.0)
            boundingBoxCubic arch `shouldBeRect` Rect 0.0 0.0 10.0 7.5
        it "handles two roots per axis and contains sampled points" $ do
            let looping = CubicBezier (Point 0.0 0.0) (Point 2.0 3.0) (Point (-1.0) 3.0) (Point 1.0 0.0)
                bounds = boundingBoxCubic looping
                samples = map (evaluateCubic looping . (/ 20.0) . fromIntegral) ([0 .. 20] :: [Int])
                contains point =
                    pointX point >= rectX bounds - epsilon
                        && pointX point <= rectX bounds + rectWidth bounds + epsilon
                        && pointY point >= rectY bounds - epsilon
                        && pointY point <= rectY bounds + rectHeight bounds + epsilon
            all contains samples `shouldBe` True
        it "handles constant derivatives and a negative discriminant" $ do
            let increasing = CubicBezier (Point 0.0 0.0) (Point 1.0 0.0) (Point 2.0 0.0) (Point 4.0 0.0)
            boundingBoxCubic increasing `shouldBeRect` Rect 0.0 0.0 4.0 0.0
        it "ignores a degenerate linear root outside the parameter interval" $ do
            let monotone = CubicBezier (Point 0.0 0.0) (Point 2.0 2.0) (Point 3.0 3.0) (Point 3.0 3.0)
                bounds = boundingBoxCubic monotone
            bounds `shouldBeRect` Rect 0.0 0.0 3.0 3.0
