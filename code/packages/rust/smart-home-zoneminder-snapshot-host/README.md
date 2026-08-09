# smart-home-zoneminder-snapshot-host

This package composes installed ZoneMinder camera entities with the D23
camera-media broker, the pinned native HTTPS snapshot executor, the existing
bounded ZoneMinder API 2.0 login transport, and the durable sealed Vault. The
host checks the current authenticated principal's exact Human Approval grant
before resolving credentials or performing network I/O.

Each approved delivery resolves one bounded, versioned username/password
envelope, obtains one short-lived access token, and builds ZoneMinder's
documented `nph-zms` `mode=single` request for the exact installed monitor. The
token-bearing endpoint exists only in zeroizing process-local broker state and
is removed after success or failure. Tokens, endpoint URIs, credential bytes,
and media bearer IDs are absent from runtime state, errors, and debug output.

The configured CGI endpoint must be credential-free, share the installed
bridge's HTTPS origin, and carry a reviewed canonical host plus pinned socket.
The production constructor supplies system time, OS CSPRNG lease nonces, the
strict native HTTPS executor, and one-login-per-operation token acquisition.
Streams, recordings, exports, playback, reusable sessions, and refresh-token
handling remain outside this package.
