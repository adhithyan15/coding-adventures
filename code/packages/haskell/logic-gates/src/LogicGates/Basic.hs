module LogicGates.Basic
    ( Bit
    , validateBit
    , andGate
    , orGate
    , notGate
    , xorGate
    , nandGate
    , norGate
    ) where

type Bit = Int

validateBit :: Bit -> Either String Bit
validateBit b
    | b == 0 || b == 1 = Right b
    | otherwise = Left $ "Invalid bit: " ++ show b

andGate :: Bit -> Bit -> Either String Bit
andGate a b = do
    a' <- validateBit a
    b' <- validateBit b
    return $ if a' == 1 && b' == 1 then 1 else 0

orGate :: Bit -> Bit -> Either String Bit
orGate a b = do
    a' <- validateBit a
    b' <- validateBit b
    return $ if a' == 1 || b' == 1 then 1 else 0

notGate :: Bit -> Either String Bit
notGate a = do
    a' <- validateBit a
    return $ if a' == 1 then 0 else 1

xorGate :: Bit -> Bit -> Either String Bit
xorGate a b = do
    a' <- validateBit a
    b' <- validateBit b
    return $ if a' /= b' then 1 else 0

nandGate :: Bit -> Bit -> Either String Bit
nandGate a b = do
    res <- andGate a b
    notGate res

norGate :: Bit -> Bit -> Either String Bit
norGate a b = do
    res <- orGate a b
    notGate res
