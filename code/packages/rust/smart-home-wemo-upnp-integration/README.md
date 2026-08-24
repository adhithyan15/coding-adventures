# smart-home-wemo-upnp-integration

This package connects local Wemo devices to D23 through their UPnP interface:

- SSDP discovery of the Wemo `basicevent` service;
- bounded retrieval and parsing of `setup.xml` service descriptions;
- SOAP `GetBinaryState` inspection for switches, outlets, and light switches;
- normalized D23 switch/light state; and
- authorized SOAP `SetBinaryState` control for recognized Wemo switches,
  outlets, plugs, and dimmers, with a current-state read before mutation and a
  fresh `GetBinaryState` readback afterward.

Configured setup URLs must use a private, link-local, or loopback IP literal.
Device-advertised control URLs must share that exact authority, and no DNS or
public endpoint is accepted. Unknown `basicevent` models remain read-only.

The integration does not claim GENA event subscriptions, energy telemetry,
rules, pairing, cloud control, arbitrary SOAP, or a long-lived connection.
Belkin documents Wemo's local UPnP discovery and local web-service ports, and
has ended its Wemo cloud service:

- https://www.belkin.com/hk/en/support-article/?articleNum=54237
- https://www.belkin.com/support-article/?articleNum=335419
