-- | Pure gradient-descent optimizers for machine-learning examples.
module GradientDescent
    ( sgd
    ) where

-- | Apply one stochastic-gradient-descent update.
--
-- The result is a newly constructed vector whose elements are
-- @weight - learningRate * gradient@. Empty vectors and mismatched vector
-- lengths are rejected explicitly.
sgd :: [Double] -> [Double] -> Double -> Either String [Double]
sgd weights gradients learningRate
    | null weights = Left "Weights and gradients must have the same non-zero length"
    | length weights /= length gradients =
        Left "Weights and gradients must have the same non-zero length"
    | otherwise = Right $ zipWith update weights gradients
  where
    update weight gradient = weight - learningRate * gradient
