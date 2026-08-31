# smart-home-upnp-media-renderer-integration

This package connects standards-compliant local UPnP AV MediaRenderer devices
to D23:

- exact MediaRenderer:1 SSDP discovery;
- bounded retrieval and parsing of UPnP device and service descriptions;
- shared, typed `upnp-av-protocol` SOAP inspection of AVTransport state,
  current track metadata, master volume, and mute state;
- authorized installation of normalized D23 player state; and
- D23-authorized, idempotent play, pause, stop, master-volume, and master-mute
  commands through a fixed SOAP allowlist with native readback verification.

Configuration accepts only one credential-free private, link-local, or loopback
IP literal. Device-advertised control URLs must share that exact authority. The
integration does not claim GENA subscriptions, ContentDirectory browse, queue
mutation, seek, next/previous, media transfer or URI selection, topology or group
control, cloud control, DNS/public endpoints, or local-token authentication.
