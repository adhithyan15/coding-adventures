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
import Data.Bits (shiftL)

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
    | not (isFinite waveAmplitude && isFinite waveFrequency && isFinite wavePhase) =
        Left "wave parameters must be finite"
    | waveAmplitude < 0.0 = Left "amplitude must be non-negative"
    | waveFrequency <= 0.0 = Left "frequency must be positive"
    | waveFrequency > maxFinite / Trig.twoPi = Left "angular frequency must be finite"
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
evaluate wave time
    | not (isFinite time) = error "time must be finite"
    | amplitude wave == 0.0 = 0.0
    | otherwise = amplitude wave * clampedUnit
  where
    reducedTime = signedRemainder time (period wave)
    reducedPhase = signedRemainder (phase wave) Trig.twoPi
    angle = Trig.twoPi * (frequency wave * reducedTime) + reducedPhase
    clampedUnit = max (-1.0) (min 1.0 (Trig.sin angle))

maxFinite :: Double
maxFinite = 1.7976931348623157e308

isFinite :: Double -> Bool
isFinite value = not (isNaN value || isInfinite value)

-- | Base-only signed remainder that never forms the potentially overflowing
-- quotient @value / modulus@. Both inputs are represented exactly as an
-- Integer mantissa times a power of two, aligned, and reduced with 'rem'.
signedRemainder :: Double -> Double -> Double
signedRemainder value modulus
    | isInfinite modulus = value
    | value == 0.0 = value
    | otherwise =
        encodeFloat (scaledValue `rem` scaledModulus) commonExponent
  where
    (valueMantissa, valueExponent) = decodeFloat value
    (modulusMantissa, modulusExponent) = decodeFloat modulus
    commonExponent = min valueExponent modulusExponent
    scaledValue = valueMantissa `shiftL` (valueExponent - commonExponent)
    scaledModulus = modulusMantissa `shiftL` (modulusExponent - commonExponent)
