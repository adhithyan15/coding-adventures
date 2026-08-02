# smart-home-reolink-integration

This package connects Reolink cameras and NVRs to D23 through the authenticated
local HTTP/HTTPS CGI API:

- manual fixed-endpoint discovery after authenticated inspection;
- bounded login-token, device-information, channel-status, motion-state, and
  logout exchanges;
- normalized camera-channel and motion-sensor entities; and
- D23 read authorization before credentials or network I/O are used.

Credentials are zeroized after use and are represented in D23 only by a
`VaultRef`. Session tokens are redacted and never enter normalized state.

This slice does not claim RTSP media transfer, recording, PTZ control, webhook
events, or ONVIF PullPoint events. Those remain separate transport/runtime work.

Set `REOLINK_USERNAME`, `REOLINK_PASSWORD`, and `REOLINK_CREDENTIAL_REF`, then
run `cargo run -p smart-home-reolink-integration -- inspect <base-url>` to emit
sanitized device and channel state.
