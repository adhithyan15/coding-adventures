# forme-plugin-runner-rs

The bounded Rust plugin-side runtime for Forme's FM02 subprocess protocol.
Authors implement the typed asynchronous `Stage` trait and pass the stage to
`run_plugin`. `StageInput<T>` and `StageOutput<T>` make single and streaming
shapes explicit while preserving the authored input/output types.

```rust
use async_trait::async_trait;
use forme_plugin_runner_rs::{Stage, StageContext, StageError, StageInput,
    StageMetadata, StageOutput, WireValue};

struct Uppercase;

#[async_trait]
impl Stage for Uppercase {
    type Input = WireValue;
    type Output = WireValue;

    fn metadata(&self) -> StageMetadata {
        StageMetadata::new("@example/uppercase", "1.0.0", 1,
            "ContentNode", "ContentNode", [] as [&str; 0])
    }

    async fn run(&self, input: StageInput<Self::Input>, _config: WireValue,
                 _context: &StageContext)
        -> Result<StageOutput<Self::Output>, StageError> {
        let StageInput::Single(value) = input else {
            return Err(StageError::new("INVALID_INPUT_SHAPE", "expected one node"));
        };
        Ok(StageOutput::Single(value))
    }
}
```

Every privileged storage, network, environment, wall-clock, absolute-filesystem,
and shell operation is an asynchronous request to the host. The SDK owns no
ambient authority. It validates canonical bounded Content-Length JSON-RPC
frames, preserves bytes and reserved objects, permits one active invocation,
bounds in-flight work, pending requests, writes, stream values, and retained
stream bytes, and redacts internal failures. Malformed, oversized,
out-of-phase, or unknown peers fail closed.

`sh BUILD` runs formatting, strict Clippy, unit tests, Linux coverage when
available, and the exact language-neutral subprocess corpus passed by the
TypeScript and Python runners.
