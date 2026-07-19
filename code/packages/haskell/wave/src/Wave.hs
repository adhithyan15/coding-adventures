-- | A validated simple-harmonic wave model.
module Wave
    ( Wave
    , newWave
    , newWaveWithPhase
    , amplitude
    , frequency
    , phase
    , period
    , angularFrequency
    , evaluate
    ) where

import qualified Trig

-- | A sinusoidal wave whose amplitude and frequency have been validated.
data Wave = Wave
    { amplitude :: Double
    , frequency :: Double
    , phase :: Double
    } deriving (Eq, Show)

-- | Construct a wave with zero phase.
newWave :: Double -> Double -> Either String Wave
newWave waveAmplitude waveFrequency =
    newWaveWithPhase waveAmplitude waveFrequency 0.0

-- | Construct a wave with an explicit phase in radians.
newWaveWithPhase :: Double -> Double -> Double -> Either String Wave
newWaveWithPhase waveAmplitude waveFrequency wavePhase
    | waveAmplitude < 0.0 = Left "amplitude must be non-negative"
    | waveFrequency <= 0.0 = Left "frequency must be positive"
    | otherwise = Right Wave
        { amplitude = waveAmplitude
        , frequency = waveFrequency
        , phase = wavePhase
        }

-- | Duration of one complete cycle in seconds.
period :: Wave -> Double
period wave = 1.0 / frequency wave

-- | Angular frequency in radians per second.
angularFrequency :: Wave -> Double
angularFrequency wave = 2.0 * Trig.piValue * frequency wave

-- | Evaluate the wave at time @t@ in seconds.
evaluate :: Wave -> Double -> Double
evaluate wave time = amplitude wave * Trig.sin angle
  where
    angle = angularFrequency wave * time + phase wave
