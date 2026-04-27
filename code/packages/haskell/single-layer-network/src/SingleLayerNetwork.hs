module SingleLayerNetwork
    ( ActivationName(..)
    , Matrix
    , TrainingStep(..)
    , Model(..)
    , predictWithParameters
    , trainOneEpochWithMatrices
    , newModel
    , predict
    , fit
    ) where

import Prelude hiding (predict)

data ActivationName = Linear | Sigmoid deriving (Eq, Show)

type Matrix = [[Double]]

data TrainingStep = TrainingStep
    { predictions :: Matrix
    , errors :: Matrix
    , weightGradients :: Matrix
    , biasGradients :: [Double]
    , nextWeights :: Matrix
    , nextBiases :: [Double]
    , loss :: Double
    } deriving (Eq, Show)

data Model = Model
    { modelWeights :: Matrix
    , modelBiases :: [Double]
    , modelActivation :: ActivationName
    } deriving (Eq, Show)

shape :: String -> Matrix -> Either String (Int, Int)
shape name matrix = case matrix of
    [] -> Left (name ++ " must contain at least one row")
    row:_ | null row -> Left (name ++ " must contain at least one column")
          | any ((/= length row) . length) matrix -> Left (name ++ " must be rectangular")
          | otherwise -> Right (length matrix, length row)

activate :: ActivationName -> Double -> Double
activate Linear value = value
activate Sigmoid value
    | value >= 0 =
        let z = exp (-value)
        in 1.0 / (1.0 + z)
    | otherwise =
        let z = exp value
        in z / (1.0 + z)

derivativeFromOutput :: ActivationName -> Double -> Double
derivativeFromOutput Linear _ = 1.0
derivativeFromOutput Sigmoid output = output * (1.0 - output)

predictWithParameters :: Matrix -> Matrix -> [Double] -> ActivationName -> Either String Matrix
predictWithParameters inputs weights biases activation = do
    (sampleCount, inputCount) <- shape "inputs" inputs
    (weightRows, outputCount) <- shape "weights" weights
    if inputCount /= weightRows then Left "input column count must match weight row count" else Right ()
    if length biases /= outputCount then Left "bias count must match output count" else Right ()
    Right
        [ [ activate activation (biases !! output + sum [ (inputs !! row !! input) * (weights !! input !! output) | input <- [0 .. inputCount - 1] ])
          | output <- [0 .. outputCount - 1]
          ]
        | row <- [0 .. sampleCount - 1]
        ]

trainOneEpochWithMatrices :: Matrix -> Matrix -> Matrix -> [Double] -> Double -> ActivationName -> Either String TrainingStep
trainOneEpochWithMatrices inputs targets weights biases learningRate activation = do
    (sampleCount, inputCount) <- shape "inputs" inputs
    (targetRows, outputCount) <- shape "targets" targets
    (weightRows, weightCols) <- shape "weights" weights
    if targetRows /= sampleCount then Left "inputs and targets must have the same row count" else Right ()
    if weightRows /= inputCount || weightCols /= outputCount then Left "weights must be shaped input_count x output_count" else Right ()
    if length biases /= outputCount then Left "bias count must match output count" else Right ()
    yhat <- predictWithParameters inputs weights biases activation
    let scale = 2.0 / fromIntegral (sampleCount * outputCount)
        errs =
            [ [ yhat !! row !! output - targets !! row !! output
              | output <- [0 .. outputCount - 1]
              ]
            | row <- [0 .. sampleCount - 1]
            ]
        deltas =
            [ [ scale * (errs !! row !! output) * derivativeFromOutput activation (yhat !! row !! output)
              | output <- [0 .. outputCount - 1]
              ]
            | row <- [0 .. sampleCount - 1]
            ]
        wgrads =
            [ [ sum [ inputs !! row !! input * deltas !! row !! output | row <- [0 .. sampleCount - 1] ]
              | output <- [0 .. outputCount - 1]
              ]
            | input <- [0 .. inputCount - 1]
            ]
        bgrads =
            [ sum [ deltas !! row !! output | row <- [0 .. sampleCount - 1] ]
            | output <- [0 .. outputCount - 1]
            ]
        nextW =
            [ [ weights !! input !! output - learningRate * wgrads !! input !! output
              | output <- [0 .. outputCount - 1]
              ]
            | input <- [0 .. inputCount - 1]
            ]
        nextB =
            [ biases !! output - learningRate * bgrads !! output
            | output <- [0 .. outputCount - 1]
            ]
        totalLoss = sum [ err * err | row <- errs, err <- row ] / fromIntegral (sampleCount * outputCount)
    Right TrainingStep
        { predictions = yhat
        , errors = errs
        , weightGradients = wgrads
        , biasGradients = bgrads
        , nextWeights = nextW
        , nextBiases = nextB
        , loss = totalLoss
        }

newModel :: Int -> Int -> ActivationName -> Model
newModel inputCount outputCount activation =
    Model (replicate inputCount (replicate outputCount 0.0)) (replicate outputCount 0.0) activation

predict :: Model -> Matrix -> Either String Matrix
predict model inputs = predictWithParameters inputs (modelWeights model) (modelBiases model) (modelActivation model)

fit :: Model -> Matrix -> Matrix -> Double -> Int -> Either String (Model, [TrainingStep])
fit model inputs targets learningRate epochs =
    go model [] epochs
  where
    go current history 0 = Right (current, reverse history)
    go current history remaining = do
        step <- trainOneEpochWithMatrices inputs targets (modelWeights current) (modelBiases current) learningRate (modelActivation current)
        let nextModel = current { modelWeights = nextWeights step, modelBiases = nextBiases step }
        go nextModel (step : history) (remaining - 1)
