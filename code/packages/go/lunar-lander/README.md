# lunar-lander (Go)

Pure, deterministic simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation (GAME02).

## Historical Lineage

- **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim Storer on a DEC PDP-8 in FOCAL-69, modelling the physics of the Apollo Lunar Module (LM) descent stage.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published under the title *Rocket* (and later *Lunar*), becoming an enduring computing classic.

## Architectural Guarantees

- **Zero ambient system authority:** `"capabilities": []` declared in `required_capabilities.json`.
- **Pure integer fixed-point physics:** 64-bit and 128-bit integer arithmetic with zero floating-point operations in the simulation core.
- **Zero heap allocation (`VER07`):** all core simulation functions (`NewFlight`, `ValidateBurn`, `StepBurn`, `IsqrtU128`, `DivEuclid`) execute with zero dynamic memory allocations on the simulation path.
- **Declarative 2D telemetry:** generates intermediate representation `PaintScene` rendered through a deterministic character-grid rasterizer.
- **Full CWE weakness closure:** `CWE-190` (Integer Overflow) and `CWE-369` (Divide by Zero) immunization.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/lunar-lander"

// Initialize flight at Apollo LM standard conditions
state := lunarlander.NewFlight()

// Command descent engine burn
cmd := lunarlander.BurnCommand{
    BurnRateLbsS:    100,
    DurationSeconds: 5,
}

// Validate candidate order
if err := lunarlander.ValidateBurn(state, cmd); err != nil {
    log.Fatalf("invalid command: %v", err)
}

// Advance simulation turn
next, err := lunarlander.StepBurn(state, cmd)
if err != nil {
    log.Fatalf("step error: %v", err)
}

// Render ASCII telemetry HUD
hud := lunarlander.RenderTelemetryASCII(next, cmd.BurnRateLbsS)
fmt.Println(hud)
```

## Development

```bash
go test -v -race -bench=. -benchmem ./...
```
