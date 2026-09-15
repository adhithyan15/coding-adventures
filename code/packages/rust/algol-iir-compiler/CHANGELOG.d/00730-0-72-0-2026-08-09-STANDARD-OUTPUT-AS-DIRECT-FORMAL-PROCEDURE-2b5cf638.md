## 0.72.0 — 2026-08-09 — standard output as direct formal procedures

A direct `procedure` actual may now be the implementation-defined standard
output procedure `print` or `output`. Specialised wrappers retain that direct
target and reuse the established statement-only `print_str` lowering at each
formal call site, including nested forwarding, without inventing an IIR
procedure signature or runtime function-pointer ABI. Dynamic procedure values
remain unsupported.

