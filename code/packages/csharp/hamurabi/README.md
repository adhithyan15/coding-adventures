# CodingAdventures.Hamurabi.CSharp

Pure, deterministic, platform-independent simulation core for the seminal 1968 mainframe resource allocation game **Hamurabi** (`GAME01`).

## Historical Context

- **1964 — The Sumerian Game:** Designed by Mabel Addis and programmed by William McKay for an IBM 7090 mainframe.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the DEC PDP-8 in FOCAL-69.
- **1973 — 101 BASIC Computer Games:** Published by David H. Ahl.

## Architectural Guarantees

- **Zero ambient authority (`VER00`, `CAP001`):** `"capabilities": []` declared in `required_capabilities.json`.
- **Pure state transitions (`VER04m`):** `Hamurabi.StepYear(state, order)` produces a new immutable `HamurabiState`.
- **Deterministic pseudo-randomness:** Explicit `PCG32` state stored directly in `HamurabiState`.
- **Bit-exact integer math:** Uses exclusively bounded integer types (`byte`, `uint`, `ulong`, `int`, `long`) with zero floating-point operations.
- **Zero dynamic memory allocation on hot path (`VER07`):** Value records and stack-allocated state transitions.

## Usage

```csharp
using CodingAdventures.Hamurabi;

// Initialize a new game with a deterministic seed
var state = Hamurabi.NewGame(seed: 42);

// Place an order: buy/sell acres, feed citizens, plant acres
var order = new TurnOrder(
    AcresDelta: 0,
    BushelsToFeed: 2000,
    AcresToPlant: 1000
);

// Validate and step
var result = Hamurabi.StepYear(state, order);
if (result.IsOk)
{
    var nextState = result.Value;
    Console.WriteLine(Hamurabi.FormatReport(nextState));
}
```

## Testing & Verification

Run the test suite with coverage:

```bash
dotnet test tests/CodingAdventures.Hamurabi.Tests/CodingAdventures.Hamurabi.Tests.csproj --disable-build-servers /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line
```
