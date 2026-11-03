### Fixed — the XAML host never burns a quarantine name on a failure

`QuarantineSnapshot` (`host/xaml/MosaicHost.cs`) could switch saving off and
report "Engram already holds 10 unreadable snapshots" when it held none:

- **An inspection error counted as taken.** `QuarantineNameTaken` caught every
  exception and answered `true`, so a fault that hid one name walked all ten.
  It now hands back the error (`could not inspect <name>: ...`), and the
  quarantine stops there, switches saving off and says so.
- **A failed copy could leave a partial file behind.**
  `File.Copy(path, corrupt, overwrite: false)` could fail partway and leave a
  partial copy, which every later attempt saw as taken. The name is now
  claimed and written in one step: `new FileStream(corrupt, FileMode.CreateNew,
  ...)` creates it exclusively and the bytes are copied through that handle.
  If the copy fails, the host deletes exactly that file (while its handle is
  still open), switches saving off and reports the error. The snapshot is
  opened for reading before anything is claimed, so an unreadable snapshot
  uses no name at all. The copy is still byte for byte, and an existing
  quarantine is still never replaced.
- Every exception from the copy is handled, not only `IOException`: .NET
  reports a write past the file-size limit as `ArgumentOutOfRangeException`,
  which previously escaped `HydrateSession` with the partial file left behind.

`tests/package_compiles.rs` pins the claim, the re-check after it, the
partial-copy removal, and the inspection error. The host needs WinUI, so the
behaviour was exercised by compiling these methods, verbatim, into a console
harness: exact-bytes copy to the first free name, ten held names, an
unreadable snapshot, a NUL path (old: "already holds 10"; new: the inspection
error), and twelve copies failed by `ulimit -f` (no name left behind).
