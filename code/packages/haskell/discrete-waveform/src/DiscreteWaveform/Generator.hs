module DiscreteWaveform.Generator
    ( Generator(..)
    , newSquareWave
    , newPWM
    , generate
    ) where

import DiscreteWaveform.Waveform

data Generator = SquareWave Int
               | PWM Int Int
    deriving (Show, Eq)

newSquareWave :: Int -> Generator
newSquareWave period = SquareWave period

newPWM :: Int -> Int -> Generator
newPWM period duty = PWM period duty

generate :: Generator -> Int -> Int -> Int -> Waveform Int
generate gen startTime endTime step =
    let times = [startTime, startTime+step .. endTime]
        vals = map (computeVal gen) times
    in Waveform endTime (zip times vals)

computeVal :: Generator -> Int -> Int
computeVal (SquareWave period) t =
    let phase = t `mod` period
    in if phase < (period `div` 2) then 1 else 0

computeVal (PWM period duty) t =
    let phase = t `mod` period
        threshold = (period * duty) `div` 100
    in if phase < threshold then 1 else 0
