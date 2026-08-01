# smart-home-govee-lan-integration

This package connects LAN-enabled Govee lights directly to D23 using Govee's
documented local UDP API:

- multicast `scan` discovery at `239.255.255.250:4001` with replies on UDP 4002;
- bounded `devStatus` polling against each device's UDP 4003 endpoint;
- normalized light state and capabilities; and
- capability-caged `turn`, `brightness`, and `colorwc` commands with a status
  query that verifies each mutation.

The device's **LAN Control** setting must be enabled in the Govee app. No cloud
API key or account credential is accepted by this package.

Protocol source: [Govee LAN API guide](https://app-h5.govee.com/user-manual/wlan-guide).

Run `cargo run -p smart-home-govee-lan-integration -- discover` to scan the
local network, or `cargo run -p smart-home-govee-lan-integration -- inspect
<host> <device-id> <sku>` to emit sanitized current state.
