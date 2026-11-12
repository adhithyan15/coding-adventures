---
category: Supply chain & CI pinning
---

# The newest CPython patch can exist only for Linux in setup-python, so pin the newest patch every runner OS has

When the workflows' `python-version: '3.13'` was pinned to an exact patch, the
obvious choice was the newest one, 3.13.18. In
`actions/python-versions/versions-manifest.json` that release had a build for
`linux/x64` only: CPython's security-only releases are source-only upstream,
and the action builds them for Linux alone. `'3.13'` had quietly been resolving
to 3.13.18 on Ubuntu and to an older patch on Windows and macOS, so the
"same" version was already different per OS, and pinning 3.13.18 would have
failed outright on every Windows and macOS job.

The pin used is the newest patch with a `win32/x64`, `darwin/arm64` and
`linux/x64` build (3.13.17 at the time).

**Linux builds are also per Ubuntu release.** The first attempt pinned the
ABI matrix to 3.10.11 / 3.11.9 / 3.12.10 -- the last patches with Windows and
macOS builds -- and the ubuntu-latest cells failed with "The version '3.11.9'
with architecture 'x64' was not found for Ubuntu 26.04": ubuntu-latest had
moved to 26.04, and only the recent security patches are built for it. For
3.10-3.12 no single patch exists on every runner OS, so those matrices pin
per OS (Linux 3.10.22 / 3.11.18 / 3.12.16; macOS and Windows 3.10.11 /
3.11.9 / 3.12.10). Check `platform_version` for the Ubuntu release
`ubuntu-latest` currently maps to, not just `platform`.

Before choosing or bumping a Python pin, query the manifest for the platforms
the workflows actually run on:

    curl -s https://raw.githubusercontent.com/actions/python-versions/main/versions-manifest.json \
      | python3 -c "import json,sys; [print(r['version'], sorted({(f['platform'], f.get('platform_version'), f['arch']) for f in r['files']})) for r in json.load(sys.stdin) if r['version'].startswith('3.13.')][:3]"

The same check applies to any toolchain action that serves prebuilt binaries
per platform: a version existing upstream is not evidence it exists for every
runner OS.
