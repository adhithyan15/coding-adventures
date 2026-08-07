module Affine2DSpec (spec) where

import Affine2D
import Point2D (Point (..))
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

shouldBeAffine :: Affine -> Affine -> Expectation
shouldBeAffine actual expected =
    mapM_ (uncurry shouldApprox) (zip (toArray actual) (toArray expected))

spec :: Spec
spec = do
    describe "Affine construction and factories" $ do
        it "constructs identity in standard component order" $ do
            identity `shouldBe` Affine 1.0 0.0 0.0 1.0 0.0 0.0
            toArray identity `shouldBe` [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        it "exposes all six immutable components" $ do
            let matrix = Affine 1.0 2.0 3.0 4.0 5.0 6.0
            affineA matrix `shouldBe` 1.0
            affineB matrix `shouldBe` 2.0
            affineC matrix `shouldBe` 3.0
            affineD matrix `shouldBe` 4.0
            affineE matrix `shouldBe` 5.0
            affineF matrix `shouldBe` 6.0
        it "leaves points unchanged under identity" $
            applyToPoint identity (Point 3.0 4.0) `shouldBePoint` Point 3.0 4.0
        it "translates positions" $
            applyToPoint (translate 3.0 4.0) (Point 1.0 1.0)
                `shouldBePoint` Point 4.0 5.0
        it "does not translate vectors" $
            applyToVector (translate 99.0 99.0) (Point 1.0 (-2.0))
                `shouldBePoint` Point 1.0 (-2.0)
        it "rotates 90 degrees counterclockwise" $
            applyToPoint (rotate Trig.halfPi) (Point 1.0 0.0)
                `shouldBePoint` Point 0.0 1.0
        it "treats a full rotation as identity" $
            isIdentity (rotate Trig.twoPi) `shouldBe` True
        it "rotates around a fixed center" $ do
            let matrix = rotateAround (Point 5.0 5.0) Trig.halfPi
            applyToPoint matrix (Point 5.0 5.0) `shouldBePoint` Point 5.0 5.0
            applyToPoint matrix (Point 6.0 5.0) `shouldBePoint` Point 5.0 6.0
        it "scales independently and uniformly" $ do
            applyToPoint (scale 2.0 3.0) (Point 1.0 1.0)
                `shouldBePoint` Point 2.0 3.0
            applyToPoint (scaleUniform 5.0) (Point 2.0 3.0)
                `shouldBePoint` Point 10.0 15.0
        it "skews on both axes" $ do
            applyToPoint (skewX (Trig.piValue / 4.0)) (Point 0.0 1.0)
                `shouldBePoint` Point 1.0 1.0
            applyToPoint (skewY (Trig.piValue / 4.0)) (Point 1.0 0.0)
                `shouldBePoint` Point 1.0 1.0

    describe "Affine composition" $ do
        it "multiplies by identity on either side" $ do
            let matrix = translate 3.0 4.0
            multiply matrix identity `shouldBeAffine` matrix
            multiply identity matrix `shouldBeAffine` matrix
        it "combines two quarter turns into a half turn" $
            multiply (rotate Trig.halfPi) (rotate Trig.halfPi)
                `shouldBeAffine` rotate Trig.piValue
        it "applies the right matrix first" $
            applyToPoint
                (multiply (translate 10.0 0.0) (scaleUniform 2.0))
                (Point 1.0 1.0)
                `shouldBePoint` Point 12.0 2.0
        it "thenTransform composes in readable left-to-right order" $
            applyToPoint
                (thenTransform (scaleUniform 2.0) (translate 10.0 0.0))
                (Point 1.0 1.0)
                `shouldBePoint` Point 12.0 2.0
        it "demonstrates non-commutativity" $ do
            let point = Point 1.0 0.0
                translateAfterRotate = multiply (translate 10.0 0.0) (rotate Trig.halfPi)
                rotateAfterTranslate = multiply (rotate Trig.halfPi) (translate 10.0 0.0)
            applyToPoint translateAfterRotate point `shouldBePoint` Point 10.0 1.0
            applyToPoint rotateAfterTranslate point `shouldBePoint` Point 0.0 11.0

    describe "Determinant and inversion" $ do
        it "reports identity, translation, scale, and rotation determinants" $ do
            determinant identity `shouldBe` 1.0
            determinant (translate 5.0 3.0) `shouldBe` 1.0
            determinant (scale 2.0 3.0) `shouldBe` 6.0
            determinant (rotate (Trig.piValue / 3.0)) `shouldApprox` 1.0
        it "inverts identity exactly" $
            invert identity `shouldBe` Just identity
        it "multiplies a transform by its inverse to identity" $ do
            let matrix = thenTransform (scale 2.0 3.0) (thenTransform (rotate 0.4) (translate 3.0 (-7.0)))
            case invert matrix of
                Nothing -> expectationFailure "expected matrix to be invertible"
                Just inverse -> isIdentity (multiply matrix inverse) `shouldBe` True
        it "rejects zero and near-singular scales" $ do
            invert (scale 0.0 1.0) `shouldBe` Nothing
            invert (scale 1e-13 1.0) `shouldBe` Nothing

    describe "Affine predicates" $ do
        it "recognizes identity within tolerance" $ do
            isIdentity identity `shouldBe` True
            isIdentity (Affine (1.0 + 1e-11) 1e-11 (-1e-11) (1.0 - 1e-11) 1e-11 (-1e-11))
                `shouldBe` True
        it "rejects visible translation and scale as identity" $ do
            isIdentity (translate 0.001 0.0) `shouldBe` False
            isIdentity (scale 2.0 1.0) `shouldBe` False
        it "recognizes only the pure-translation linear block" $ do
            isTranslationOnly identity `shouldBe` True
            isTranslationOnly (translate 5.0 3.0) `shouldBe` True
            isTranslationOnly (rotate 0.1) `shouldBe` False
            isTranslationOnly (scale 2.0 1.0) `shouldBe` False
