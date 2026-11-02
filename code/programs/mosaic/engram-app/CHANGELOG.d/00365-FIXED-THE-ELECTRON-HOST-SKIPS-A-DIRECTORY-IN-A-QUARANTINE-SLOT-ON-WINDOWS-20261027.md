### Fixed — the Electron host skips a directory in a quarantine slot on Windows

`quarantineSnapshot` (`host/electron/host.js`) copies an unreadable
snapshot to the first free `.corrupt` / `.corrupt.N` name with
`COPYFILE_EXCL`, and treated only `EEXIST` as "this slot is taken". POSIX
reports a directory in the slot as `EEXIST`, but Windows reports it as
`EPERM` (or `EISDIR`), so on Windows the host switched persistence off
instead of moving to the next slot, and
`snapshot_quarantine::electron_never_replaces_an_earlier_quarantine`
failed in the Windows build. A slot is now skipped whenever the copy fails
and something already exists at that name, on every platform; any other
failure still disables persistence for the session.
