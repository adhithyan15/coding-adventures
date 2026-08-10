# smart-home-reolink-integration

This package connects Reolink cameras and NVRs to D23 through the authenticated
local HTTP/HTTPS CGI API:

- manual fixed-endpoint discovery after authenticated inspection;
- bounded login-token, device-information, channel-status, pairing-only channel
  ability, motion-state, and logout exchanges;
- normalized camera-channel and motion-sensor entities;
- documented JPEG snapshot capability for awake, online `RLC-*` physical
  channels, including NVR channels whose `typeInfo` and `abilityChn.snap` are
  authenticated during pairing, composed by `smart-home-reolink-snapshot-host`;
- capability-probed `GetRecV20` state plus authorized `SetRecV20` recording
  enable/disable with exact readback verification; and
- capability-probed `GetPtzPreset` support plus authorized preset recall and
  direction/speed movement that always sends `Stop` within five seconds; and
- D23 read authorization before credentials or network I/O are used.

Credentials are zeroized after use and are represented in D23 only by a
`VaultRef`. Session tokens are redacted and never enter normalized state.

This package excludes empty NVR slots and does not infer snapshot support for
battery cameras or logical channels. It does not infer a current PTZ position
where firmware offers no portable readback. Recording search/download, RTSP media transfer,
autonomous guard/patrol behavior, webhook events, and ONVIF PullPoint events
remain separate transport/runtime work.

Set `REOLINK_USERNAME`, `REOLINK_PASSWORD`, and `REOLINK_CREDENTIAL_REF`, then
run `cargo run -p smart-home-reolink-integration -- inspect <base-url>` to emit
sanitized device and channel state.
