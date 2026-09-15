# The benchmark shape decides the answer

Base64 for media looked like a clear win on wire size, so I measured peak
memory to confirm. One 4 MiB asset:

```
  array: 3.57x wire, 2.00x peak   |   base64: 1.33x wire, 2.67x peak
```

Base64 was **worse** on peak. Reported as-is that would have been a real
argument against the change — and it would have been misleading, because one
asset is base64's worst possible case. The intermediate encoded string is
per-asset, while the output buffer holds every asset. An `.apkg` is many small
files, not one enormous one:

```
  2048 x 16 KiB (32 MiB) | array 3.57x wire 2.00x peak | base64 1.33x wire 1.34x peak
```

Same code, opposite conclusion. The first benchmark was not wrong; it measured
a shape the product does not have.

**Pick the fixture from the workload, not from what is easy to write.** A
single big buffer is the convenient benchmark, and here it inverted the result.
Both numbers are now in the source comment, including the case where base64
loses, so nobody has to rediscover it.

Two smaller things from the same change:

- **A second copy of the same bug lived one crate away.** Fixing
  `MediaAssetRecord` left `ResolvedMediaFile` in `engram-anki-package`
  untouched, and that one backs `eg_read_anki_apkg_media` — a response that IS
  a single media file. Only a downstream test failing on `left: Array` vs
  `right: String` pointed at it. When a fix is about a *shape* rather than a
  site, grep for the shape: `pub data: Vec<u8>` found it in seconds.
- **Test fixtures that look like expectations are not.** Three failing
  assertions held numeric arrays; two were inputs and one was an expectation.
  Blanket-replacing all three would have passed CI while deleting the only
  coverage of the legacy read path — the path that keeps every snapshot already
  on a user's disk loadable. They are classified individually now, with the
  input ones left as arrays deliberately.
