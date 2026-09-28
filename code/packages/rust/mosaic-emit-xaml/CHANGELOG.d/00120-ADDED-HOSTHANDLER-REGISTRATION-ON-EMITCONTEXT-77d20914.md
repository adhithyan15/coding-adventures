### Added — `HostHandler` registration on `EmitContext`

- The Host* event handlers are accumulated on `EmitContext::host_handlers`
  during the walk and emitted inline in the code-behind partial class
  after the PR-2 helper methods. The dedup is by handler name, mirroring
  the helper-dedup pattern.

