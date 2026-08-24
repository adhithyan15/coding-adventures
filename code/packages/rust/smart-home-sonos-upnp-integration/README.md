# smart-home-sonos-upnp-integration

This package connects local Sonos players to D23 through their UPnP interface:

- SSDP discovery of Sonos `ZonePlayer` devices;
- bounded retrieval and parsing of UPnP device and service descriptions;
- SOAP inspection of AVTransport state, current track metadata, master volume,
  and mute state;
- authorized installation of normalized D23 player state; and
- D23-authorized, idempotent play, pause, stop, master-volume, and master-mute
  commands through a fixed SOAP allowlist with native readback verification.

Configuration accepts only one credential-free private, link-local, or loopback
IP literal. Device-advertised control URLs must share that exact authority. The
integration does not claim GENA subscriptions, topology or group control, queue
mutation, browse, seek, next/previous, media transfer, cloud control, DNS/public
endpoints, or local-token authentication.
