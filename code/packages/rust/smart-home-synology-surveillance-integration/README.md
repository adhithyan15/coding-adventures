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
When package information explicitly grants snapshot access, camera entities
also expose `camera.snapshot`. The transport can open one isolated snapshot
session, revalidate the exact privilege-filtered camera, produce the documented
version-9 `GetSnapshot` bearer endpoint in zeroizing memory, and explicitly
close that session after a trusted host finishes delivery.
Plain HTTP is accepted only for loopback protocol tests.

This integration does not itself deliver media. Recordings, playback, exports,
events, PTZ, external recording, configuration, OTP, remembered device-token
flows, and reusable sessions remain outside its bounded inspection/session
contract.

Protocol references:

- [Surveillance Station Web API Guide](https://global.download.synology.com/download/Document/Software/DeveloperGuide/Package/SurveillanceStation/All/enu/Surveillance_Station_Web_API.pdf)
- [DSM Login Web API Guide](https://kb.synology.com/en-us/DG/DSM_Login_Web_API_Guide/2)
