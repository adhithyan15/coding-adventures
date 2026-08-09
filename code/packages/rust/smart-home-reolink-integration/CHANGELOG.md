# Changelog

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
