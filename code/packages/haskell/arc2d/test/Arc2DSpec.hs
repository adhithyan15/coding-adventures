module Arc2DSpec (spec) where

import Arc2D
import Bezier2D (CubicBezier (..), evaluateCubic)
import Data.Maybe (fromJust)
import Point2D (Point (..), Rect (..))
import qualified Point2D
import Test.Hspec
import qualified Trig

epsilon :: Double
epsilon = 1e-8

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

quarterCircle :: CenterArc
quarterCircle = CenterArc Point2D.origin 1.0 1.0 0.0 Trig.halfPi 0.0

quarterSvg :: SvgArc
quarterSvg = SvgArc (Point 1.0 0.0) (Point 0.0 1.0) 1.0 1.0 0.0 False True

spec :: Spec
spec = do
    describe "CenterArc" $ do
        it "stores center-form parameters" $ do
            arcCenter quarterCircle `shouldBe` Point2D.origin
            arcRx quarterCircle `shouldBe` 1.0
            arcRy quarterCircle `shouldBe` 1.0
            arcStartAngle quarterCircle `shouldBe` 0.0
            arcSweepAngle quarterCircle `shouldBe` Trig.halfPi
            arcRotation quarterCircle `shouldBe` 0.0
        it "evaluates quarter-circle endpoints and midpoint" $ do
            evaluateCenterArc quarterCircle 0.0 `shouldBePoint` Point 1.0 0.0
            evaluateCenterArc quarterCircle 0.5 `shouldBePoint` Point (Trig.sqrt 0.5 `rightOr` 0.0) (Trig.sqrt 0.5 `rightOr` 0.0)
            evaluateCenterArc quarterCircle 1.0 `shouldBePoint` Point 0.0 1.0
        it "evaluates a full circle midpoint" $
            evaluateCenterArc (CenterArc Point2D.origin 2.0 2.0 0.0 Trig.twoPi 0.0) 0.5
                `shouldBePoint` Point (-2.0) 0.0
        it "applies center offset and ellipse rotation" $ do
            let arc = CenterArc (Point 5.0 3.0) 2.0 1.0 0.0 Trig.halfPi Trig.halfPi
            evaluateCenterArc arc 0.0 `shouldBePoint` Point 5.0 5.0
            evaluateCenterArc arc 1.0 `shouldBePoint` Point 4.0 3.0
        it "returns tangents in both sweep directions" $ do
            tangentCenterArc quarterCircle 0.0 `shouldBePoint` Point 0.0 Trig.halfPi
            let clockwise = CenterArc Point2D.origin 1.0 1.0 0.0 (negate Trig.halfPi) 0.0
            tangentCenterArc clockwise 0.0 `shouldBePoint` Point 0.0 (negate Trig.halfPi)
        it "rotates tangent vectors without translating them" $ do
            let arc = CenterArc (Point 20.0 30.0) 2.0 1.0 0.0 Trig.halfPi Trig.halfPi
            tangentCenterArc arc 0.0 `shouldBePoint` Point (negate Trig.halfPi) 0.0
        it "computes tight axis-aligned quarter-circle bounds" $
            boundingBoxCenterArc quarterCircle `shouldBeRect` Rect 0.0 0.0 1.0 1.0
        it "computes exact full rotated-ellipse bounds" $ do
            let arc = CenterArc Point2D.origin 3.0 1.0 0.0 Trig.twoPi (Trig.piValue / 4.0)
                xRadius = Trig.sqrt 5.0 `rightOr` 0.0
            boundingBoxCenterArc arc `shouldBeRect` Rect (negate xRadius) (negate xRadius) (2.0 * xRadius) (2.0 * xRadius)
        it "handles negative sweeps when selecting bounds extrema" $ do
            let arc = CenterArc Point2D.origin 2.0 1.0 0.0 (negate Trig.halfPi) 0.0
            boundingBoxCenterArc arc `shouldBeRect` Rect 0.0 (-1.0) 2.0 1.0
        it "bounds a zero-sweep arc as one point" $
            boundingBoxCenterArc (CenterArc (Point 1.0 2.0) 3.0 4.0 0.0 0.0 0.0)
                `shouldBeRect` Rect 4.0 2.0 0.0 0.0
        it "creates one cubic with the quarter-circle magic controls" $ do
            let curves = toCubicBeziersCenter quarterCircle
                curve = head curves
                magic = (4.0 / 3.0) * Trig.tan (Trig.piValue / 8.0)
            length curves `shouldBe` 1
            cubicP0 curve `shouldBePoint` Point 1.0 0.0
            cubicP1 curve `shouldBePoint` Point 1.0 magic
            cubicP2 curve `shouldBePoint` Point magic 1.0
            cubicP3 curve `shouldBePoint` Point 0.0 1.0
            evaluateCubic curve 0.5 `shouldBePointWithin` (Point (Trig.sqrt 0.5 `rightOr` 0.0) (Trig.sqrt 0.5 `rightOr` 0.0), 3e-4)
        it "splits full circles into four continuous cubics" $ do
            let curves = toCubicBeziersCenter (CenterArc Point2D.origin 1.0 1.0 0.0 Trig.twoPi 0.0)
            length curves `shouldBe` 4
            mapM_ (uncurry shouldJoin) (zip curves (drop 1 curves))
            cubicP3 (last curves) `shouldBePoint` cubicP0 (head curves)
        it "splits a 91-degree arc into two cubics" $
            length (toCubicBeziersCenter (CenterArc Point2D.origin 1.0 1.0 0.0 (Trig.radians 91.0) 0.0))
                `shouldBe` 2
        it "represents a zero sweep with one degenerate cubic" $ do
            let curves = toCubicBeziersCenter (CenterArc Point2D.origin 2.0 1.0 0.0 0.0 0.0)
                curve = head curves
            length curves `shouldBe` 1
            cubicP0 curve `shouldBePoint` Point 2.0 0.0
            cubicP1 curve `shouldBePoint` Point 2.0 0.0
            cubicP2 curve `shouldBePoint` Point 2.0 0.0
            cubicP3 curve `shouldBePoint` Point 2.0 0.0

    describe "SvgArc" $ do
        it "stores endpoint-form parameters" $ do
            svgFrom quarterSvg `shouldBe` Point 1.0 0.0
            svgTo quarterSvg `shouldBe` Point 0.0 1.0
            svgRx quarterSvg `shouldBe` 1.0
            svgRy quarterSvg `shouldBe` 1.0
            svgRotation quarterSvg `shouldBe` 0.0
            svgLargeArc quarterSvg `shouldBe` False
            svgSweep quarterSvg `shouldBe` True
        it "rejects coincident endpoints and each zero-radius case" $ do
            toCenterArc (SvgArc Point2D.origin Point2D.origin 1.0 1.0 0.0 False True) `shouldBe` Nothing
            toCenterArc (SvgArc Point2D.origin (Point 1.0 0.0) 1e-11 1.0 0.0 False True) `shouldBe` Nothing
            toCenterArc (SvgArc Point2D.origin (Point 1.0 0.0) 1.0 1e-11 0.0 False True) `shouldBe` Nothing
        it "converts the unit quarter circle" $ do
            let arc = fromJust (toCenterArc quarterSvg)
            arcCenter arc `shouldBePoint` Point2D.origin
            arcRx arc `shouldApprox` 1.0
            arcRy arc `shouldApprox` 1.0
            arcSweepAngle arc `shouldApprox` Trig.halfPi
        it "accepts negative radii by taking their absolute values" $ do
            let arc = fromJust (toCenterArc quarterSvg {svgRx = -1.0, svgRy = -1.0})
            arcRx arc `shouldApprox` 1.0
            arcRy arc `shouldApprox` 1.0
        it "scales radii that are too small to join the endpoints" $ do
            let arc = SvgArc (Point 0.0 0.0) (Point 10.0 0.0) 0.1 0.1 0.0 False True
                centerArc = fromJust (toCenterArc arc)
            arcRx centerArc `shouldApprox` 5.0
            arcRy centerArc `shouldApprox` 5.0
        it "honors clockwise and counterclockwise sweep flags" $ do
            let counterclockwise = fromJust (toCenterArc quarterSvg)
                clockwise = fromJust (toCenterArc quarterSvg {svgSweep = False})
            arcSweepAngle counterclockwise `shouldSatisfy` (> 0.0)
            arcSweepAngle clockwise `shouldSatisfy` (< 0.0)
        it "wraps a positive raw angle into a large clockwise sweep" $ do
            let arc = fromJust (toCenterArc quarterSvg {svgLargeArc = True, svgSweep = False})
            arcSweepAngle arc `shouldSatisfy` (< negate Trig.piValue)
        it "honors the large-arc flag" $ do
            let endpoint = Point (negate (Trig.cos 0.3)) (Trig.sin 0.3)
                base = SvgArc (Point 1.0 0.0) endpoint 1.0 1.0 0.0 False True
                smallArc = fromJust (toCenterArc base)
                largeArc = fromJust (toCenterArc base {svgLargeArc = True})
            abs (arcSweepAngle smallArc) `shouldSatisfy` (< Trig.piValue)
            abs (arcSweepAngle largeArc) `shouldSatisfy` (> Trig.piValue)
        it "preserves endpoints through nonzero rotation conversion" $ do
            let arc = SvgArc (Point 2.0 1.0) (Point (-1.0) 2.0) 3.0 1.5 (Trig.piValue / 4.0) False True
                centerArc = fromJust (toCenterArc arc)
            evaluateCenterArc centerArc 0.0 `shouldBePoint` svgFrom arc
            evaluateCenterArc centerArc 1.0 `shouldBePoint` svgTo arc
        it "delegates evaluation, cubics, and tight bounds" $ do
            evaluateSvgArc quarterSvg 0.0 `shouldBePoint` svgFrom quarterSvg
            evaluateSvgArc quarterSvg 1.0 `shouldBePoint` svgTo quarterSvg
            boundingBoxSvgArc quarterSvg `shouldBeRect` Rect 0.0 0.0 1.0 1.0
            let curves = toCubicBeziersSvg quarterSvg
                centerArc = fromJust (toCenterArc quarterSvg)
            curves `shouldBe` toCubicBeziersCenter centerArc
        it "uses line fallbacks for degenerate endpoint arcs" $ do
            let arc = SvgArc (Point 4.0 2.0) (Point (-2.0) 8.0) 0.0 2.0 0.0 False True
            evaluateSvgArc arc 0.25 `shouldBePoint` Point 2.5 3.5
            boundingBoxSvgArc arc `shouldBeRect` Rect (-2.0) 2.0 6.0 6.0
            toCubicBeziersSvg arc `shouldBe` []

rightOr :: Either error value -> value -> value
rightOr result fallback = either (const fallback) id result

shouldJoin :: CubicBezier -> CubicBezier -> Expectation
shouldJoin left right = cubicP3 left `shouldBePoint` cubicP0 right

shouldBePointWithin :: Point -> (Point, Double) -> Expectation
shouldBePointWithin actual (expected, tolerance) =
    Point2D.distance actual expected `shouldSatisfy` (< tolerance)
