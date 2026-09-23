---
category: CI & GitHub Actions
---

# Verify a generated-artifact check in the same generation mode CI uses

**What went wrong.** I added a QML render check for the Qt navigation split
(#15833), verified it locally both ways — passes on the fixed output, fails on
the broken one — and it still failed on its first CI run:

```
Shell.qml:12:5: Required property mosaicHost was not initialized
qml: Did not load any objects, exiting.
```

Not a wrong assertion. The component would not instantiate at all.

The cause was the generation flags. Locally I ran:

```
mosaic-compile pkg <fixture> --backend qt --output <dir>
```

CI runs:

```
mosaic-compile pkg <fixture> --backend qt --output <dir> \
    --emit-project --profile native-complete --runtime-library <lib>
```

and **that mode emits a different component surface**: `required property var
mosaicHost` instead of `property var mosaicHost: null`. My check never supplied
a host, which is fine against an optional property and fatal against a required
one.

**What to do differently.**

- **Reproduce the generation command, not just the artifact.** For a check that
  consumes generated output, copy the exact flags out of the CI step before
  running it locally. `--emit-project`, a profile, a runtime library — any of
  them can change the generated API, not merely where files land.
- **The both-ways check does not cover this.** "Passes on fixed, fails on
  broken" proves the assertions discriminate. It says nothing about whether the
  harness can load the artifact CI actually produces. Those are separate
  failures and the first one hides behind the second.
- **After fixing the harness, re-run the broken case.** Making a check load
  successfully is exactly the kind of edit that can also make it stop
  discriminating. I re-confirmed exit 1 on the pre-fix QML after adding the
  stub host.
- Related, same family: [[a-text-assertion-on-generated-markup-cannot-see-that-the-markup-renders]].
