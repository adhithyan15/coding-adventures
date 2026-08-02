# smart-home-roku-ecp-integration

This package connects Roku streaming players and Roku TVs to D23 through the
local External Control Protocol (ECP):

- SSDP discovery using the `roku:ecp` search target;
- bounded local HTTP inspection of device information, installed apps, and the
  active app;
- normalized read-only media state; and
- D23 read authorization before network I/O or runtime installation.

Run `cargo run -p smart-home-roku-ecp-integration -- discover` to scan the LAN,
or `cargo run -p smart-home-roku-ecp-integration -- inspect <base-url>` to inspect
one device.

This slice does not claim remote-control commands, app launches, media transfer,
or recording. D23 needs a protocol-neutral media command contract before those
operations can be exposed honestly.
