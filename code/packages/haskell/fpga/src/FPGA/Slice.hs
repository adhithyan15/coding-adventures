module FPGA.Slice
    ( SliceOutput(..)
    , Slice(..)
    , newSlice
    , configureSlice
    , evaluateSlice
    ) where

import LogicGates
import FPGA.LUT

data SliceOutput = SliceOutput
    { outputA  :: Bit
    , outputB  :: Bit
    , carryOut :: Bit
    } deriving (Show, Eq)

data Slice = Slice
    { sliceK      :: Int
    , sliceLutA   :: LUT
    , sliceLutB   :: LUT
    , sliceFFA    :: FlipFlopState
    , sliceFFB    :: FlipFlopState
    , sliceFFAEn  :: Bool
    , sliceFFBEn  :: Bool
    , sliceCarryEn :: Bool
    } deriving (Show, Eq)

newSlice :: Int -> Either String Slice
newSlice k = do
    lutA <- newLUT k
    lutB <- newLUT k
    let ff0 = FlipFlopState 0 1 0
    return $ Slice k lutA lutB ff0 ff0 False False False

configureSlice :: Slice -> [Bit] -> [Bit] -> Bool -> Bool -> Bool -> Either String Slice
configureSlice slice ttA ttB ffAEn ffBEn cEn = do
    lA <- configureLUT (sliceLutA slice) ttA
    lB <- configureLUT (sliceLutB slice) ttB
    let ff0 = FlipFlopState 0 1 0
    return $ slice { sliceLutA = lA, sliceLutB = lB, sliceFFAEn = ffAEn, sliceFFBEn = ffBEn, sliceCarryEn = cEn, sliceFFA = ff0, sliceFFB = ff0 }

evaluateSlice :: Slice -> [Bit] -> [Bit] -> Bit -> Bit -> Either String (Slice, SliceOutput)
evaluateSlice slice inA inB clock cIn = do
    lutAOut <- evaluateLUT (sliceLutA slice) inA
    lutBOut <- evaluateLUT (sliceLutB slice) inB
    
    (nFFA, outA) <- if sliceFFAEn slice
                    then do
                         let ff = sliceFFA slice
                         nstate <- dFlipFlop lutAOut clock (ffQ ff) (ffQBar ff) (ffMasterQ ff) (1 - ffMasterQ ff)
                         aOut <- mux2 lutAOut (ffQ nstate) 1
                         return (nstate, aOut)
                    else return (sliceFFA slice, lutAOut)
                    
    (nFFB, outB) <- if sliceFFBEn slice
                    then do
                         let ff = sliceFFB slice
                         nstate <- dFlipFlop lutBOut clock (ffQ ff) (ffQBar ff) (ffMasterQ ff) (1 - ffMasterQ ff)
                         bOut <- mux2 lutBOut (ffQ nstate) 1
                         return (nstate, bOut)
                    else return (sliceFFB slice, lutBOut)
                    
    cOut <- if sliceCarryEn slice
            then do
                 t1 <- andGate lutAOut lutBOut
                 axb <- xorGate lutAOut lutBOut
                 t2 <- andGate cIn axb
                 orGate t1 t2
            else return 0
            
    return (slice { sliceFFA = nFFA, sliceFFB = nFFB }, SliceOutput outA outB cOut)
