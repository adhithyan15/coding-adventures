# coding-adventures-forme-plugin-runner-py

The bounded Python plugin-side runtime for the Forme FM02 protocol. Authors
declare an async stage with `define_stage`, then pass it to `run_plugin`.
Storage, network, environment, wall-clock, filesystem, and shell operations are
asynchronous RPCs mediated by the host; cancellation observation remains local.

```python
from forme_plugin_runner import define_stage, run_plugin

@define_stage(
    name="@example/uppercase",
    version="1.0.0",
    api_version=1,
    consumes="ContentNode",
    produces="ContentNode",
    capabilities=[],
)
async def uppercase(value, _config, _ctx):
    return {**value, "text": value["text"].upper()}

if __name__ == "__main__":
    run_plugin(uppercase)
```

The runtime validates its bounded bootstrap arguments and handshake identity,
uses canonical Content-Length/JSON-RPC framing, escapes binary and reserved
wire values, permits one active run, and translates single, stream, hybrid,
cancellation, capability, and typed-stage-error behavior. Malformed, oversized,
out-of-phase, or unknown peers fail closed. Optional async `init` and `dispose`
callbacks receive the same mediated context, and one-shot process-signal
shutdown cancels active work, wakes streams, runs bounded cleanup, and stops
the peer. Ingress, in-flight requests, outgoing notifications, pending calls,
streams, and frames all have explicit memory or concurrency limits.

`sh BUILD` runs formatting, lint, strict typing, unit coverage, and the exact
language-neutral corpus also passed by the TypeScript reference runner.
