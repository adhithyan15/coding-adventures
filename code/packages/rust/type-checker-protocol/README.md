# type-checker-protocol

Shared generic type-checking interfaces and hook dispatch for Rust compiler frontends.

## Bounded diagnostics

`type_checker_protocol::bounded` caps what a checker can accumulate, so a
hostile program cannot turn errors into an allocation attack (#18474
security review). Push every diagnostic through `push_bounded`. It keeps
at most 256 diagnostics and 1 MiB of messages, cuts each message at 4 KiB,
and appends one `DIAGNOSTICS_TRUNCATED` note. Format lists with
`name_list`, which names 8 items and then "… and K more". Once
`diagnostics_closed` returns true, skip building diagnostics.
