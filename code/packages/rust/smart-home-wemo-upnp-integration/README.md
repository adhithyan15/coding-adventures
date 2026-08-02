# smart-home-wemo-upnp-integration

This package connects local Wemo devices to D23 through their UPnP interface:

- SSDP discovery of the Wemo `basicevent` service;
- bounded retrieval and parsing of `setup.xml` service descriptions;
- SOAP `GetBinaryState` inspection for switches, outlets, and light switches;
- normalized D23 switch/light state; and
- authorized SOAP `SetBinaryState` control for devices identified as Wemo light
  switches or dimmers.

Generic Wemo outlets remain read-only until D23 has a protocol-neutral switch
command capability. The integration does not claim GENA event subscriptions,
energy telemetry, rules, pairing, or cloud control.
