# zwave-command-classes

Z-Wave command class value and D23 mapping primitives.

This crate owns the first semantic layer above `zwave-core` frame and id
primitives. It deliberately does not perform Serial API I/O, controller
correlation, inclusion, or S2 security.

Included surfaces:

- command-class command parse/encode helpers
- binary switch, multilevel switch, and door lock get/set builders
- value-report parsing for Basic, Switch Binary, Switch Multilevel, Sensor
  Binary, Sensor Multilevel, and Door Lock
- Z-Wave level/boolean/door-lock normalization helpers
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
