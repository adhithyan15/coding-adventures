module TwoLayerNetwork
    ( ActivationName(..)
    , Matrix
    , Parameters(..)
    , ForwardPass(..)
    , TrainingStep(..)
    , xorWarmStartParameters
    , forward
    , trainOneEpoch
    ) where

data ActivationName = Linear | Sigmoid deriving (Eq, Show)

type Matrix = [[Double]]

data Parameters = Parameters
    { inputToHiddenWeights :: Matrix
    , hiddenBiases :: [Double]
    , hiddenToOutputWeights :: Matrix
    , outputBiases :: [Double]
    } deriving (Eq, Show)

data ForwardPass = ForwardPass
    { hiddenRaw :: Matrix
    , hiddenActivations :: Matrix
    , outputRaw :: Matrix
    , predictions :: Matrix
    } deriving (Eq, Show)

data TrainingStep = TrainingStep
    { stepPredictions :: Matrix
    , errors :: Matrix
    , outputDeltas :: Matrix
    , hiddenDeltas :: Matrix
    , hiddenToOutputWeightGradients :: Matrix
    , outputBiasGradients :: [Double]
    , inputToHiddenWeightGradients :: Matrix
    , hiddenBiasGradients :: [Double]
    , nextParameters :: Parameters
    , loss :: Double
    } deriving (Eq, Show)

xorWarmStartParameters :: Parameters
xorWarmStartParameters = Parameters
    { inputToHiddenWeights = [[4.0, -4.0], [4.0, -4.0]]
    , hiddenBiases = [-2.0, 6.0]
    , hiddenToOutputWeights = [[4.0], [4.0]]
    , outputBiases = [-6.0]
    }

activate :: ActivationName -> Double -> Double
activate Linear value = value
activate Sigmoid value
    | value >= 0 =
        let z = exp (-value)
        in 1.0 / (1.0 + z)
    | otherwise =
        let z = exp value
        in z / (1.0 + z)

derivative :: ActivationName -> Double -> Double -> Double
derivative Linear _ _ = 1.0
derivative Sigmoid _ activated = activated * (1.0 - activated)

dot :: Matrix -> Matrix -> Matrix
dot left right =
    [ [ sum [ (left !! row !! k) * (right !! k !! col) | k <- [0 .. width - 1] ]
      | col <- [0 .. cols - 1]
      ]
    | row <- [0 .. rows - 1]
    ]
  where
    rows = length left
    width = length (head left)
    cols = length (head right)

transposeM :: Matrix -> Matrix
transposeM matrix =
    [ [ matrix !! row !! col | row <- [0 .. rows - 1] ]
    | col <- [0 .. cols - 1]
    ]
  where
    rows = length matrix
    cols = length (head matrix)

addBiases :: Matrix -> [Double] -> Matrix
addBiases matrix biases =
    [ [ value + biases !! col | (col, value) <- zip [0..] row ]
    | row <- matrix
    ]

applyActivation :: ActivationName -> Matrix -> Matrix
applyActivation activation matrix = map (map (activate activation)) matrix

columnSums :: Matrix -> [Double]
columnSums matrix =
    [ sum [ row !! col | row <- matrix ]
    | col <- [0 .. length (head matrix) - 1]
    ]

subtractScaled :: Matrix -> Matrix -> Double -> Matrix
subtractScaled matrix gradients learningRate =
    [ [ matrix !! row !! col - learningRate * gradients !! row !! col
      | col <- [0 .. length (matrix !! row) - 1]
      ]
    | row <- [0 .. length matrix - 1]
    ]

subtractScaledVector :: [Double] -> [Double] -> Double -> [Double]
subtractScaledVector values gradients learningRate =
    [ value - learningRate * gradients !! index | (index, value) <- zip [0..] values ]

mse :: Matrix -> Double
mse errs = sum [ value * value | row <- errs, value <- row ] / fromIntegral (sum (map length errs))

forward :: Matrix -> Parameters -> ActivationName -> ActivationName -> ForwardPass
forward inputs parameters hiddenActivation outputActivation =
    ForwardPass hiddenRaw' hiddenActivations' outputRaw' predictions'
  where
    hiddenRaw' = addBiases (dot inputs (inputToHiddenWeights parameters)) (hiddenBiases parameters)
    hiddenActivations' = applyActivation hiddenActivation hiddenRaw'
    outputRaw' = addBiases (dot hiddenActivations' (hiddenToOutputWeights parameters)) (outputBiases parameters)
    predictions' = applyActivation outputActivation outputRaw'

trainOneEpoch :: Matrix -> Matrix -> Parameters -> Double -> ActivationName -> ActivationName -> TrainingStep
trainOneEpoch inputs targets parameters learningRate hiddenActivation outputActivation =
    TrainingStep yhat errs outDeltas hidDeltas h2oGrads outBiasGrads i2hGrads hidBiasGrads nextParams (mse errs)
  where
    pass = forward inputs parameters hiddenActivation outputActivation
    yhat = predictions pass
    sampleCount = length inputs
    outputCount = length (head targets)
    scale = 2.0 / fromIntegral (sampleCount * outputCount)
    errs =
        [ [ yhat !! row !! output - targets !! row !! output
          | output <- [0 .. outputCount - 1]
          ]
        | row <- [0 .. sampleCount - 1]
        ]
    outDeltas =
        [ [ scale * errs !! row !! output * derivative outputActivation (outputRaw pass !! row !! output) (yhat !! row !! output)
          | output <- [0 .. outputCount - 1]
          ]
        | row <- [0 .. sampleCount - 1]
        ]
    h2oGrads = dot (transposeM (hiddenActivations pass)) outDeltas
    outBiasGrads = columnSums outDeltas
    hiddenErrors = dot outDeltas (transposeM (hiddenToOutputWeights parameters))
    hiddenWidth = length (hiddenBiases parameters)
    hidDeltas =
        [ [ hiddenErrors !! row !! hidden * derivative hiddenActivation (hiddenRaw pass !! row !! hidden) (hiddenActivations pass !! row !! hidden)
          | hidden <- [0 .. hiddenWidth - 1]
          ]
        | row <- [0 .. sampleCount - 1]
        ]
    i2hGrads = dot (transposeM inputs) hidDeltas
    hidBiasGrads = columnSums hidDeltas
    nextParams = Parameters
        { inputToHiddenWeights = subtractScaled (inputToHiddenWeights parameters) i2hGrads learningRate
        , hiddenBiases = subtractScaledVector (hiddenBiases parameters) hidBiasGrads learningRate
        , hiddenToOutputWeights = subtractScaled (hiddenToOutputWeights parameters) h2oGrads learningRate
        , outputBiases = subtractScaledVector (outputBiases parameters) outBiasGrads learningRate
        }
