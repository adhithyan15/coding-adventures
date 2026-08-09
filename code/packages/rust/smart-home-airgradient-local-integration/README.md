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
Country configuration accepts assigned ISO 3166 alpha-2 codes and requires an
exact host-owned coarse-location consent grant. Enabling AirGradient vendor
cloud upload requires a separate environmental-telemetry egress grant bound to
`https://api.airgradient.com`; disabling upload remains privacy-protective and
does not require a consent grant. Both commands still require D23 human
approval, and every persistent change is verified through `/config` readback.
Normalized state records only whether a country is configured, never the
country value.
Credential-free `mqtt://` and `mqtts://` broker routes require an explicit
port and an exact environmental-telemetry egress grant. Custom HTTP routing
accepts only a fully qualified DNS name and requires a grant for the matching
HTTPS origin. AirGradient firmware applies that domain to telemetry, remote
configuration, and OTA traffic together, so consent covers the coupled route.
Both controls require D23 human approval before `PUT /config`, verify exact
`GET /config` readback, and expose only configured/not-configured state.
Disabling either route is privacy-protective and does not require a consent
grant.
Monitors with `configurationControl=cloud` reject local commands explicitly;
`configurationControl=both` succeeds with a warning that a later cloud update
may overwrite the local value.

Credential-bearing MQTT command values are rejected. The current upstream
firmware logs parsed MQTT usernames and passwords, so host-side Vault leasing
cannot prevent a device-side disclosure; authenticated MQTT remains blocked
until that firmware behavior is removed and one-shot credential injection can
be proven without normalized-state or request-plan exposure.

```bash
cargo run -p smart-home-airgradient-local-integration -- discover ecda3b1eaaaf
cargo run -p smart-home-airgradient-local-integration -- inspect 192.0.2.50
```
