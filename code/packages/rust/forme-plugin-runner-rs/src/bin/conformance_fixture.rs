use async_trait::async_trait;
use forme_plugin_runner_rs::{
    run_plugin, RunnerOptions, Stage, StageContext, StageError, StageInput, StageMetadata,
    StageOutput, WireValue, KERNEL_API_VERSION,
};
use std::collections::BTreeMap;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

struct FixtureStage {
    mode: String,
}

#[async_trait]
impl Stage for FixtureStage {
    type Input = WireValue;
    type Output = WireValue;

    fn metadata(&self) -> StageMetadata {
        let consumes = if matches!(self.mode.as_str(), "stream" | "stream-single") {
            "Stream<ContentNode>"
        } else {
            "ContentNode"
        };
        let produces = if matches!(self.mode.as_str(), "stream" | "single-stream") {
            "Stream<ContentNode>"
        } else {
            "ContentNode"
        };
        StageMetadata::new(
            "@forme/conformance",
            "1.0.0",
            KERNEL_API_VERSION,
            consumes,
            produces,
            [
                "storage:read",
                "storage:write",
                "env:ALLOWED",
                "filesystem:user",
                "network:example.com",
                "system:time:wallclock",
                "system:shell",
            ],
        )
    }

    async fn run(
        &self,
        input: StageInput<WireValue>,
        _config: WireValue,
        context: &StageContext,
    ) -> Result<StageOutput<WireValue>, StageError> {
        if self.mode == "stream" {
            let StageInput::Stream(mut input) = input else {
                return Err(StageError::new("INVALID_INPUT_SHAPE", "expected stream"));
            };
            let (sender, receiver) = mpsc::channel(4);
            while let Some(value) = input.next().await {
                sender
                    .send(value)
                    .await
                    .map_err(|_| StageError::new("STREAM_CLOSED", "output stream closed"))?;
            }
            drop(sender);
            return Ok(StageOutput::Stream(receiver));
        }
        if self.mode == "stream-single" {
            let StageInput::Stream(mut input) = input else {
                return Err(StageError::new("INVALID_INPUT_SHAPE", "expected stream"));
            };
            let mut values = Vec::new();
            while let Some(value) = input.next().await {
                values.push(value?);
            }
            return Ok(StageOutput::Single(object([(
                "values",
                WireValue::Array(values),
            )])));
        }
        let StageInput::Single(input) = input else {
            return Err(StageError::new(
                "INVALID_INPUT_SHAPE",
                "expected single value",
            ));
        };
        if self.mode == "single-stream" {
            let (sender, receiver) = mpsc::channel(2);
            sender
                .send(Ok(input.clone()))
                .await
                .map_err(|_| StageError::new("STREAM_CLOSED", "output stream closed"))?;
            sender
                .send(Ok(object([("copy", input)])))
                .await
                .map_err(|_| StageError::new("STREAM_CLOSED", "output stream closed"))?;
            drop(sender);
            return Ok(StageOutput::Stream(receiver));
        }

        let operation = input
            .get("operation")
            .and_then(WireValue::as_str)
            .unwrap_or("");
        match operation {
            "error" => {
                Err(StageError::new("FIXTURE", "fixture failed").with_field("line", 3.into()))
            }
            "waitForCancel" => loop {
                context
                    .cancellation
                    .check()
                    .map_err(|error| StageError::new("CANCELLED", error.to_string()))?;
                sleep(Duration::from_millis(2)).await;
            },
            "capabilityDenied" => {
                let value = context.env().get("DENIED").await?;
                Ok(StageOutput::Single(object([(
                    "value",
                    value.map(Into::into).unwrap_or(WireValue::Null),
                )])))
            }
            "malformedResponse" => {
                let value = context.env().get("MALFORMED").await?;
                Ok(StageOutput::Single(object([(
                    "value",
                    value.map(Into::into).unwrap_or(WireValue::Null),
                )])))
            }
            "context" => Ok(StageOutput::Single(context_result(context).await?)),
            _ => Ok(StageOutput::Single(
                input.get("value").cloned().unwrap_or(WireValue::Null),
            )),
        }
    }
}

async fn context_result(context: &StageContext) -> Result<WireValue, StageError> {
    let data = context.storage().read("posts/a.md").await?;
    context.storage().write("out/a.md", &data).await?;
    let entries = context.storage().list("posts").await?;
    let mut watch = context.storage().watch("posts").await?;
    let watched = match watch.next().await {
        Some(Ok(value)) => vec![value],
        Some(Err(error)) => return Err(error),
        None => Vec::new(),
    };
    watch.close().await;
    context.storage().remove("out/stale.md").await?;
    let response = context
        .network()
        .fetch(
            "https://example.com/data",
            object([
                ("method", "POST".into()),
                ("body", "AQID".into()),
                ("headers", object([("x-test", "yes".into())])),
            ]),
        )
        .await?;
    context
        .filesystem()
        .write_absolute("/safe/out", &data)
        .await?;
    let shell = context
        .shell()
        .run(
            "tool",
            &["arg".to_owned()],
            object([("stdin", "aGVsbG8=".into())]),
        )
        .await?;
    context
        .logger()
        .info("runner fixture", object([("ok", true.into())]))
        .await?;
    Ok(WireValue::Object(BTreeMap::from([
        ("bytes".into(), WireValue::Bytes(data.clone())),
        (
            "bounded".into(),
            WireValue::Bytes(context.storage().read_bounded("posts/a.md", 16).await?),
        ),
        (
            "exists".into(),
            context.storage().exists("posts/a.md").await?.into(),
        ),
        ("stat".into(), context.storage().stat("posts/a.md").await?),
        ("entries".into(), WireValue::Array(entries)),
        ("watched".into(), WireValue::Array(watched)),
        (
            "env".into(),
            context
                .env()
                .get("ALLOWED")
                .await?
                .map(Into::into)
                .unwrap_or(WireValue::Null),
        ),
        (
            "envRequired".into(),
            context.env().get_or_throw("ALLOWED").await?.into(),
        ),
        ("nowMs".into(), context.time().now_ms().await?),
        ("nowIso".into(), context.time().now_iso().await?.into()),
        ("home".into(), context.filesystem().home_dir().await?.into()),
        ("temp".into(), context.filesystem().temp_dir().await?.into()),
        (
            "absolute".into(),
            WireValue::Bytes(context.filesystem().read_absolute("/safe/a").await?),
        ),
        (
            "absoluteBounded".into(),
            WireValue::Bytes(
                context
                    .filesystem()
                    .read_absolute_bounded("/safe/a", 16)
                    .await?,
            ),
        ),
        ("status".into(), response.status.into()),
        ("body".into(), WireValue::Bytes(response.bytes().to_vec())),
        (
            "shell".into(),
            object([
                ("exitCode", shell.exit_code.into()),
                ("stdout", WireValue::Bytes(shell.stdout)),
                ("stderr", WireValue::Bytes(shell.stderr)),
            ]),
        ),
    ])))
}

fn object<const N: usize>(entries: [(&str, WireValue); N]) -> WireValue {
    WireValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

#[tokio::main]
async fn main() {
    let mode = std::env::args().nth(3).unwrap_or_else(|| "single".into());
    let status = if run_plugin(FixtureStage { mode }, RunnerOptions::from_args())
        .await
        .is_ok()
    {
        0
    } else {
        eprintln!("forme Rust runner terminated after a protocol failure");
        1
    };
    // Tokio's portable stdin adapter owns a blocking helper thread. A plugin
    // executable is process-scoped, so exit after the runner has flushed its
    // final response instead of waiting for a host-owned pipe to close.
    std::process::exit(status);
}
