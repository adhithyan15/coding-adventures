# zwave-command-classes

Z-Wave command class value and D23 mapping primitives.

This crate owns the first semantic layer above `zwave-core` frame and id
primitives. It deliberately does not perform Serial API I/O, controller
correlation, inclusion, or S2 security.

Included surfaces:

- command-class command parse/encode helpers
- body-free command summaries for logs and interview diagnostics, including
  payload-free and Get/Set/Report helper predicates
- command batch summaries for payload, request/report, extended class,
  encodability, and unique command-class rollups
- binary switch, multilevel switch, and door lock get/set builders, plus
  battery and meter get builders
- command-class interview descriptors for state-query commands and D23
  capability projection
- value-report parsing for Basic, Switch Binary, Switch Multilevel, Sensor
  Binary, Sensor Multilevel, Door Lock, Battery, Meter, and Notification
- value-report encoding for fixture, simulator, and parser round-trip coverage
- Z-Wave level/boolean/door-lock normalization helpers
- command-class projection summaries for D23 command and sensor surface checks
- command-class projection readiness summaries for command-class inventory,
  projected capability, command, sensor, and observe-only surface gates
- command-class projection signoff summaries for final D23 projection gates
- command-class projection closure summaries for final D23 projection closure
  gates
- D23 capability projection for common command classes
- D23 `StateDelta` projection for common value reports

## Dependencies

- smart-home-core
- zwave-core

## Development

```bash
# Run tests
bash BUILD
```
