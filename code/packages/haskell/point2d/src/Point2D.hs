-- | Immutable 2D points, vectors, and axis-aligned rectangles.
module Point2D
    ( Point (..)
    , Rect (..)
    , origin
    , add
    , subtract
    , scale
    , negate
    , dot
    , cross
    , magnitude
    , magnitudeSquared
    , normalize
    , distance
    , distanceSquared
    , lerp
    , perpendicular
    , angle
    , rectFromPoints
    , zeroRect
    , minPoint
    , maxPoint
    , center
    , isEmpty
    , containsPoint
    , union
    , intersection
    , expandBy
    ) where

import qualified Prelude as P
import Prelude hiding (negate, subtract)
import qualified Trig

-- | A position or vector in the Cartesian plane.
data Point = Point
    { pointX :: Double
    , pointY :: Double
    }
    deriving (Eq, Show)

-- | An axis-aligned rectangle represented by its top-left corner and extent.
data Rect = Rect
    { rectX :: Double
    , rectY :: Double
    , rectWidth :: Double
    , rectHeight :: Double
    }
    deriving (Eq, Show)

-- | The additive identity @(0, 0)@.
origin :: Point
origin = Point 0.0 0.0

-- | Add two points component by component.
add :: Point -> Point -> Point
add left right =
    Point
        (pointX left + pointX right)
        (pointY left + pointY right)

-- | Subtract the second point from the first component by component.
subtract :: Point -> Point -> Point
subtract left right =
    Point
        (pointX left - pointX right)
        (pointY left - pointY right)

-- | Multiply both components by a scalar.
scale :: Point -> Double -> Point
scale point factor = Point (pointX point * factor) (pointY point * factor)

-- | Return the additive inverse of a point.
negate :: Point -> Point
negate point = Point (P.negate (pointX point)) (P.negate (pointY point))

-- | Compute the scalar dot product.
dot :: Point -> Point -> Double
dot left right = pointX left * pointX right + pointY left * pointY right

-- | Compute the scalar two-dimensional cross product.
cross :: Point -> Point -> Double
cross left right = pointX left * pointY right - pointY left * pointX right

-- | Compute Euclidean length through the shared pure 'Trig.sqrt'.
magnitude :: Point -> Double
magnitude point =
    case Trig.sqrt (magnitudeSquared point) of
        Right value -> value
        Left _ -> 0.0

-- | Compute squared Euclidean length without a square root.
magnitudeSquared :: Point -> Double
magnitudeSquared point = pointX point * pointX point + pointY point * pointY point

-- | Return a unit vector, or 'origin' when the magnitude is effectively zero.
normalize :: Point -> Point
normalize point
    | size < 1e-12 = origin
    | otherwise = Point (pointX point / size) (pointY point / size)
  where
    size = magnitude point

-- | Compute Euclidean distance between two points.
distance :: Point -> Point -> Double
distance left right = magnitude (subtract left right)

-- | Compute squared Euclidean distance between two points.
distanceSquared :: Point -> Point -> Double
distanceSquared left right = magnitudeSquared (subtract left right)

-- | Linearly interpolate without clamping the interpolation factor.
lerp :: Point -> Point -> Double -> Point
lerp start end amount =
    Point
        (pointX start + amount * (pointX end - pointX start))
        (pointY start + amount * (pointY end - pointY start))

-- | Rotate a vector 90 degrees counterclockwise.
perpendicular :: Point -> Point
perpendicular point = Point (P.negate (pointY point)) (pointX point)

-- | Return the direction in radians in the range @(-pi, pi]@.
angle :: Point -> Double
angle point = Trig.atan2 (pointY point) (pointX point)

-- | Construct a rectangle from top-left and bottom-right corner points.
rectFromPoints :: Point -> Point -> Rect
rectFromPoints minimumPoint maximumPoint =
    Rect
        (pointX minimumPoint)
        (pointY minimumPoint)
        (pointX maximumPoint - pointX minimumPoint)
        (pointY maximumPoint - pointY minimumPoint)

-- | The empty rectangle at the origin.
zeroRect :: Rect
zeroRect = Rect 0.0 0.0 0.0 0.0

-- | Return the rectangle's top-left corner.
minPoint :: Rect -> Point
minPoint rect = Point (rectX rect) (rectY rect)

-- | Return the rectangle's bottom-right corner.
maxPoint :: Rect -> Point
maxPoint rect =
    Point
        (rectX rect + rectWidth rect)
        (rectY rect + rectHeight rect)

-- | Return the rectangle's center.
center :: Rect -> Point
center rect =
    Point
        (rectX rect + rectWidth rect / 2.0)
        (rectY rect + rectHeight rect / 2.0)

-- | A rectangle is empty when either extent is non-positive.
isEmpty :: Rect -> Bool
isEmpty rect = rectWidth rect <= 0.0 || rectHeight rect <= 0.0

-- | Test the half-open region @[x, x + width) x [y, y + height)@.
containsPoint :: Rect -> Point -> Bool
containsPoint rect point =
    rectX rect <= pointX point
        && pointX point < rectX rect + rectWidth rect
        && rectY rect <= pointY point
        && pointY point < rectY rect + rectHeight rect

-- | Return the smallest rectangle that contains both inputs.
-- Empty rectangles act as the identity value.
union :: Rect -> Rect -> Rect
union left right
    | isEmpty left = right
    | isEmpty right = left
    | otherwise = Rect minimumX minimumY (maximumX - minimumX) (maximumY - minimumY)
  where
    minimumX = min (rectX left) (rectX right)
    minimumY = min (rectY left) (rectY right)
    maximumX = max (rectX left + rectWidth left) (rectX right + rectWidth right)
    maximumY = max (rectY left + rectHeight left) (rectY right + rectHeight right)

-- | Return the positive-area overlap, or 'Nothing' for disjoint or touching rectangles.
intersection :: Rect -> Rect -> Maybe Rect
intersection left right
    | overlapWidth <= 0.0 || overlapHeight <= 0.0 = Nothing
    | otherwise = Just (Rect overlapX overlapY overlapWidth overlapHeight)
  where
    overlapX = max (rectX left) (rectX right)
    overlapY = max (rectY left) (rectY right)
    overlapWidth = min (rectX left + rectWidth left) (rectX right + rectWidth right) - overlapX
    overlapHeight = min (rectY left + rectHeight left) (rectY right + rectHeight right) - overlapY

-- | Move every edge outward by an amount; negative amounts shrink the rectangle.
expandBy :: Rect -> Double -> Rect
expandBy rect amount =
    Rect
        (rectX rect - amount)
        (rectY rect - amount)
        (rectWidth rect + 2.0 * amount)
        (rectHeight rect + 2.0 * amount)
