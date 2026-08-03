# smart-home-blue-iris-integration

Production-facing, read-only Blue Iris NVR inspection for D23.

The integration accepts a manually configured local HTTPS origin and a Vault
credential reference. After D23 authorizes `smart_home.read`, the production
transport performs Blue Iris's documented `/json` challenge-response login and
requests `camlist`. Credentials, the MD5 challenge response, and the resulting
session stay inside the transport. The normalized result includes the server
identity and each camera's enabled, online, signal, motion, trigger, alert, and
recording state.

Plain HTTP is accepted only for loopback transport tests. The runtime does not
expose snapshots, streams, clips, PTZ, recording controls, configuration
mutation, or administrative commands in this slice.

Protocol reference:

- [Blue Iris manual, JSON interface](https://blueirissoftware.com/blueiris.pdf)
