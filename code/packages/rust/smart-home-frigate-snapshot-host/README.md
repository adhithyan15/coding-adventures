# smart-home-frigate-snapshot-host

Authorized production composition for bounded Frigate camera snapshots.

The host performs exact D23 Human Approval preflight before Vault or network
access. It resolves one bounded credential envelope, verifies the exact
installed Frigate bridge and native camera name, registers credentials and the
reviewed-address-pinned endpoint only in process-local media state, and delivers
one JPEG through an isolated login-cookie-logout transaction. Credentials and
the JWT cookie are zeroized and removed after every delivery outcome.

Session reuse, events, streams, recordings, export, playback, commands, and
configuration mutations remain outside this package.
