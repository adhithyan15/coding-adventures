-- | Pure loss functions and derivatives for machine-learning examples.
module LossFunctions
    ( epsilon
    , mse
    , mae
    , bce
    , cce
    , mseDerivative
    , maeDerivative
    , bceDerivative
    , cceDerivative
    ) where

-- | Smallest probability used by the cross-entropy functions.
epsilon :: Double
epsilon = 1e-7

-- | Mean squared error.
mse :: [Double] -> [Double] -> Either String Double
mse actual predicted = do
    count <- validateInputs actual predicted
    let squaredError truth prediction = (truth - prediction) ^ (2 :: Int)
    pure $ sum (zipWith squaredError actual predicted) / fromIntegral count

-- | Mean absolute error.
mae :: [Double] -> [Double] -> Either String Double
mae actual predicted = do
    count <- validateInputs actual predicted
    pure $ sum (zipWith (\truth prediction -> abs (truth - prediction)) actual predicted)
        / fromIntegral count

-- | Binary cross-entropy, with predictions clamped away from zero and one.
bce :: [Double] -> [Double] -> Either String Double
bce actual predicted = do
    count <- validateInputs actual predicted
    let term truth prediction =
            let probability = clampProbability prediction
            in truth * log probability
                + (1.0 - truth) * log (1.0 - probability)
    pure $ negate (sum (zipWith term actual predicted) / fromIntegral count)

-- | Categorical cross-entropy for a one-hot target vector.
cce :: [Double] -> [Double] -> Either String Double
cce actual predicted = do
    count <- validateInputs actual predicted
    let term truth prediction = truth * log (clampProbability prediction)
    pure $ negate (sum (zipWith term actual predicted) / fromIntegral count)

-- | Gradient of 'mse' with respect to each prediction.
mseDerivative :: [Double] -> [Double] -> Either String [Double]
mseDerivative actual predicted = do
    count <- validateInputs actual predicted
    let scale = 2.0 / fromIntegral count
    pure $ zipWith (\truth prediction -> scale * (prediction - truth)) actual predicted

-- | Gradient of 'mae' with respect to each prediction.
maeDerivative :: [Double] -> [Double] -> Either String [Double]
maeDerivative actual predicted = do
    count <- validateInputs actual predicted
    let scale = 1.0 / fromIntegral count
        derivative truth prediction
            | prediction > truth = scale
            | prediction < truth = negate scale
            | otherwise = 0.0
    pure $ zipWith derivative actual predicted

-- | Gradient of 'bce' with respect to each prediction.
bceDerivative :: [Double] -> [Double] -> Either String [Double]
bceDerivative actual predicted = do
    count <- validateInputs actual predicted
    let scale = 1.0 / fromIntegral count
        derivative truth prediction =
            let probability = clampProbability prediction
            in scale * ((probability - truth) / (probability * (1.0 - probability)))
    pure $ zipWith derivative actual predicted

-- | Gradient of 'cce' with respect to each prediction.
cceDerivative :: [Double] -> [Double] -> Either String [Double]
cceDerivative actual predicted = do
    count <- validateInputs actual predicted
    let scale = negate (1.0 / fromIntegral count)
        derivative truth prediction = scale * truth / clampProbability prediction
    pure $ zipWith derivative actual predicted

validateInputs :: [Double] -> [Double] -> Either String Int
validateInputs actual predicted
    | null actual = Left "Inputs must have the same non-zero length"
    | length actual /= length predicted = Left "Inputs must have the same non-zero length"
    | otherwise = Right (length actual)

clampProbability :: Double -> Double
clampProbability value
    | value < epsilon = epsilon
    | value > 1.0 - epsilon = 1.0 - epsilon
    | otherwise = value
