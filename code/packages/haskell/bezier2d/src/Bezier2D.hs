-- | Pure quadratic and cubic Bezier curve geometry.
module Bezier2D
    ( QuadraticBezier (..)
    , CubicBezier (..)
    , evaluateQuadratic
    , derivativeQuadratic
    , splitQuadratic
    , toPolylineQuadratic
    , boundingBoxQuadratic
    , elevate
    , evaluateCubic
    , derivativeCubic
    , splitCubic
    , toPolylineCubic
    , boundingBoxCubic
    ) where

import Point2D (Point (..), Rect (..))
import qualified Point2D

-- | A quadratic Bezier with two on-curve endpoints and one control point.
data QuadraticBezier = QuadraticBezier
    { quadraticP0 :: Point
    , quadraticP1 :: Point
    , quadraticP2 :: Point
    }
    deriving (Eq, Show)

-- | A cubic Bezier with two on-curve endpoints and two control points.
data CubicBezier = CubicBezier
    { cubicP0 :: Point
    , cubicP1 :: Point
    , cubicP2 :: Point
    , cubicP3 :: Point
    }
    deriving (Eq, Show)

-- | Evaluate a quadratic curve with two levels of de Casteljau interpolation.
evaluateQuadratic :: QuadraticBezier -> Double -> Point
evaluateQuadratic curve amount = Point2D.lerp firstLevel secondLevel amount
  where
    firstLevel = Point2D.lerp (quadraticP0 curve) (quadraticP1 curve) amount
    secondLevel = Point2D.lerp (quadraticP1 curve) (quadraticP2 curve) amount

-- | Return the unnormalized quadratic tangent vector at a parameter value.
derivativeQuadratic :: QuadraticBezier -> Double -> Point
derivativeQuadratic curve amount =
    Point2D.scale (Point2D.lerp firstDifference secondDifference amount) 2.0
  where
    firstDifference = Point2D.subtract (quadraticP1 curve) (quadraticP0 curve)
    secondDifference = Point2D.subtract (quadraticP2 curve) (quadraticP1 curve)

-- | Split a quadratic into exact left and right sub-curves.
splitQuadratic :: QuadraticBezier -> Double -> (QuadraticBezier, QuadraticBezier)
splitQuadratic curve amount =
    ( QuadraticBezier (quadraticP0 curve) firstLevel midpoint
    , QuadraticBezier midpoint secondLevel (quadraticP2 curve)
    )
  where
    firstLevel = Point2D.lerp (quadraticP0 curve) (quadraticP1 curve) amount
    secondLevel = Point2D.lerp (quadraticP1 curve) (quadraticP2 curve) amount
    midpoint = Point2D.lerp firstLevel secondLevel amount

-- | Adaptively flatten a quadratic curve using the G2D02 midpoint criterion.
toPolylineQuadratic :: QuadraticBezier -> Double -> [Point]
toPolylineQuadratic curve tolerance
    | midpointError <= tolerance = [quadraticP0 curve, quadraticP2 curve]
    | otherwise = leftPoints ++ drop 1 rightPoints
  where
    chordMidpoint = Point2D.lerp (quadraticP0 curve) (quadraticP2 curve) 0.5
    curveMidpoint = evaluateQuadratic curve 0.5
    midpointError = Point2D.distance chordMidpoint curveMidpoint
    (left, right) = splitQuadratic curve 0.5
    leftPoints = toPolylineQuadratic left tolerance
    rightPoints = toPolylineQuadratic right tolerance

-- | Compute the tight axis-aligned bounds of a quadratic curve.
boundingBoxQuadratic :: QuadraticBezier -> Rect
boundingBoxQuadratic curve = boundsOfPoints candidates
  where
    xRoot = quadraticExtremum
        (pointX (quadraticP0 curve))
        (pointX (quadraticP1 curve))
        (pointX (quadraticP2 curve))
    yRoot = quadraticExtremum
        (pointY (quadraticP0 curve))
        (pointY (quadraticP1 curve))
        (pointY (quadraticP2 curve))
    roots = maybeToList xRoot ++ maybeToList yRoot
    candidates = quadraticP0 curve : quadraticP2 curve : map (evaluateQuadratic curve) roots

-- | Elevate a quadratic exactly to a cubic representation.
elevate :: QuadraticBezier -> CubicBezier
elevate curve = CubicBezier start firstControl secondControl end
  where
    start = quadraticP0 curve
    control = quadraticP1 curve
    end = quadraticP2 curve
    firstControl = Point2D.add (Point2D.scale start (1.0 / 3.0)) (Point2D.scale control (2.0 / 3.0))
    secondControl = Point2D.add (Point2D.scale control (2.0 / 3.0)) (Point2D.scale end (1.0 / 3.0))

-- | Evaluate a cubic curve with three levels of de Casteljau interpolation.
evaluateCubic :: CubicBezier -> Double -> Point
evaluateCubic curve amount = Point2D.lerp secondLeft secondRight amount
  where
    firstLeft = Point2D.lerp (cubicP0 curve) (cubicP1 curve) amount
    firstMiddle = Point2D.lerp (cubicP1 curve) (cubicP2 curve) amount
    firstRight = Point2D.lerp (cubicP2 curve) (cubicP3 curve) amount
    secondLeft = Point2D.lerp firstLeft firstMiddle amount
    secondRight = Point2D.lerp firstMiddle firstRight amount

