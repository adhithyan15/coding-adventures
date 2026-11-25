# logic-gates (Haskell)

The basic logic gates, plus combinational and sequential circuits built on them.
Bits are `Int`s; every function validates its inputs and returns
`Either LogicGateError Bit`.

## Basic gates

| Function   | Meaning                         |
|------------|---------------------------------|
| `notGate`  | flip the bit                    |
| `andGate`  | 1 only if both inputs are 1     |
| `orGate`   | 1 if either input is 1          |
| `xorGate`  | 1 if the inputs differ          |
| `nandGate` | NOT (a AND b)                   |
| `norGate`  | NOT (a OR b)                    |
| `xnorGate` | 1 if the inputs are equal       |
| `nandNot`  | NOT built from NAND only        |
| `nandAnd`  | AND built from NAND only        |
| `nandOr`   | OR built from NAND only         |
| `nandXor`  | XOR built from 4 NANDs only     |

```haskell
import LogicGates

andGate 1 1    -- Right 1
xnorGate 1 0   -- Right 0
nandXor 1 1    -- Right 0
andGate 2 1    -- Left (InvalidBit {param = "a", value = 2})
```

## Errors

```haskell
data LogicGateError
  = InvalidBit { param :: String, value :: Int }  -- not 0 or 1
  | InvalidShape String                           -- wrong list length/shape
  deriving (Eq, Show)
```

Match on constructors and fields, never on rendered text. The FIRST invalid
parameter (argument order) is reported, named by the public function's own
parameter: `nandOr 1 2` is `Left (InvalidBit "b" 2)`, never an inner wire name.
Breaking change in 0.3.0: previously errors were `String`.

## Testing

`cabal test` (hspec).
