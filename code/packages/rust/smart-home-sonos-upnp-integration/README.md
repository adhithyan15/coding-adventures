# smart-home-sonos-upnp-integration

This package connects local Sonos players to D23 through their UPnP interface:

- SSDP discovery of Sonos `ZonePlayer` devices;
- bounded retrieval and parsing of UPnP device and service descriptions;
- SOAP inspection of AVTransport state, current track metadata, master volume,
  and mute state; and
- authorized installation of normalized read-only D23 player state.

The integration remains read-only until D23 has protocol-neutral media-player
entity and command contracts. It does not claim GENA subscriptions, topology or
group control, queue mutation, playback control, cloud control, or local-token
authentication.
