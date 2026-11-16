# coding-adventures-hamurabi (Haskell)

Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game (**GAME01** / **HA01**).

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at BOCES in Westchester County, New York.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the DEC PDP-8 in FOCAL-69.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published by DEC.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):
- **Zero ambient authority:** `"capabilities": []` declared in `required_capabilities.json`.
- **Pure state transitions:** `stepYear :: HamurabiState -> TurnOrder -> Either HamurabiError HamurabiState`.
- **Explicit pseudorandomness:** All simulation rolls enter deterministically via explicit 64-bit PCG32 state embedded in `HamurabiState` (`rngState`, `rngInc = 109`).
- **Bit-exact integer arithmetic:** Bounded unsigned and signed integers (`Word8`, `Word32`, `Word64`, `Int32`) with zero floating-point operations.
- **Thunk leak prevention (CWE-400):** Strict fields (`!Word8`, `!Word32`, `!Word64`, `!Int32`, `!Bool`, `!GameStatus`) across all records.

## Usage

```haskell
import Hamurabi

-- Start a new game with a fixed deterministic seed
let state0 = newGame 42

-- Issue turn orders: acres to trade (+ buy, - sell), bushels to feed, acres to plant
let order = TurnOrder { acresDelta = 0, bushelsToFeed = 2000, acresToPlant = 1000 }

-- Advance one year
case stepYear state0 order of
  Left err -> putStrLn ("Refusal: " ++ show err)
  Right state1 -> do
    putStr (formatReport state1)
```
