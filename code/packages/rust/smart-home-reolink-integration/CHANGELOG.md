# Changelog

## 0.5.0

- Retain documented `GetDevInfo.exactType` and per-channel
  `GetChannelstatus.typeInfo`, excluding empty NVR slots from installed camera
  state.
- Add pairing-only `GetAbility` inspection and bind each NVR physical channel's
  JPEG snapshot support to its documented `abilityChn.snap` version and execute
  permission.

## 0.4.1

- Add reviewed-socket pinning for HTTP loopback tests and certificate-verifying
  HTTPS hosts while retaining the reviewed hostname for SNI.
- Move credentials into zeroizing storage before validation so rejected input
  is cleared on drop.

## 0.4.0

- Advertise the documented JPEG snapshot capability only for awake, online
  `RLC-*` physical camera channels so the bounded snapshot host can require an
  exact installed-device proof.

## 0.3.0

- Added capability-probed `GetPtzPreset` support, authorized preset recall, and
  direction/speed movement bounded to five seconds with an explicit native
  `Stop` command and real loopback HTTP proof.

## 0.2.0

- Added capability-probed Reolink `GetRecV20` state and D23-authorized
  `SetRecV20` recording enable/disable with authenticated readback verification
  and a real loopback HTTP proof.

## 0.1.0

- Added authenticated Reolink CGI login-token lifecycle, device and channel
  inspection, optional motion reads, D23 normalization, authorization-before-I/O,
  and a real loopback HTTP proof.
