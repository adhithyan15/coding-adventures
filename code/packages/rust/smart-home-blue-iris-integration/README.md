# smart-home-blue-iris-integration

Production-facing Blue Iris NVR inspection and bounded camera control for D23.

The integration accepts a manually configured local HTTPS origin and a Vault
credential reference. After D23 authorizes `smart_home.read`, the production
transport performs Blue Iris's documented `/json` challenge-response login and
requests `camlist`. Credentials, the MD5 challenge response, and each fresh
session stay inside the transport, and every successful operation explicitly
logs out. The normalized result includes the server identity and each camera's
enabled, online, signal, motion, trigger, alert, and recording state.

When login grants `clipcreate`, camera entities expose the typed
`camera.recording` command. The transport changes only Blue Iris manual
recording and confirms the result through exact `camlist.isManRec` readback.
When login grants `ptz` and `camlist` marks a camera as PTZ-capable, entities
also expose preset recall for presets 1 through 20 and directional movement
with a 1-100 speed and a duration bounded to five seconds. Movement always ends
with Blue Iris's native Stop command; an ambiguous start response also triggers
a best-effort stop. All controls retain the existing D23 human-approval tier.

Plain HTTP is accepted only for loopback transport tests. The runtime does not
expose snapshots, streams, clips, media transfer, general camera configuration,
or administrative commands in this slice.

Protocol reference:

- [Blue Iris manual, JSON interface](https://blueirissoftware.com/blueiris.pdf)
