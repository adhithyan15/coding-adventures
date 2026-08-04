# chief-of-staff-agent-discovery

Verified discovery for sealed D18 `.agent` packages. It supports explicit
inspection and stable, non-recursive directory scans. Every candidate is
signature-verified before the exact authenticated manifest bytes are parsed.
The result is an inert `HostRegistration`; callers must explicitly submit it to
the authenticated control plane. Complete verified snapshots can be compared
with `plan_catalog_reload`, which returns added, removed, and replaced agents in
stable identity order. Discovery and planning each enforce a 4,096-package
bound and never register, stop, replace, or start agents.
