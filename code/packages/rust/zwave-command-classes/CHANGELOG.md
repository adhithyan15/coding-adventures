# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- Command-class command parse/encode primitives.
- Get/set builders for binary switch, multilevel switch, and door lock command
  classes.
- Value-report parsing for common actuator and sensor command classes.
- Notification report parsing and D23 state-delta mapping for motion, contact,
  lock, and alarm-style sensor events.
- Battery Get/Report primitives and D23 `sensor.battery` projection.
- Meter Get/Report primitives and D23 energy/power state projection.
- Command-class interview descriptors that expose state-query commands and D23
  capability projection for supported classes.
- Command-class projection summaries for D23 command and sensor surface checks.
- D23 capability and state-delta mapping for common reports.
