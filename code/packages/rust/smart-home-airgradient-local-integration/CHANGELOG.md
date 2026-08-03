# Changelog

## 0.2.0

- Add authorized LED-bar mode and brightness plus display-brightness controls.
- Add a human-approval CO2 calibration command over the local configuration API.
- Read configuration-control mode before writes, reject cloud-only conflicts,
  warn when cloud configuration can overwrite a local change, and verify
  persistent settings through readback.
- Add real loopback coverage for every command and denial-before-I/O coverage.

## 0.1.0

- Add verified `airgradient_<serial>.local` and manual HTTP discovery.
- Add bounded local monitor identity and environmental telemetry inspection.
- Normalize environmental and diagnostic measurements into D23 sensors.
- Add authorization-before-transport and loopback HTTP coverage.
