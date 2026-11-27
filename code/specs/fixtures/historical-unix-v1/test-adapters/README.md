# Historical Unix fixture adapters

These trace adapters exist only to replay `file-read.json` from package
`BUILD` and `BUILD_windows` recipes. Each recipe compiles or copies its adapter
to a temporary build location; none is part of a language package API or
distribution. `build_native.py` runs the native toolchains and replay in one
Python process so its temporary output paths remain valid across executor
shell boundaries. The public C, C++, Java, and Lua packages
expose reusable in-memory simulator libraries. Publishing a separate
user-facing CLI requires an explicit `stdout:write:*` capability and its
required hardware-key approval.
