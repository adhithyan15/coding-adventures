# chief-of-staff-agent-stdio-protocol

Pure host-side codec for D18 Level 4 agents. A Level 4 agent can be written in
any language that reads stdin, writes stdout, parses JSON, and handles base64.
It needs no Chief of Staff SDK.

The host writes one `message` object per line. The agent writes one correlated
`response` object per line. The codec owns validation and framing but does not
open pipes, spawn a process, read a clock, or acknowledge a channel message.
Those effects remain in the future subprocess host adapter.

```json
{"protocol":"chief-agent-stdio-v1","kind":"message","message_id":"019...","channel_id":"019...","sequence":"7","timestamp_ns":"42","content_type":"text/plain","payload_b64":"aGVsbG8="}
{"protocol":"chief-agent-stdio-v1","kind":"response","input_message_id":"019...","content_type":"text/plain","payload_b64":"d29ybGQ="}
```

The response must name the input message currently in flight. A malformed,
oversized, duplicate-key, non-canonical, or mismatched response fails closed so
the host does not publish output or advance the input cursor.
