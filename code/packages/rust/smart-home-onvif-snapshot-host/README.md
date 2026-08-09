# smart-home-onvif-snapshot-host

This package composes installed ONVIF camera entities with the D23 camera-media
broker, the pinned native HTTPS snapshot executor, and the durable sealed Vault.
The host checks the current authenticated principal's exact Human Approval grant
before resolving the opaque credential reference or performing network I/O.

Snapshot endpoints remain process-local in `CameraMediaService`. A delivery
resolves the target camera's opaque bridge credential reference to one bounded,
versioned, zeroizing JSON credential payload, registers credentials for that
entity, redeems one short-lived snapshot lease, and removes the credentials on
every return path. The stable Vault record remains available for later approved
snapshots. Endpoint URIs, credential bytes, and media bearer IDs are absent from
errors and debug output.

The production constructor supplies system time, OS CSPRNG lease nonces, and the
strict native HTTPS executor. The authenticated principal source and Vault
boundary remain host inputs. RTSP streams are intentionally outside this package
until a supervised resource owner and teardown lifecycle exist.
