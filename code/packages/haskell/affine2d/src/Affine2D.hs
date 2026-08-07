-- | Immutable two-dimensional affine transformation matrices.
module Affine2D
    ( Affine (..)
    , identity
    , translate
    , rotate
    , rotateAround
    , scale
    , scaleUniform
    , skewX
    , skewY
    , thenTransform
    , multiply
    , applyToPoint
    , applyToVector
    , determinant
    , invert
    , isIdentity
    , isTranslationOnly
    , toArray
    ) where

import Point2D (Point (..))
import qualified Trig

-- | A G2D01 matrix stored in SVG and Canvas order @[a, b, c, d, e, f]@.
--
-- The implicit homogeneous matrix is:
--
-- @
-- [ a c e ]
-- [ b d f ]
-- [ 0 0 1 ]
-- @
data Affine = Affine
    { affineA :: Double
    , affineB :: Double
    , affineC :: Double
    , affineD :: Double
    , affineE :: Double
    , affineF :: Double
    }
    deriving (Eq, Show)

-- | The transform that leaves every point unchanged.
identity :: Affine
identity = Affine 1.0 0.0 0.0 1.0 0.0 0.0

-- | Construct a pure translation.
translate :: Double -> Double -> Affine
translate tx ty = Affine 1.0 0.0 0.0 1.0 tx ty

-- | Construct a counterclockwise rotation by an angle in radians.
rotate :: Double -> Affine
rotate radians = Affine cosine sine (-sine) cosine 0.0 0.0
  where
    cosine = Trig.cos radians
    sine = Trig.sin radians

-- | Construct a counterclockwise rotation about an arbitrary center.
rotateAround :: Point -> Double -> Affine
rotateAround rotationCenter radians =
    thenTransform
        (thenTransform
            (translate (-pointX rotationCenter) (-pointY rotationCenter))
            (rotate radians))
        (translate (pointX rotationCenter) (pointY rotationCenter))

-- | Construct a non-uniform scale.
scale :: Double -> Double -> Affine
scale sx sy = Affine sx 0.0 0.0 sy 0.0 0.0

-- | Construct a uniform scale.
scaleUniform :: Double -> Affine
scaleUniform factor = scale factor factor

-- | Construct a horizontal shear by an angle in radians.
skewX :: Double -> Affine
skewX radians = Affine 1.0 0.0 (Trig.tan radians) 1.0 0.0 0.0

-- | Construct a vertical shear by an angle in radians.
skewY :: Double -> Affine
skewY radians = Affine 1.0 (Trig.tan radians) 0.0 1.0 0.0 0.0

-- | Compose transforms in readable order: apply the first, then the second.
-- The G2D01 name is adapted because @then@ is a Haskell keyword.
thenTransform :: Affine -> Affine -> Affine
thenTransform first next = multiply next first

-- | Matrix multiplication. The right transform is applied first.
multiply :: Affine -> Affine -> Affine
multiply self other =
    Affine
        (affineA self * affineA other + affineC self * affineB other)
        (affineB self * affineA other + affineD self * affineB other)
        (affineA self * affineC other + affineC self * affineD other)
        (affineB self * affineC other + affineD self * affineD other)
        (affineA self * affineE other + affineC self * affineF other + affineE self)
        (affineB self * affineE other + affineD self * affineF other + affineF self)

-- | Apply the full transform to a position, including translation.
applyToPoint :: Affine -> Point -> Point
applyToPoint matrix point =
    Point
        (affineA matrix * pointX point + affineC matrix * pointY point + affineE matrix)
        (affineB matrix * pointX point + affineD matrix * pointY point + affineF matrix)

-- | Apply only the linear part to a direction vector.
applyToVector :: Affine -> Point -> Point
applyToVector matrix vector =
    Point
        (affineA matrix * pointX vector + affineC matrix * pointY vector)
        (affineB matrix * pointX vector + affineD matrix * pointY vector)

-- | Return the signed area scaling factor of the linear part.
determinant :: Affine -> Double
determinant matrix =
    affineA matrix * affineD matrix - affineB matrix * affineC matrix

-- | Return the inverse, or 'Nothing' when the matrix is numerically singular.
invert :: Affine -> Maybe Affine
invert matrix
    | abs det < 1e-12 = Nothing
    | otherwise =
        Just
            (Affine
                (affineD matrix / det)
                ((-affineB matrix) / det)
                ((-affineC matrix) / det)
                (affineA matrix / det)
                ((affineC matrix * affineF matrix - affineD matrix * affineE matrix) / det)
                ((affineB matrix * affineE matrix - affineA matrix * affineF matrix) / det))
  where
    det = determinant matrix

-- | Test whether all six components are within G2D01 epsilon of identity.
isIdentity :: Affine -> Bool
isIdentity matrix =
    abs (affineA matrix - 1.0) < epsilon
        && abs (affineB matrix) < epsilon
        && abs (affineC matrix) < epsilon
        && abs (affineD matrix - 1.0) < epsilon
        && abs (affineE matrix) < epsilon
        && abs (affineF matrix) < epsilon
  where
    epsilon = 1e-10

-- | Test whether the matrix contains no rotation, scale, or shear.
isTranslationOnly :: Affine -> Bool
isTranslationOnly matrix =
    abs (affineA matrix - 1.0) < epsilon
        && abs (affineB matrix) < epsilon
        && abs (affineC matrix) < epsilon
        && abs (affineD matrix - 1.0) < epsilon
  where
    epsilon = 1e-10

-- | Return components in SVG and Canvas order.
toArray :: Affine -> [Double]
toArray matrix =
    [ affineA matrix
    , affineB matrix
    , affineC matrix
    , affineD matrix
    , affineE matrix
    , affineF matrix
    ]
