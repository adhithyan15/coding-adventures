# Splitting a chunk to satisfy a "largest chunk" gate is gaming the metric

The first fix for the bundle failure gave the oversized `curriculum-plans` group
a `maxSize` so it split into four chunks, each under the ceiling. The gate went
green. It was the wrong fix, and `main` landed the right one concurrently:
make the per-track plans lazy so the bytes leave the preload set entirely.

The tell was in the commit message that shipped it — it conceded that "the bytes
are eager either way, so first paint downloads the same total" and argued the
cacheability gain made it acceptable. A gate measuring the LARGEST eager chunk
cannot see four chunks totalling the same half-megabyte. HL-C110 had written
this exact trade down in advance as gaming the metric.

**Generalisation:** when a budget is expressed as a max over a partition, any fix
that changes the partition rather than the total satisfies the gate without
buying the thing the gate exists to protect. Ask what the gate is a proxy for —
here, bytes before first paint — and move that number.
