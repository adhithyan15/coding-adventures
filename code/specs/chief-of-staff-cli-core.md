# D18 Chief Operator CLI Core

## Status

This document specifies the first operator command surface for the D18 Chief
daemon. The Rust implementation package is `chief-of-staff-cli-core`.

## Purpose

The CLI core converts declaratively parsed argv into typed calls on the
authenticated Chief daemon client. This slice exposes only the host lifecycle
operations already implemented by `chief-of-staff-daemon-api`:

- `agents` lists durable host records;
- `doctor HOST` returns durable intent and fresh supervisor evidence;
- `register HOST PACKAGE_PATH PACKAGE_HASH` registers immutable package
  identity and initial desired state;
- `start HOST` and `stop HOST` update durable desired state;
- `reconcile` runs one bounded convergence tick; and
- `deregister HOST` removes stopped, inactive intent.

Pipeline control, interactive agent conversations, approvals, vault unlock,
daemon installation, and background scheduling remain later slices because the
daemon does not expose those capabilities yet.

## Trust Boundary

The core never accepts credentials, tokens, passphrases, or socket endpoints in
argv. It executes against an already connected and authenticated client supplied
by the outer process adapter. Secure terminal input, credential lifetime,
loopback connection policy, and process exit codes therefore remain outside the
parser and dispatcher.

The public client trait is implemented for `chief_of_staff_daemon_api::DaemonClient`.
Tests may inject an in-memory client without opening sockets.

## Declarative Syntax

The command tree is embedded as a `cli-builder` 1.0 JSON specification. The
parser owns help, version output, subcommand routing, required argument checks,
and enum validation.

`register` accepts these flags:

- `--restart always|on_failure|never`, defaulting to `never`;
- `--state running|stopped`, defaulting to `stopped`.

The conservative defaults prevent an omitted option from creating an automatic
restart loop or immediately launching a newly registered package.

Host names and package paths are validated by `chief-of-staff-service-registry`.
Package hashes must be exactly 64 lowercase hexadecimal characters and are
decoded into 32 bytes before any daemon call. The CLI introduces no second
host-identity grammar.

## Output and Errors

Successful daemon results remain `JsonValue` values and can be rendered as
deterministic two-space-indented JSON with a trailing newline. This preserves
the daemon's precision-safe string representation for revisions and timestamps.

Errors distinguish declarative parse failures, invalid typed input, daemon
client failures, and local result-serialization failures. Daemon failures keep
their typed source; their payload-blind `Display` behavior is not weakened by
the CLI core.

## Capabilities

The package performs no filesystem, network, process, environment, clock,
randomness, terminal, or secret-store access. Network and credential authority
belong to the injected authenticated client and the future executable adapter.

## Required Tests

The package must cover:

- declarative help and version behavior;
- parsing every supported command;
- conservative registration defaults and explicit enum overrides;
- host-name, package-path, and lowercase SHA-256 validation;
- dispatch of every command through an injected client;
- preservation of typed daemon failures;
- deterministic JSON result rendering; and
- absence of credential-bearing argv flags or help text.
