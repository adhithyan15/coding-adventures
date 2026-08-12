# chief-of-staff-host

Concrete child executable for supervised D18 Chief agent packages. The process
bootstraps the encrypted control session over standard input/output, accepts only
the authenticated public package trust and launch bindings, independently verifies
the sealed package in its working directory, and sends readiness only after the
signed Level 1 policy matches those bindings exactly.

The first production Level 1 loop deliberately requires exactly one read channel
and one write channel. It requests one verified message, executes the existing
provider-neutral `SKILL.md` runtime through the authenticated completion data plane,
publishes the response, and acknowledges only after publication. Idle reads sleep
before retrying, and the host emits authenticated heartbeats without busy-spinning.
A redacted unavailable response to the read-only receive operation follows the same
bounded backoff path, allowing the daemon's fail-closed placeholder service to stay
healthy until concrete authority is composed. Failures after an input remain terminal
so the child never guesses whether a completion or publication can be retried safely.
An authenticated `Terminate` received while waiting for any operation is a clean
exit. Data-plane failures are redacted and terminal for this child, leaving durable
input acknowledgement unchanged when processing did not finish.

The child-side `LlmClient` also carries provider-neutral tool-aware turns over a
distinct authenticated operation. Complete offered definitions, selection policy,
and prior call/result pairs cross the session, and structured final-text/tool-call
responses retain provider and polyfill audit fields. The model-emitted call is not
executed by this adapter; D18D policy and execution remain parent-side work.

The production integration gate launches this executable with durable pipeline
bindings, provisions its exact channel keys from owner-only files, sends its model
request through the daemon's configured Ollama adapter, decrypts the published
weather report as the authorized sink, and verifies that input acknowledgement is
persisted only after publication.

The executable rejects `--package-runtime deno` until a separately reviewed Deno
adapter is composed. It uses no ambient environment configuration and inherits the
supervisor-selected package directory as its only package location.

## Validation

```sh
sh chief-of-staff-host/BUILD
```
