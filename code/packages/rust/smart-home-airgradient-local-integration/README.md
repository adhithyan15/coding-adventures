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
Monitors with `configurationControl=cloud` reject local commands explicitly;
`configurationControl=both` succeeds with a warning that a later cloud update
may overwrite the local value.

```bash
cargo run -p smart-home-airgradient-local-integration -- discover ecda3b1eaaaf
cargo run -p smart-home-airgradient-local-integration -- inspect 192.0.2.50
```