-- | Return the unnormalized cubic tangent vector at a parameter value.
derivativeCubic :: CubicBezier -> Double -> Point
derivativeCubic curve amount = Point2D.scale weighted 3.0
  where
    complement = 1.0 - amount
    firstDifference = Point2D.subtract (cubicP1 curve) (cubicP0 curve)
    secondDifference = Point2D.subtract (cubicP2 curve) (cubicP1 curve)
    thirdDifference = Point2D.subtract (cubicP3 curve) (cubicP2 curve)
    weighted =
        Point2D.add
            (Point2D.scale firstDifference (complement * complement))
            (Point2D.add
                (Point2D.scale secondDifference (2.0 * complement * amount))
                (Point2D.scale thirdDifference (amount * amount)))

-- | Split a cubic into exact left and right sub-curves.
splitCubic :: CubicBezier -> Double -> (CubicBezier, CubicBezier)
splitCubic curve amount =
    ( CubicBezier (cubicP0 curve) firstLeft secondLeft midpoint
    , CubicBezier midpoint secondRight firstRight (cubicP3 curve)
    )
  where
    firstLeft = Point2D.lerp (cubicP0 curve) (cubicP1 curve) amount
    firstMiddle = Point2D.lerp (cubicP1 curve) (cubicP2 curve) amount
    firstRight = Point2D.lerp (cubicP2 curve) (cubicP3 curve) amount
    secondLeft = Point2D.lerp firstLeft firstMiddle amount
    secondRight = Point2D.lerp firstMiddle firstRight amount
    midpoint = Point2D.lerp secondLeft secondRight amount

-- | Adaptively flatten a cubic curve using the G2D02 midpoint criterion.
toPolylineCubic :: CubicBezier -> Double -> [Point]
toPolylineCubic curve tolerance
    | midpointError <= tolerance = [cubicP0 curve, cubicP3 curve]
    | otherwise = leftPoints ++ drop 1 rightPoints
  where
    chordMidpoint = Point2D.lerp (cubicP0 curve) (cubicP3 curve) 0.5
    curveMidpoint = evaluateCubic curve 0.5
    midpointError = Point2D.distance chordMidpoint curveMidpoint
    (left, right) = splitCubic curve 0.5
    leftPoints = toPolylineCubic left tolerance
    rightPoints = toPolylineCubic right tolerance

-- | Compute the tight axis-aligned bounds of a cubic curve.
boundingBoxCubic :: CubicBezier -> Rect
boundingBoxCubic curve = boundsOfPoints candidates
  where
    xRoots = cubicExtrema
        (pointX (cubicP0 curve))
        (pointX (cubicP1 curve))
        (pointX (cubicP2 curve))
        (pointX (cubicP3 curve))
    yRoots = cubicExtrema
        (pointY (cubicP0 curve))
        (pointY (cubicP1 curve))
        (pointY (cubicP2 curve))
        (pointY (cubicP3 curve))
    candidates = cubicP0 curve : cubicP3 curve : map (evaluateCubic curve) (xRoots ++ yRoots)

quadraticExtremum :: Double -> Double -> Double -> Maybe Double
quadraticExtremum first control end
    | abs denominator <= epsilon = Nothing
    | parameter > 0.0 && parameter < 1.0 = Just parameter
    | otherwise = Nothing
  where
    denominator = first - 2.0 * control + end
    parameter = (first - control) / denominator

cubicExtrema :: Double -> Double -> Double -> Double -> [Double]
cubicExtrema first firstControl secondControl end
    | abs quadraticCoefficient <= epsilon = linearRoots
    | discriminant < 0.0 = []
    | otherwise = filter insideUnitInterval [rootPlus, rootMinus]
  where
    quadraticCoefficient = -3.0 * first + 9.0 * firstControl - 9.0 * secondControl + 3.0 * end
    linearCoefficient = 6.0 * first - 12.0 * firstControl + 6.0 * secondControl
    constantCoefficient = -3.0 * first + 3.0 * firstControl
    discriminant = linearCoefficient * linearCoefficient - 4.0 * quadraticCoefficient * constantCoefficient
    rootDistance = sqrt discriminant
    denominator = 2.0 * quadraticCoefficient
    rootPlus = (-linearCoefficient + rootDistance) / denominator
    rootMinus = (-linearCoefficient - rootDistance) / denominator
    linearRoots
        | abs linearCoefficient <= epsilon = []
        | insideUnitInterval linearRoot = [linearRoot]
        | otherwise = []
    linearRoot = -constantCoefficient / linearCoefficient

insideUnitInterval :: Double -> Bool
insideUnitInterval parameter = parameter > 0.0 && parameter < 1.0

boundsOfPoints :: [Point] -> Rect
boundsOfPoints points = Rect minimumX minimumY (maximumX - minimumX) (maximumY - minimumY)
  where
    xs = map pointX points
    ys = map pointY points
    minimumX = minimum xs
    maximumX = maximum xs
    minimumY = minimum ys
    maximumY = maximum ys

maybeToList :: Maybe value -> [value]
maybeToList Nothing = []
maybeToList (Just value) = [value]

epsilon :: Double
epsilon = 1e-12
