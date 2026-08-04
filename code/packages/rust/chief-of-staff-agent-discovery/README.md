# chief-of-staff-agent-discovery

Verified discovery for sealed D18 `.agent` packages. It supports explicit
inspection and stable, non-recursive directory scans. Every candidate is
signature-verified before the exact authenticated manifest bytes are parsed.
The result is an inert `HostRegistration`; callers must explicitly submit it to
the authenticated control plane. Discovery never registers or starts agents.
