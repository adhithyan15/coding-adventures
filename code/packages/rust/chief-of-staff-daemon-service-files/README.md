# chief-of-staff-daemon-service-files

Pure renderers for the user-scoped service definitions required by D18. The
package accepts explicit absolute paths to `chief-of-staff-daemon` and its TOML
configuration, then returns deterministic file contents for:

- macOS `~/Library/LaunchAgents/dev.chiefofstaff.plist`
- Linux `~/.config/systemd/user/chief-of-staff.service`
- Windows `%APPDATA%\ChiefOfStaff\Tasks\daemon.xml`

The definitions start at login, run without elevation, keep one daemon
instance, and restart after abnormal exit. launchd and systemd stop the daemon
with SIGTERM, which is handled cooperatively by `chief-of-staff-daemon`.
Windows uses an interactive-token logon task because the current daemon is a
console application rather than a Windows Service Control Manager executable.

This package intentionally performs no writes and starts no programs. A future
`chief-of-staff install-daemon` command can preview the returned content, write
it with owner-only policy, and invoke the native registration command without
putting paths through a shell.

Typical registration commands after securely writing the rendered file are:

```text
launchctl bootstrap gui/<uid> ~/Library/LaunchAgents/dev.chiefofstaff.plist
systemctl --user daemon-reload
systemctl --user enable --now chief-of-staff.service
schtasks /Create /TN \ChiefOfStaff\Daemon /XML %APPDATA%\ChiefOfStaff\Tasks\daemon.xml /F
```

## Validation

```sh
sh chief-of-staff-daemon-service-files/BUILD
```
