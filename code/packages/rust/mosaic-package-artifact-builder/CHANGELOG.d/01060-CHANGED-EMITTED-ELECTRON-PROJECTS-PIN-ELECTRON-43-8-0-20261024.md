### Changed — emitted Electron projects pin Electron 43.8.0

`build_electron_package_json` hardcoded `"electron": "42.5.0"` in every project
`mosaic-compile pkg --backend electron --emit-project` writes. The repo's
hand-written Electron apps moved to 43.8.0 for its security fixes, and 42.5.0
predates all of them, so every Mosaic-emitted desktop shell kept the old
runtime. The emitter now pins 43.8.0: the newest 43.x release older than the
seven-day release floor the Engram lock regeneration enforces (43.8.1 was
published on 2026-10-20 and clears it on 2026-10-27). The UI32-M Electron shell
test now asserts the pin, so a later bump has to update it deliberately.
