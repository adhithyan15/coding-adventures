# smart-home-synology-surveillance-integration

Production-facing Synology Surveillance Station camera health inspection for
D23.

The integration accepts a manually configured local HTTPS origin and a Vault
credential reference. After D23 authorizes `smart_home.read`, the production
transport queries `SYNO.API.Info`, validates the advertised paths and versions,
opens an isolated SID-format `SurveillanceStation` session, reads package
information plus privilege-filtered camera status, and explicitly logs out.
Credentials, SID values, SynoToken values, and login bodies remain inside
zeroizing transport memory and never enter request plans or normalized state.

Camera entities expose confirmed native status, channel, vendor, and model.
Normal and ready cameras map online; transitional states map degraded; native
connection, authorization, stream, storage, and disabled states map offline.
Plain HTTP is accepted only for loopback protocol tests.

This slice intentionally does not expose snapshots, recordings, playback,
exports, events, PTZ, external recording, configuration, OTP, or remembered
device-token flows. Those operations need the existing media/event hosts,
operation-specific D23 contracts, or a concrete interactive authentication
lifecycle.

Protocol references:

- [Surveillance Station Web API Guide](https://global.download.synology.com/download/Document/Software/DeveloperGuide/Package/SurveillanceStation/All/enu/Surveillance_Station_Web_API.pdf)
- [DSM Login Web API Guide](https://kb.synology.com/en-us/DG/DSM_Login_Web_API_Guide/2)
