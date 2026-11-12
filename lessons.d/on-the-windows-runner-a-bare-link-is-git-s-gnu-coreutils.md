---
category: BUILD files & dependency management
---

# On the Windows runner a bare 'link' is Git's GNU coreutils link.exe, not MSVC's: drop Git's usr\bin from PATH before C builds

`lua/tcp_client`'s first Windows run compiled every luasocket source with MSVC `cl`, then died in luarocks' link step with `link: unknown option -- d`. The bare command `link` resolved to Git for Windows' GNU coreutils `link.exe` (`C:\Program Files\Git\usr\bin`), which sits ahead of MSVC on PATH; luarocks calls `link -dll -def:...`.

The same shadowing applies to any tool that shells out to a bare `link`, `find` or `sort` on the runner (Git's MSYS tools win over the Windows/MSVC ones of the same name).

Fix: in a PowerShell recipe script (its own process, so nothing leaks), remove Git's `usr\bin` from `$env:PATH`, then assert `(Get-Command link).Source` is under Visual Studio / MSVC / Windows Kits before building. Read the failing log for the first non-compile error: the compile lines passing is not evidence the toolchain works.
