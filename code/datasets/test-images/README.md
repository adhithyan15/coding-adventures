# test-images — synthetic test image suite (CC0)

A small, deterministic suite of 256×256 RGB PPM (P6) images for use in
image-processing demos and test code.  All images are dedicated to the
public domain under [CC0 1.0](LICENSE) — no attribution required, no
restrictions.

## Why synthetic?

The classic image-processing test photos (Lenna, Peppers, Mandrill,
Cameraman) all have at-best-fuzzy licensing.  Synthesising the suite
sidesteps the question entirely: the images are produced by
deterministic code we own, so they're trivially CC0, byte-for-byte
reproducible across machines, and we can tune each one to exercise a
specific filter property.

## The images

| File | Size | What it stresses |
|------|------|------------------|
| `gradient_quad.ppm` | 256×256 | RGB quadrants + a yellow disc on near-black background.  Sanity check + visually obvious filter outputs. |
| `peppers_synthetic.ppm` | 256×256 | Five saturated colour blobs (red / yellow / green / orange / crimson) over a dark blue-grey background.  Stand-in for the classic "Peppers" image — broad smooth surfaces in distinct hues for sepia and colour-matrix work. |
| `zone_plate.ppm` | 256×256 | Radial sinusoidal pattern of increasing frequency.  Aliasing / resampling / anti-alias filter test (grayscale). |
| `gamma_ramp.ppm` | 256×256 | 16 calibrated grayscale steps next to a smooth horizontal gradient.  Posterise and gamma show banding clearly. |
| `mandrill_proxy.ppm` | 256×256 | Procedural high-frequency noise in mixed warm hues.  Stand-in for the classic "Mandrill" image — high-frequency content for sharpening / posterise / contrast tests, kept entirely original (no recognisable face). |

Renders of each image are produced by `gen_test_images.py`; the rendered
PPMs are committed alongside the script so consumers don't need Python
to use them.

## Regenerating

```sh
python3 code/datasets/test-images/gen_test_images.py
```

The script is deterministic — re-running it produces byte-for-byte
identical output, so a `git diff` after a regeneration should show
nothing changed.  Add a new image by:

1. Adding a generator function to `gen_test_images.py`.
2. Adding an entry to `IMAGES` in the same file.
3. Running the script.
4. Adding a row to the table in this README.
5. Committing the new `.ppm` next to the script change.

## Why PPM (P6) and not PNG?

PPM is the simplest possible RGB-bytes-with-a-tiny-header format.  It
costs us 17 bytes of overhead per file and lets every demo program in
the repo stay zero-dependency: `image-codec-ppm` is a few hundred
lines, no external image-codec crate needed.  Workspaces that want
PNG/JPEG/etc. can convert at the boundary.

## Using the images

From the `instagram-filters` demo:

```sh
instagram-filters \
  --input  code/datasets/test-images/peppers_synthetic.ppm \
  --output /tmp/peppers_sepia.ppm \
  --filter sepia
```

For a rendered-PNG version (e.g. to embed in docs), pipe through any
PPM-to-PNG converter.  An in-tree round-trip is:

```sh
# Build a small converter using image-codec-ppm + paint-codec-png from
# the workspace.  See `code/programs/rust/image-format-roundtrip/`
# (when it lands) — meanwhile, ImageMagick's `convert` does the job.
```

## Adding non-synthetic images later

If we ever want to add real photographs, the cleanest sources are:

- **NASA imagery** — explicitly public domain (`https://images.nasa.gov`).
- **Wikimedia Commons** filtered to "Public domain" or "CC0".
- **Creative Commons CC0 search** — https://wordpress.org/openverse/.

Files added from those sources should land in their own subdirectory
(e.g. `code/datasets/test-images/nasa/`) so the licence story stays
clean per-source.
