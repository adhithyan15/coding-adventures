module DiscreteWaveform.Waveform
    ( Waveform(..)
    , newWaveform
    , sample
    , addSample
    ) where

data Waveform a = Waveform
    { wfCurrentTime :: Int
    , wfSamples     :: [(Int, a)]
    } deriving (Show, Eq)

newWaveform :: Waveform a
newWaveform = Waveform 0 []

sample :: Waveform a -> Int -> Maybe a
sample wf t = 
    let past = filter (\(time, _) -> time <= t) (wfSamples wf)
    in if null past then Nothing else Just (snd $ last past)

addSample :: Waveform a -> Int -> a -> Waveform a
addSample wf t val =
    let newSamples = wfSamples wf ++ [(t, val)]
    in wf { wfSamples = newSamples, wfCurrentTime = t }
