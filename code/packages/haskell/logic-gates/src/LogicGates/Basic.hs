{-# LANGUAGE Safe #-}
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
-- @Either LogicGateError Bit@: 'Left' with an 'InvalidBit' naming the
-- offending parameter and value, or 'Right' with the output bit.  The error
-- is a structured value, never message text, so callers match on constructors
-- (LG02 section 7).  The first invalid parameter, in argument order, is the
-- one reported, and it is always named by the public gate's own parameter
-- (@a@, @b@) -- never by an inner NAND wire.  The NAND-built gates validate
-- up front, before any wiring happens.
module LogicGates.Basic
    ( Bit
    , LogicGateError(..)
    , validateBit
    , validateBits
    , takeChecked
    , forceBits
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

-- | Why a gate or circuit rejected its input.  Match on the constructor and
-- fields; never on rendered text.
data LogicGateError
    = InvalidBit { param :: String, value :: Int }
      -- ^ A bit argument was neither 0 nor 1.  'param' is the public
      -- parameter name; 'value' is the offending integer.
    | InvalidShape String
      -- ^ A list argument had the wrong length or arrangement.  The string
      -- names the rule that was broken.
    deriving (Eq, Show)

-- | Check that a value is a real bit, naming the parameter for the error.
--
-- >>> validateBit "a" 1
-- Right 1
-- >>> validateBit "a" 2
-- Left (InvalidBit {param = "a", value = 2})
validateBit :: String -> Bit -> Either LogicGateError Bit
validateBit name b
    | b == 0 || b == 1 = Right b
    | otherwise = Left (InvalidBit (boundName name) b)

-- | Parameter names flow into 'InvalidBit', so a caller-chosen name must not
-- be able to inject an unbounded or control-character-laden string into an
-- error value.  Names are truncated to 64 characters and control characters
-- become @?@.  Every internal name is short printable ASCII, so it is
-- unchanged.
boundName :: String -> String
boundName = map (\c -> if c < ' ' || c == '\DEL' then '?' else c) . take 64

-- | Check every element of a list (at most 65536 of them; longer or infinite
-- lists are an 'InvalidShape'), reporting the first bad one under the
-- list's own parameter name.
validateBits :: String -> [Bit] -> Either LogicGateError [Bit]
validateBits name xs = takeChecked name maxBitsLength xs >>= mapM (validateBit name)

-- | Longest list 'validateBits' accepts (@2 ^ 16@).  Longer or infinite
-- lists are rejected with 'InvalidShape' instead of being walked.
maxBitsLength :: Int
maxBitsLength = 65536

-- | Bounded, fully-forced snapshot of a caller-supplied list.
--
-- Haskell lists can be lazy and infinite (@cycle [0]@) or astronomically
-- long.  A bare 'length' on such a list never returns, and because the loop
-- does not allocate, @System.Timeout@ cannot interrupt it.  So every
-- list-taking function snapshots its input through this function FIRST: it
-- walks at most @cap + 1@ cells, rejects the list with 'InvalidShape' if it
-- is longer than @cap@, and otherwise returns a list whose spine and
-- elements have all been evaluated, so no thunk can fire later -- for any
-- total input.  A bottom element supplied by the caller is the caller's own
-- bottom and is raised here, when the snapshot is taken.
--
-- >>> takeChecked "sel" 2 [1, 0]
-- Right [1,0]
-- >>> takeChecked "sel" 2 (cycle [0])
-- Left (InvalidShape "sel must have at most 2 elements")
takeChecked :: String -> Int -> [Int] -> Either LogicGateError [Int]
takeChecked name cap xs =
    let prefix = take (cap + 1) xs
        n = length prefix
    in if n > cap
       then Left (InvalidShape (name ++ " must have at most " ++ show cap ++ " elements"))
       else forceBits prefix `seq` Right prefix

-- | Evaluate every element of a list (the spine is already forced by
-- consuming it) and hand the list back.
forceBits :: [Int] -> [Int]
forceBits xs = foldr seq () xs `seq` xs

-- ---------------------------------------------------------------------------
-- Fundamental gates
-- ---------------------------------------------------------------------------

-- | NOT: flips the bit.
--
-- > A | out
-- > --+----
-- > 0 | 1
-- > 1 | 0
notGate :: Bit -> Either LogicGateError Bit
notGate a = do
    a' <- validateBit "a" a
    return $ if a' == 1 then 0 else 1

-- | AND: 1 only when /both/ inputs are 1.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 0
-- > 0 1 | 0
-- > 1 0 | 0
-- > 1 1 | 1
andGate :: Bit -> Bit -> Either LogicGateError Bit
andGate a b = do
    a' <- validateBit "a" a
    b' <- validateBit "b" b
    return $ if a' == 1 && b' == 1 then 1 else 0

-- | OR: 1 when /at least one/ input is 1.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 0
-- > 0 1 | 1
-- > 1 0 | 1
-- > 1 1 | 1
orGate :: Bit -> Bit -> Either LogicGateError Bit
orGate a b = do
    a' <- validateBit "a" a
    b' <- validateBit "b" b
    return $ if a' == 1 || b' == 1 then 1 else 0

-- | XOR (exclusive or): 1 when the inputs /differ/.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 0
-- > 0 1 | 1
-- > 1 0 | 1
-- > 1 1 | 0
xorGate :: Bit -> Bit -> Either LogicGateError Bit
xorGate a b = do
    a' <- validateBit "a" a
    b' <- validateBit "b" b
    return $ if a' /= b' then 1 else 0

-- | NAND: AND followed by NOT.  0 only when both inputs are 1.
--
-- > A B | out
-- > ----+----
-- > 0 0 | 1
-- > 0 1 | 1
-- > 1 0 | 1
-- > 1 1 | 0
nandGate :: Bit -> Bit -> Either LogicGateError Bit
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
norGate :: Bit -> Bit -> Either LogicGateError Bit
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
xnorGate :: Bit -> Bit -> Either LogicGateError Bit
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
nandNot :: Bit -> Either LogicGateError Bit
nandNot a = do
    a' <- validateBit "a" a
    nandGate a' a'

-- | AND from two NANDs: a NAND, then invert it with a second NAND.
--
-- > a --\
-- >      NAND --- t --+--\
-- > b --/             |   NAND --- out    out = NOT(NAND(a,b)) = AND(a,b)
-- >                   +--/
nandAnd :: Bit -> Bit -> Either LogicGateError Bit
nandAnd a b = do
    a' <- validateBit "a" a
    b' <- validateBit "b" b
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
nandOr :: Bit -> Bit -> Either LogicGateError Bit
nandOr a b = do
    a' <- validateBit "a" a
    b' <- validateBit "b" b
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
nandXor :: Bit -> Bit -> Either LogicGateError Bit
nandXor a b = do
    a' <- validateBit "a" a
    b' <- validateBit "b" b
    n1 <- nandGate a' b'
    n2 <- nandGate a' n1
    n3 <- nandGate b' n1
    nandGate n2 n3
