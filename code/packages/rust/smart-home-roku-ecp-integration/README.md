# smart-home-roku-ecp-integration

This package connects Roku streaming players and Roku TVs to D23 through the
local External Control Protocol (ECP):

- SSDP discovery using the `roku:ecp` search target;
- bounded local HTTP inspection of device information, installed apps, the
  active app, and media-player state;
- normalized media state; and
- D23 read and low-risk media-command authorization before network I/O.

Run `cargo run -p smart-home-roku-ecp-integration -- discover` to scan the LAN,
or `cargo run -p smart-home-roku-ecp-integration -- inspect <base-url>` to inspect
one device.

The command surface accepts only D23 `play` and `pause`. It reads the current
media-player state, sends the fixed ECP `Play` key only from the exact opposite
state, and requires a second media-player query to prove the requested result.
Configured endpoints must use private, link-local, or loopback IP literals.

This slice accepts no credentials and exposes no app launch, arbitrary keypress,
browse, input, power, volume, media transfer, recording, or long-lived
connection.
