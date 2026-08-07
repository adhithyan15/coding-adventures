-- | Pure neural-network activation functions and their derivatives.
module ActivationFunctions
    ( leakyReluSlope
    , linear
    , linearDerivative
    , sigmoid
    , sigmoidDerivative
    , relu
    , reluDerivative
    , leakyRelu
    , leakyReluDerivative
    , tanh
    , tanhDerivative
    , softplus
    , softplusDerivative
    ) where

import Prelude hiding (tanh)
import qualified Prelude as P

-- | The slope applied to non-positive inputs by 'leakyRelu'.
leakyReluSlope :: Double
leakyReluSlope = 0.01

-- | Identity activation, commonly used for regression outputs.
linear :: Double -> Double
linear x = x

-- | Derivative of 'linear'.
linearDerivative :: Double -> Double
linearDerivative _ = 1.0

-- | Logistic sigmoid with guards around the finite Double exponent range.
sigmoid :: Double -> Double
sigmoid x
    | x < -709.0 = 0.0
    | x > 709.0 = 1.0
    | otherwise = 1.0 / (1.0 + exp (-x))

-- | Derivative of 'sigmoid'.
sigmoidDerivative :: Double -> Double
sigmoidDerivative x = s * (1.0 - s)
  where
    s = sigmoid x

-- | Rectified linear unit.
relu :: Double -> Double
relu x
    | x > 0.0 = x
    | otherwise = 0.0

-- | Derivative of 'relu', defined as zero at the origin.
reluDerivative :: Double -> Double
reluDerivative x
    | x > 0.0 = 1.0
    | otherwise = 0.0

-- | ReLU with a small gradient for non-positive inputs.
leakyRelu :: Double -> Double
leakyRelu x
    | x > 0.0 = x
    | otherwise = leakyReluSlope * x

-- | Derivative of 'leakyRelu'.
leakyReluDerivative :: Double -> Double
leakyReluDerivative x
    | x > 0.0 = 1.0
    | otherwise = leakyReluSlope

-- | Hyperbolic tangent activation.
tanh :: Double -> Double
tanh = P.tanh

-- | Derivative of 'tanh'.
tanhDerivative :: Double -> Double
tanhDerivative x = 1.0 - t * t
  where
    t = tanh x

-- | Stable softplus: @log (1 + exp x)@ without overflow for large @x@.
softplus :: Double -> Double
softplus x = log1p (exp (-abs x)) + max x 0.0

-- | Derivative of 'softplus'.
softplusDerivative :: Double -> Double
softplusDerivative = sigmoid

-- | Accurate @log (1 + value)@ for the small positive values used by
-- 'softplus'. The correction recovers bits lost when @1 + value@ rounds.
log1p :: Double -> Double
log1p value
    | w == 1.0 = value
    | otherwise = log w * (value / (w - 1.0))
  where
    w = 1.0 + value
