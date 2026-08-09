# Changelog

## 0.5.0

- Add human-approved, readback-verified credential-free MQTT broker and custom
  HTTPS-domain routing over the documented local `/config` contract.
- Require exact destination-bound consent before enabling either telemetry
  route while allowing privacy-protective shutdown without consent.
- Redact configured destinations and any pre-existing MQTT userinfo from debug
  and normalized state; reject credential-bearing command values because the
  current upstream firmware logs parsed MQTT credentials.

## 0.4.0

- Add human-approved, readback-verified country and AirGradient cloud-upload
  controls over the documented local `/config` contract.
- Require exact host-owned data-use grants before country configuration or
  vendor-cloud upload enablement can perform transport I/O; permit upload
  shutdown without a consent grant.
- Validate assigned ISO 3166 alpha-2 codes and keep the selected country out of
  normalized state and debug output.

## 0.3.0

- Add typed local temperature/PM display settings, ABC days, gas learning
  offsets, compensated display, LED self-test, and validated correction
  profiles with native readback verification.

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
