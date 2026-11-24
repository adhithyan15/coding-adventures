-- |
-- Module      : LogicGates.Basic
-- Description : The seven fundamental logic gates plus four gates built only from NAND.
--
-- A logic gate is the smallest decision-maker in a computer.  It looks at one
-- or two bits (a bit is a 0 or a 1) and produces one bit.  Everything a CPU
-- does -- adding, comparing, remembering -- is built from these.
--
-- This module has two halves:
--
--   1. The seven fundamental gates: NOT, AND, OR, XOR, NAND, NOR, XNOR.
--   2. Four gates (NOT, AND, OR, XOR) rebuilt using /nothing but NAND/.
--      That NAND alone is enough to build every other gate is called
--      "functional completeness", and it is why real chips are largely
--      made of NAND (or NOR) transistors.
--
-- Error model
-- -----------
-- Bits are plain 'Int's, so the type system cannot stop a caller passing 2 or
-- -1.  Every gate therefore validates every argument and returns
-- @Either String Bit@: 'Left' with a message naming the offending value, or
-- 'Right' with the output bit.  The NAND-built gates validate too, even
-- though they delegate to 'nandGate', so a bad input is rejected up front.
module LogicGates.Basic
    ( Bit
    , validateBit
    , notGate
    , andGate
    , orGate
    , xorGate
    , nandGate
    , norGate
    , xnorGate
    , nandNot
    , nandAnd
    , nandOr
    , nandXor
    ) where

-- | A bit.  Only 0 and 1 are valid; everything else is rejected by
-- 'validateBit'.
type Bit = Int

-- | Check that a value is a real bit.
--
-- >>> validateBit 1
-- Right 1
-- >>> validateBit 2
-- Left "Invalid bit: 2"
validateBit :: Bit -> Either String Bit
validateBit b
    | b == 0 || b == 1 = Right b
    | otherwise = Left $ "Invalid bit: " ++ show b

-- ---------------------------------------------------------------------------
-- Fundamental gates
-- ---------------------------------------------------------------------------

-- | NOT: flips the bit.
--
-- > A | out
-- > --+----
-- > 0 | 1
-- > 1 | 0
notGate :: Bit -> Either String Bit
notGate a = do
    a' <- validateBit a
    return $ if a' == 1 then 0 else 1

-- | AND: 1 only when /both/ inputs are 1.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 0
-- > 0 1 | 0
-- > 1 0 | 0
-- > 1 1 | 1
andGate :: Bit -> Bit -> Either String Bit
andGate a b = do
    a' <- validateBit a
    b' <- validateBit b
    return $ if a' == 1 && b' == 1 then 1 else 0

-- | OR: 1 when /at least one/ input is 1.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 0
-- > 0 1 | 1
-- > 1 0 | 1
-- > 1 1 | 1
orGate :: Bit -> Bit -> Either String Bit
orGate a b = do
    a' <- validateBit a
    b' <- validateBit b
    return $ if a' == 1 || b' == 1 then 1 else 0

-- | XOR (exclusive or): 1 when the inputs /differ/.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 0
-- > 0 1 | 1
-- > 1 0 | 1
-- > 1 1 | 0
xorGate :: Bit -> Bit -> Either String Bit
xorGate a b = do
    a' <- validateBit a
    b' <- validateBit b
    return $ if a' /= b' then 1 else 0

-- | NAND: AND followed by NOT.  0 only when both inputs are 1.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 1
-- > 0 1 | 1
-- > 1 0 | 1
-- > 1 1 | 0
nandGate :: Bit -> Bit -> Either String Bit
nandGate a b = do
    res <- andGate a b
    notGate res

-- | NOR: OR followed by NOT.  1 only when both inputs are 0.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 1
-- > 0 1 | 0
-- > 1 0 | 0
-- > 1 1 | 0
norGate :: Bit -> Bit -> Either String Bit
norGate a b = do
    res <- orGate a b
    notGate res

-- | XNOR (equivalence): XOR followed by NOT.  1 when the inputs are /equal/.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 1
-- > 0 1 | 0
-- > 1 0 | 0
-- > 1 1 | 1
xnorGate :: Bit -> Bit -> Either String Bit
xnorGate a b = do
    res <- xorGate a b
    notGate res

-- ---------------------------------------------------------------------------
-- Gates built from NAND only
-- ---------------------------------------------------------------------------
--
-- Each function below uses 'nandGate' and nothing else for its logic.  The
-- only other thing it does is call 'validateBit' first, so that a bad input
-- is reported up front, before any wiring happens.

-- | NOT from one NAND, with both inputs tied together.
--
-- > a --+--\
-- >     |   NAND --- out      NAND(a,a) = NOT(a)
-- >     +--/
--
-- > a | NAND(a,a)
-- > --+----------
-- > 0 | 1
-- > 1 | 0
nandNot :: Bit -> Either String Bit
nandNot a = do
    a' <- validateBit a
    nandGate a' a'

-- | AND from two NANDs: a NAND, then invert it with a second NAND.
--
-- > a --\
-- >      NAND --- t --+--\
-- > b --/             |   NAND --- out    out = NOT(NAND(a,b)) = AND(a,b)
-- >                   +--/
nandAnd :: Bit -> Bit -> Either String Bit
nandAnd a b = do
    a' <- validateBit a
    b' <- validateBit b
    t <- nandGate a' b'
    nandGate t t

-- | OR from three NANDs, using De Morgan: @a OR b = NOT(NOT a AND NOT b)@.
--
-- > a --+--\
-- >     |   NAND --- na --\
-- >     +--/               \
-- >                         NAND --- out
-- >     +--\               /
-- >     |   NAND --- nb --/
-- > b --+--/
nandOr :: Bit -> Bit -> Either String Bit
nandOr a b = do
    a' <- validateBit a
    b' <- validateBit b
    na <- nandGate a' a'
    nb <- nandGate b' b'
    nandGate na nb

-- | XOR from four NANDs -- the classic construction.
--
-- > n1  = NAND(a, b)
-- > n2  = NAND(a, n1)
-- > n3  = NAND(b, n1)
-- > out = NAND(n2, n3)
--
-- The shared n1 feeds both middle gates:
--
-- > a --+-------------\
-- >     |              NAND --- n2 --\
-- >     |     +-------/               \
-- >     |     |                        NAND --- out
-- >     |   n1|                       /
-- >     |     +-------\               /
-- > b --+-------------NAND --- n3 ---/
--
-- Worked check for a=1, b=1: n1=0, n2=NAND(1,0)=1, n3=NAND(1,0)=1,
-- out=NAND(1,1)=0, which is XOR(1,1).
nandXor :: Bit -> Bit -> Either String Bit
nandXor a b = do
    a' <- validateBit a
    b' <- validateBit b
    n1 <- nandGate a' b'
    n2 <- nandGate a' n1
    n3 <- nandGate b' n1
    nandGate n2 n3
