-- | A pure single-neuron binary classifier.
module Perceptron
    ( Perceptron
    , learningRate
    , epochs
    , weights
    , bias
    , new
    , defaultPerceptron
    , fit
    , fitColumnLabels
    , predict
    ) where

import ActivationFunctions (sigmoid, sigmoidDerivative)
import LossFunctions (bceDerivative)
import qualified Matrix as M

-- | Model configuration and learned parameters.
--
-- A newly constructed model has no weights. 'fit' returns a new trained model
-- rather than mutating the original value.
data Perceptron = Perceptron
    { learningRate :: Double
    , epochs :: Int
    , weights :: Maybe [Double]
    , bias :: Double
    }
    deriving (Eq, Show)

-- | Construct a model with validated hyperparameters.
new :: Double -> Int -> Either String Perceptron
new rate epochCount
    | not (isFinite rate) = Left "Learning rate must be finite"
    | epochCount < 0 = Left "Epochs must be non-negative"
    | otherwise = Right (Perceptron rate epochCount Nothing 0.0)

-- | A model configured with a learning rate of @0.1@ and @2000@ epochs.
defaultPerceptron :: Perceptron
defaultPerceptron = Perceptron 0.1 2000 Nothing 0.0

-- | Fit a model against one scalar label per sample.
--
-- Training begins from zero weights and zero bias on every call. As in the
-- other language implementations, epoch zero is an update, so a model with
-- zero configured epochs performs one gradient step.
fit :: Perceptron -> [[Double]] -> [Double] -> Either String Perceptron
fit model featureRows labels = do
    features <- validateFeatures featureRows
    validateLabels (M.rows features) labels
    initialWeights <- M.zeros (M.cols features) 1
    (trainedWeights, trainedBias) <- train 0 initialWeights 0.0 features labels
    pure
        model
            { weights = Just (map head (M.toRows trainedWeights))
            , bias = trainedBias
            }
  where
    train epoch currentWeights currentBias features trainingLabels = do
        raw <- M.dot features currentWeights
        let rawWithBias = M.addScalar raw currentBias
            rawValues = map head (M.toRows rawWithBias)
            predictions = map sigmoid rawValues
        lossGradient <- bceDerivative trainingLabels predictions
        let combinedGradient =
                zipWith (*) lossGradient (map sigmoidDerivative rawValues)
            biasGradient = sum combinedGradient
        gradientColumn <- M.fromRows (map (: []) combinedGradient)
        weightGradient <- M.dot (M.transpose features) gradientColumn
        nextWeights <-
            M.subtract
                currentWeights
                (M.scale weightGradient (learningRate model))
        let nextBias = currentBias - learningRate model * biasGradient
        if epoch == epochs model
            then Right (nextWeights, nextBias)
            else train (epoch + 1) nextWeights nextBias features trainingLabels

-- | Fit a model against labels represented as one-column rows.
fitColumnLabels :: Perceptron -> [[Double]] -> [[Double]] -> Either String Perceptron
fitColumnLabels model featureRows labelRows = do
    labels <- traverse flattenLabel labelRows
    fit model featureRows labels
  where
    flattenLabel [value] = Right value
    flattenLabel _ = Left "Column labels must have exactly one value per row"

-- | Predict the positive-class probability for every sample.
predict :: Perceptron -> [[Double]] -> Either String [Double]
predict model featureRows = case weights model of
    Nothing -> Left "Perceptron has not been trained yet. Call fit first"
    Just learnedWeights -> do
        features <- validateFeatures featureRows
        if M.cols features /= length learnedWeights
            then
                Left
                    ( "Feature width "
                        ++ show (M.cols features)
                        ++ " does not match trained width "
                        ++ show (length learnedWeights)
                    )
            else do
                weightColumn <- M.fromRows (map (: []) learnedWeights)
                raw <- M.dot features weightColumn
                pure $ map (sigmoid . (+ bias model) . head) (M.toRows raw)

validateFeatures :: [[Double]] -> Either String M.Matrix
validateFeatures [] = Left "Feature data must contain at least one sample"
validateFeatures featureRows@(firstRow : remainingRows)
    | null firstRow = Left "Samples must contain at least one feature"
    | any ((/= length firstRow) . length) remainingRows =
        Left "All samples must have the same number of features"
    | any (not . isFinite) (concat featureRows) =
        Left "Feature values must be finite"
    | otherwise = M.fromRows featureRows

validateLabels :: Int -> [Double] -> Either String ()
validateLabels expectedRows labels
    | null labels || length labels /= expectedRows =
        Left "Labels must match the non-zero sample count"
    | any (not . isFinite) labels = Left "Labels must be finite"
    | otherwise = Right ()

isFinite :: Double -> Bool
isFinite value = not (isNaN value || isInfinite value)
