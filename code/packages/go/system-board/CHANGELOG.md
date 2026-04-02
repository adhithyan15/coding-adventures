# Changelog

## 0.2.0 — 2026-04-02

### Changed

- **Operations pattern**: Wrapped all public functions and methods with `StartNew` for automatic timing, structured logging, and panic recovery. Methods covered: `NewSystemBoard`, `PowerOn`, `Step`, `Run`, `InjectKeystroke`, `DisplaySnapshot`, `GetBootTrace`, `IsIdle`, `GetCycleCount`, `GetCurrentPhase`, `DefaultSystemConfig`, `BootPhase.String`, `BootTrace.AddEvent`, `BootTrace.Phases`, `BootTrace.EventsInPhase`, `BootTrace.TotalCycles`, `BootTrace.PhaseStartCycle`. The public API is fully backward-compatible.

## 0.1.0 — 2026-03-21

### Added
- `SystemBoard` type composing all S-series components into a complete computer
- `PowerOn()`, `Step()`, `Run(maxCycles)` for boot sequence execution
- `DisplaySnapshot()` for reading the text display
- `InjectKeystroke()` for keyboard input simulation
- `IsIdle()`, `GetCycleCount()`, `GetCurrentPhase()` for state queries
- `BootTrace` with phase tracking, event logging, cycle counts
- `BootPhase` enum: PowerOn, BIOS, Bootloader, KernelInit, UserProgram, Idle
- `SystemConfig` and `DefaultSystemConfig()` for one-line setup
- Automatic ecall trap interception and Go-side syscall dispatch
- Boot phase detection based on PC location
- **TestBootToHelloWorld**: the critical integration test proving the full stack works
- 22 tests with 88%+ coverage
