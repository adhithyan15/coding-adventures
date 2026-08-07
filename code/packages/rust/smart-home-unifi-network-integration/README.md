# smart-home-unifi-network-integration

Production-facing read-only UniFi Network inspection for D23.

The integration accepts a manually configured local UniFi OS HTTPS origin and
a Vault API-key reference. After D23 authorizes smart_home.read, the transport
uses Ubiquiti's official /proxy/network/integration/v1 API to read application
information, bounded paginated sites, and bounded paginated adopted devices.
The API key exists only in zeroizing transport memory and is materialized as
X-API-Key while encoding each request; request plans contain only the Vault
reference.

Adopted devices become confirmed network-diagnostic entities. Native online,
transitional, and unavailable states map to online, degraded, and offline
health without inventing topology or reachability.

This slice intentionally excludes remote Site Manager access, connected-client
inspection, detailed statistics, event streaming, adoption, guest
authorization, port actions, and configuration mutations. Those require
privacy and telemetry-egress policy, supervised event hosts, or
operation-specific D23 contracts and readable verification.

Protocol references:

- [Getting Started with the Official UniFi API](https://help.ui.com/hc/en-us/articles/30076656117655-Getting-Started-with-the-Official-UniFi-API)
- [UniFi Network API](https://developer.ui.com/network/v10.4.57/gettingstarted)
