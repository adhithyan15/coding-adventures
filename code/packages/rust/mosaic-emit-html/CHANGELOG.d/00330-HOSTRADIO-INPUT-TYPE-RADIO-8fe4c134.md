- `HostRadio`    → `<input type="radio" …>`

Static HTML has no JS runtime, so the standard pattern from
`HostInput` and `HostButton` is reused — slot-typed props become
`data-*` markers the host's template engine or hydration pass can
post-process:

