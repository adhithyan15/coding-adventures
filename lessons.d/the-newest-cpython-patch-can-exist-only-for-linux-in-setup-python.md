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
`linux/x64` build (3.13.17 at the time), and likewise 3.12.10, 3.11.9 and
3.10.11 for the ABI matrix.

Before choosing or bumping a Python pin, query the manifest for the platforms
the workflows actually run on:

    curl -s https://raw.githubusercontent.com/actions/python-versions/main/versions-manifest.json \
      | python3 -c "import json,sys; [print(r['version'], sorted({(f['platform'], f['arch']) for f in r['files']})) for r in json.load(sys.stdin) if r['version'].startswith('3.13.')][:3]"

The same check applies to any toolchain action that serves prebuilt binaries
per platform: a version existing upstream is not evidence it exists for every
runner OS.
