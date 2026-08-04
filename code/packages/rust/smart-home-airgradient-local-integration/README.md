# smart-home-airgradient-local-integration

First-party AirGradient local monitor integration for D23.

It resolves the documented `airgradient_<serial>.local` mDNS hostname or
accepts a manual host. A bounded local HTTP client reads
`/measures/current`, verifies the returned serial, model, and firmware, and
installs normalized PM, CO2, temperature, humidity, VOC, NOx, particle-count,
and Wi-Fi sensor entities. It also reads `/config` and installs an indicator and
display control surface plus an explicit CO2 calibration command.

The local runtime supports LED-bar mode (`co2`, `pm`, `iaqs`, or `off`), LED-bar
brightness, display brightness, and the documented 400 ppm CO2 calibration
trigger. Every command is validated and authorized before transport I/O.
Brightness and mode updates are confirmed with a configuration readback.
Typed non-credential settings also cover Celsius/Fahrenheit display, PM mass or
US AQI display, 0-200 day automatic CO2 baseline calibration, 0-720 hour VOC
and NOx learning offsets, compensated display values, LED self-test, and
sensor-specific correction profiles. Persistent settings are read back and
verified against the monitor's native response.
Monitors with `configurationControl=cloud` reject local commands explicitly;
`configurationControl=both` succeeds with a warning that a later cloud update
may overwrite the local value.

MQTT broker and custom HTTP destination settings remain excluded because they
can carry credentials or redirect telemetry and therefore require Vault leases
and destination policy.

```bash
cargo run -p smart-home-airgradient-local-integration -- discover ecda3b1eaaaf
cargo run -p smart-home-airgradient-local-integration -- inspect 192.0.2.50
```
