# smart-home-synology-snapshot-host

Authorized production composition for bounded Synology Surveillance Station
camera snapshots.

The host performs exact D23 Human Approval preflight before Vault or network
access. It resolves one bounded credential envelope, opens one isolated
Surveillance Station SID/SynoToken session, confirms snapshot permission and
the exact privilege-filtered installed camera, registers the documented
version-9 `GetSnapshot` endpoint only in zeroizing process-local media state,
delivers one reviewed-address-pinned JPEG, removes the endpoint, and explicitly
logs out on every delivery outcome.

OTP and remembered-device authentication, session reuse, events, streams,
recordings, export, playback, PTZ, and configuration mutations remain outside
this package.
