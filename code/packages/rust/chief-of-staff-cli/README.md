# chief-of-staff-cli

Concrete `chief-of-staff` operator executable for D18. Help and version output
remain local. Host lifecycle commands load the strict default configuration,
acquire the owner-only local credential outside argv, connect to the configured
loopback WebSocket API, authenticate, dispatch through
`chief-of-staff-cli-core`, and close the session.

The same authenticated path supports typed `wire` and `unwire` pipeline
commands. The CLI validates and constructs the exact package, agent, channel,
and optional model binding before dispatch; the daemon derives requester and
request identity from the authenticated session and protocol envelope.

`chief-of-staff install-daemon` derives the sibling
`chief-of-staff-daemon` executable and default configuration path, then uses the
strict daemon config loader before the secure installer publishes and registers
a current-user launchd, systemd, or Task Scheduler definition. Native tools are
invoked directly with typed argument vectors; no shell is involved.

## Validation

```sh
sh chief-of-staff-cli/BUILD
```
