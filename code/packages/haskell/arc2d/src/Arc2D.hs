-- | Pure elliptical arc geometry in SVG endpoint and center forms.
module Arc2D
    ( SvgArc (..)
    , CenterArc (..)
    , toCenterArc
    , evaluateCenterArc
    , tangentCenterArc
    , boundingBoxCenterArc
    , toCubicBeziersCenter
    , toCubicBeziersSvg
    , evaluateSvgArc
    , boundingBoxSvgArc
    ) where

import Bezier2D (CubicBezier (..))
import Data.Either (fromRight)
import Data.Fixed (mod')
import Point2D (Point (..), Rect (..))
import qualified Point2D
import qualified Trig

-- | An elliptical arc in SVG endpoint form.
data SvgArc = SvgArc
    { svgFrom :: Point
    , svgTo :: Point
    , svgRx :: Double
    , svgRy :: Double
    , svgRotation :: Double
    , svgLargeArc :: Bool
    , svgSweep :: Bool
    }
    deriving (Eq, Show)

-- | An elliptical arc in center form.
data CenterArc = CenterArc
    { arcCenter :: Point
    , arcRx :: Double
    , arcRy :: Double
    , arcStartAngle :: Double
    , arcSweepAngle :: Double
    , arcRotation :: Double
    }
    deriving (Eq, Show)

-- | Convert SVG endpoint form to center form with the W3C algorithm.
-- Degenerate endpoint arcs and effectively zero radii return 'Nothing'.
toCenterArc :: SvgArc -> Maybe CenterArc
toCenterArc arc
    | Point2D.distanceSquared start end < pointEpsilonSquared = Nothing
    | abs (svgRx arc) < radiusEpsilon = Nothing
    | abs (svgRy arc) < radiusEpsilon = Nothing
    | otherwise = Just CenterArc
        { arcCenter = Point centerX centerY
        , arcRx = correctedRx
        , arcRy = correctedRy
        , arcStartAngle = startAngle
        , arcSweepAngle = correctedSweep
        , arcRotation = rotation
        }
  where
    start = svgFrom arc
    end = svgTo arc
    rotation = svgRotation arc
    cosineRotation = Trig.cos rotation
    sineRotation = Trig.sin rotation
    deltaX = (pointX start - pointX end) / 2.0
    deltaY = (pointY start - pointY end) / 2.0
    localX = cosineRotation * deltaX + sineRotation * deltaY
    localY = negate sineRotation * deltaX + cosineRotation * deltaY
    initialRx = abs (svgRx arc)
    initialRy = abs (svgRy arc)
    lambda = square (localX / initialRx) + square (localY / initialRy)
    radiusScale = if lambda > 1.0 then squareRoot lambda else 1.0
    correctedRx = initialRx * radiusScale
    correctedRy = initialRy * radiusScale
    rxSquared = square correctedRx
    rySquared = square correctedRy
    localXSquared = square localX
    localYSquared = square localY
    numerator = rxSquared * rySquared
        - rxSquared * localYSquared
        - rySquared * localXSquared
    denominator = rxSquared * localYSquared + rySquared * localXSquared
    centerFactor = squareRoot (max 0.0 (numerator / denominator))
    centerSign = if svgLargeArc arc == svgSweep arc then -1.0 else 1.0
    centerLocalX = centerSign * centerFactor * correctedRx * localY / correctedRy
    centerLocalY = centerSign * centerFactor * negate (correctedRy * localX / correctedRx)
    midpointX = (pointX start + pointX end) / 2.0
    midpointY = (pointY start + pointY end) / 2.0
    centerX = cosineRotation * centerLocalX - sineRotation * centerLocalY + midpointX
    centerY = sineRotation * centerLocalX + cosineRotation * centerLocalY + midpointY
    unitStartX = (localX - centerLocalX) / correctedRx
    unitStartY = (localY - centerLocalY) / correctedRy
    unitEndX = (negate localX - centerLocalX) / correctedRx
    unitEndY = (negate localY - centerLocalY) / correctedRy
    startAngle = angleBetween 1.0 0.0 unitStartX unitStartY
    rawSweep = angleBetween unitStartX unitStartY unitEndX unitEndY
    correctedSweep
        | not (svgSweep arc) && rawSweep > 0.0 = rawSweep - Trig.twoPi
        | svgSweep arc && rawSweep < 0.0 = rawSweep + Trig.twoPi
        | otherwise = rawSweep

-- | Evaluate a center-form arc at a normalized parameter.
evaluateCenterArc :: CenterArc -> Double -> Point
evaluateCenterArc arc amount = pointAtAngle arc angle
  where
    angle = arcStartAngle arc + amount * arcSweepAngle arc

-- | Return the unnormalized derivative with respect to the normalized parameter.
tangentCenterArc :: CenterArc -> Double -> Point
tangentCenterArc arc amount = Point worldX worldY
  where
    angle = arcStartAngle arc + amount * arcSweepAngle arc
    localX = negate (arcRx arc) * Trig.sin angle * arcSweepAngle arc
    localY = arcRy arc * Trig.cos angle * arcSweepAngle arc
    cosineRotation = Trig.cos (arcRotation arc)
    sineRotation = Trig.sin (arcRotation arc)
    worldX = cosineRotation * localX - sineRotation * localY
    worldY = sineRotation * localX + cosineRotation * localY

-- | Compute tight axis-aligned bounds from the endpoints and rotated-ellipse extrema.
boundingBoxCenterArc :: CenterArc -> Rect
boundingBoxCenterArc arc = boundsOfPoints (map (pointAtAngle arc) includedAngles)
  where
    rotation = arcRotation arc
    xExtremum = Trig.atan2
        (negate (arcRy arc) * Trig.sin rotation)
        (arcRx arc * Trig.cos rotation)
    yExtremum = Trig.atan2
        (arcRy arc * Trig.cos rotation)
        (arcRx arc * Trig.sin rotation)
    extrema =
        [ xExtremum
        , xExtremum + Trig.piValue
        , yExtremum
        , yExtremum + Trig.piValue
        ]
    includedAngles =
        arcStartAngle arc
            : arcStartAngle arc + arcSweepAngle arc
            : filter (angleOnArc arc) extrema

-- | Approximate a center-form arc with cubic Beziers of at most 90 degrees.
toCubicBeziersCenter :: CenterArc -> [CubicBezier]
toCubicBeziersCenter arc = map segment ([0 .. segmentCount - 1] :: [Int])
  where
    segmentCount = max 1 (ceiling (abs (arcSweepAngle arc) / Trig.halfPi))
    segmentSweep = arcSweepAngle arc / fromIntegral segmentCount
    magic = (4.0 / 3.0) * Trig.tan (segmentSweep / 4.0)
    segment index = CubicBezier
        (worldPoint startLocal)
        (worldPoint firstControlLocal)
        (worldPoint secondControlLocal)
        (worldPoint endLocal)
      where
        startAngle = arcStartAngle arc + fromIntegral index * segmentSweep
        endAngle = startAngle + segmentSweep
        startLocal = localPoint arc startAngle
        endLocal = localPoint arc endAngle
        startTangent = localAngleTangent arc startAngle
        endTangent = localAngleTangent arc endAngle
        firstControlLocal = Point2D.add startLocal (Point2D.scale startTangent magic)
        secondControlLocal = Point2D.subtract endLocal (Point2D.scale endTangent magic)
    worldPoint = rotateTranslate (arcCenter arc) (arcRotation arc)

-- | Convert an endpoint-form arc to cubics, or return an empty list if degenerate.
toCubicBeziersSvg :: SvgArc -> [CubicBezier]
toCubicBeziersSvg = maybe [] toCubicBeziersCenter . toCenterArc

-- | Evaluate an endpoint-form arc, falling back to its line segment when degenerate.
evaluateSvgArc :: SvgArc -> Double -> Point
evaluateSvgArc arc amount =
    maybe
        (Point2D.lerp (svgFrom arc) (svgTo arc) amount)
        (`evaluateCenterArc` amount)
        (toCenterArc arc)

-- | Bound an endpoint-form arc, falling back to its line segment when degenerate.
boundingBoxSvgArc :: SvgArc -> Rect
boundingBoxSvgArc arc =
    maybe
        (boundsOfPoints [svgFrom arc, svgTo arc])
        boundingBoxCenterArc
        (toCenterArc arc)

pointAtAngle :: CenterArc -> Double -> Point
pointAtAngle arc angle = rotateTranslate (arcCenter arc) (arcRotation arc) (localPoint arc angle)

localPoint :: CenterArc -> Double -> Point
localPoint arc angle = Point (arcRx arc * Trig.cos angle) (arcRy arc * Trig.sin angle)

localAngleTangent :: CenterArc -> Double -> Point
localAngleTangent arc angle = Point
    (negate (arcRx arc) * Trig.sin angle)
    (arcRy arc * Trig.cos angle)

rotateTranslate :: Point -> Double -> Point -> Point
rotateTranslate center rotation local = Point
    (cosineRotation * pointX local - sineRotation * pointY local + pointX center)
    (sineRotation * pointX local + cosineRotation * pointY local + pointY center)
  where
    cosineRotation = Trig.cos rotation
    sineRotation = Trig.sin rotation

angleOnArc :: CenterArc -> Double -> Bool
angleOnArc arc angle
    | abs sweep >= Trig.twoPi - angleEpsilon = True
    | sweep >= 0.0 = normalizeAngle (angle - start) <= sweep + angleEpsilon
    | otherwise = normalizeAngle (start - angle) <= negate sweep + angleEpsilon
  where
    start = arcStartAngle arc
    sweep = arcSweepAngle arc

normalizeAngle :: Double -> Double
normalizeAngle angle = angle `mod'` Trig.twoPi

angleBetween :: Double -> Double -> Double -> Double -> Double
angleBetween firstX firstY secondX secondY =
    Trig.atan2
        (firstX * secondY - firstY * secondX)
        (firstX * secondX + firstY * secondY)

boundsOfPoints :: [Point] -> Rect
boundsOfPoints points = Rect minimumX minimumY (maximumX - minimumX) (maximumY - minimumY)
  where
    xs = map pointX points
    ys = map pointY points
    minimumX = minimum xs
    maximumX = maximum xs
    minimumY = minimum ys
    maximumY = maximum ys

squareRoot :: Double -> Double
squareRoot = fromRight 0.0 . Trig.sqrt

square :: Double -> Double
square value = value * value

pointEpsilonSquared :: Double
pointEpsilonSquared = 1e-20

radiusEpsilon :: Double
radiusEpsilon = 1e-10

angleEpsilon :: Double
angleEpsilon = 1e-12
