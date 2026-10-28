### Fixed — the XAML host moves on when a quarantine name is taken after its check

`QuarantineSnapshot` (`host/xaml/MosaicHost.cs`) copies an unreadable snapshot
to the first free `.corrupt` / `.corrupt.N` name with
`File.Copy(..., overwrite: false)`. Any failed copy switched persistence off,
even one that failed because something got to the name after the check. On
Windows that includes a directory, which `File.Copy` reports as
`UnauthorizedAccessException`, not as "already exists". A slot is now skipped
whenever the copy fails *and* something is at that name, the rule #18477
applies to the Electron host. Any other failure still disables persistence.

The check is now `QuarantineNameTaken`. It also counts a link whose target is
gone (`LinkTarget`), so it does not depend on whether `File.Exists` sees a
dangling link on that platform. `tests/package_compiles.rs` pins both.
