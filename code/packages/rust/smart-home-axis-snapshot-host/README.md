# smart-home-axis-snapshot-host

Production composition for one bounded Axis VAPIX camera snapshot.

The host requires exact D23 Human Approval before Vault or network access,
validates the installed Axis camera-1 entity and its probed JPEG capability,
resolves one bounded versioned credential envelope, and registers the
documented `/axis-cgi/jpg/image.cgi?camera=1` endpoint against a reviewed
canonical host and pinned socket. Basic or Digest credentials and the endpoint
remain process-local and are removed after every delivery outcome.

Production delivery is HTTPS-only. Plain HTTP is available solely to explicit
loopback fixtures. Streams, recordings, playback, source enumeration, and
broader media transfer remain outside this host.
