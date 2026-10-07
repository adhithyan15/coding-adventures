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

## Provisioning vault secrets

`vault put`, `vault delete` and `vault list` write the Chief vault directly
(D18U, "Provisioning commands"). They never contact the daemon. They open
`[vault] storage_path` with the owner-only `[vault] kek_path`, and refuse when
that key is not configured.

```sh
# Pipe the value in. It is never an argument, and a terminal is refused so a
# typed secret is never echoed.
pass show weather/api-key | chief-of-staff vault put weather-key \
    --mode leased --tier 1 --allow-agent weather-host

chief-of-staff vault list                 # names only, nothing decrypted
chief-of-staff vault delete weather-key
```

- **Every policy flag is required.** `--mode` and `--tier` have no default.
  You must give either `--allow-agent HOST` (repeatable) or `--any-agent`.
- **Agent names are host names.** `--allow-agent` takes the name the host was
  registered under, because that is the identity the daemon attests.
- **One trailing newline is stripped.** Use `--raw` to keep the bytes exactly.
- **Output never contains the value.** It shows the action, the secret name,
  and that the change takes effect after the next daemon restart. Restarting
  also revokes every lease minted against the old value.

Errors print their full cause chain, for example
`chief CLI: command failed: invalid tier: must be 0, 1, 2, or 3`. Every error
in that chain is payload-blind, so the chain carries no secret material.

## Validation

```sh
sh chief-of-staff-cli/BUILD
```
