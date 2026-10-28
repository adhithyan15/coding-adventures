-- | Pure quadratic and cubic Bezier curve geometry.
module Bezier2D
    ( QuadraticBezier (..)
    , CubicBezier (..)
    , FlatteningError (..)
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

-- | Deterministic failures from the bounded G2D02 flattening contract.
data FlatteningError
    = InvalidTolerance
    | NonFiniteControlPoint
    | NonFiniteGeometry
    | FlatteningBudgetExceeded
    deriving (Eq, Show)

maxFlatteningDepth :: Int
maxFlatteningDepth = 32

maxFlatteningSubdivisions :: Int
maxFlatteningSubdivisions = 65535

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

-- | Flatten a quadratic with conservative finite-segment error measurement.
--
-- The work list makes both safety budgets explicit. A 'Left' never exposes the
-- internally accumulated prefix, so callers cannot mistake a partial curve for
-- a complete approximation.
toPolylineQuadratic :: QuadraticBezier -> Double -> Either FlatteningError [Point]
toPolylineQuadratic curve tolerance = do
    validateFlatteningInputs tolerance
        [quadraticP0 curve, quadraticP1 curve, quadraticP2 curve]
    go [(curve, 0)] 0 [quadraticP0 curve]
  where
    go [] _ pointsReversed = Right (reverse pointsReversed)
    go ((current, depth) : pending) subdivisions pointsReversed = do
        errorBound <- distanceToSegment
            (quadraticP1 current)
            (quadraticP0 current)
            (quadraticP2 current)
        if errorBound <= tolerance
            then go pending subdivisions (quadraticP2 current : pointsReversed)
            else if depth >= maxFlatteningDepth || subdivisions >= maxFlatteningSubdivisions
                then Left FlatteningBudgetExceeded
                else
                    let (left, right) = splitQuadratic current 0.5
                    in go
                        ((left, depth + 1) : (right, depth + 1) : pending)
                        (subdivisions + 1)
                        pointsReversed

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

-- | Flatten a cubic under the same conservative and bounded contract.
toPolylineCubic :: CubicBezier -> Double -> Either FlatteningError [Point]
toPolylineCubic curve tolerance = do
    validateFlatteningInputs tolerance
        [cubicP0 curve, cubicP1 curve, cubicP2 curve, cubicP3 curve]
    go [(curve, 0)] 0 [cubicP0 curve]
  where
    go [] _ pointsReversed = Right (reverse pointsReversed)
    go ((current, depth) : pending) subdivisions pointsReversed = do
        firstError <- distanceToSegment
            (cubicP1 current)
            (cubicP0 current)
            (cubicP3 current)
        secondError <- distanceToSegment
            (cubicP2 current)
            (cubicP0 current)
            (cubicP3 current)
        if max firstError secondError <= tolerance
            then go pending subdivisions (cubicP3 current : pointsReversed)
            else if depth >= maxFlatteningDepth || subdivisions >= maxFlatteningSubdivisions
                then Left FlatteningBudgetExceeded
                else
                    let (left, right) = splitCubic current 0.5
                    in go
                        ((left, depth + 1) : (right, depth + 1) : pending)
                        (subdivisions + 1)
                        pointsReversed

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

validateFlatteningInputs :: Double -> [Point] -> Either FlatteningError ()
validateFlatteningInputs tolerance points
    | not (isFinite tolerance) || tolerance <= 0.0 = Left InvalidTolerance
    | all pointIsFinite points = Right ()
    | otherwise = Left NonFiniteControlPoint

pointIsFinite :: Point -> Bool
pointIsFinite point = isFinite (pointX point) && isFinite (pointY point)

isFinite :: Double -> Bool
isFinite value = not (isNaN value || isInfinite value)

-- | Measure a point against the finite segment, not its infinite support line.
-- Scaling first keeps the square root stable for very large or tiny values;
-- every derived value is checked so finite inputs cannot silently create NaN.
distanceToSegment :: Point -> Point -> Point -> Either FlatteningError Double
distanceToSegment point start end = do
    deltaX <- finiteValue (pointX end - pointX start)
    deltaY <- finiteValue (pointY end - pointY start)
    let scale = max (abs deltaX) (abs deltaY)
    if scale == 0.0
        then stableDistance point start
        else do
            let normalizedX = deltaX / scale
                normalizedY = deltaY / scale
            lengthSquared <- finiteValue
                (normalizedX * normalizedX + normalizedY * normalizedY)
            offsetX <- finiteValue (pointX point - pointX start)
            offsetY <- finiteValue (pointY point - pointY start)
            normalizedOffsetX <- finiteValue (offsetX / scale)
            normalizedOffsetY <- finiteValue (offsetY / scale)
            projection <- finiteValue
                ((normalizedOffsetX * normalizedX + normalizedOffsetY * normalizedY)
                    / lengthSquared)
            let boundedProjection = max 0.0 (min 1.0 projection)
            nearestX <- finiteValue (pointX start + boundedProjection * deltaX)
            nearestY <- finiteValue (pointY start + boundedProjection * deltaY)
            stableDistance point (Point nearestX nearestY)

stableDistance :: Point -> Point -> Either FlatteningError Double
stableDistance left right = do
    deltaX <- finiteValue (pointX left - pointX right)
    deltaY <- finiteValue (pointY left - pointY right)
    let scale = max (abs deltaX) (abs deltaY)
    if scale == 0.0
        then Right 0.0
        else finiteValue
            (scale * sqrt ((deltaX / scale) ^ (2 :: Int) + (deltaY / scale) ^ (2 :: Int)))

finiteValue :: Double -> Either FlatteningError Double
finiteValue value
    | isFinite value = Right value
    | otherwise = Left NonFiniteGeometry

epsilon :: Double
epsilon = 1e-12
