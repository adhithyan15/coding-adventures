# smart-home-reolink-snapshot-host

Production composition for one bounded Reolink RLC physical-channel snapshot.

The host requires exact D23 Human Approval before Vault or network access,
validates the installed online and awake RLC camera entity, resolves one bounded
versioned credential envelope, and registers the documented
`/cgi-bin/api.cgi?cmd=Snap` HTTPS endpoint against a reviewed canonical host and
pinned socket. Percent-encoded credentials and the complete endpoint remain in
zeroizing process-local memory and are removed after every delivery outcome.

The host does not infer support for NVR channels, battery cameras, or excluded
model families. Streams, recordings, playback, logical-channel snapshots, and
broader media transfer remain outside this package.
