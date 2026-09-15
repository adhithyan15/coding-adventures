- `physics/circuit-parts.adj` — **the locator named a file that never existed**, and the table is
  converted to per-row provenance (RS-5e, #14986) on the corrected source.

  #15199 recorded this as the stdlib's one dead link: `.../circuit_basics_and_components.pdf`,
  404 on both `http` and `https`. It is worse than dead. The Wayback CDX index holds **no capture
  of that address at any time**, while **two** separate files in the same folder —
  `circuit_basics.pdf` and `circuit_components.pdf` — both returned 200 and are archived. The
  shipped filename is those two names run together. A dead link is a page that went away; this
  was an address that was never right.

  **The quote was sound all along.** The shipped span occurs in `circuit_components.pdf`, which
  defines all seven parts. So the sentence really was taken from a real document, and only the
  address recorded for it was wrong. The locator now points at a **verified capture** of that file
  (HTTP 200, 449,715 bytes), because the live URL is 404 today and pointing at it would restore a
  correct-but-unfetchable address. Nothing here claims more than `source_labeled` — this is an
  address a reader can open, not a content hash.

  **"Verbatim" is weaker here than for this cascade's HTML tables, and the file says so:** the
  source is a PDF whose text layout wraps sentences across lines, so each span matches only after
  whitespace collapsing. Measured against the captured file: each of the seven occurs **exactly
  once**, none names another row's key, and a fabricated control sentence is reported absent by
  the same comparison. Both controls behaved as intended.

  All seven rows were warranted by the BATTERY sentence, so `? circuit_part_role(resistor, $R)`
  came back proved by a sentence about batteries; six of the seven were in that position. Each row
  now carries its own definition.

  **The existing test passed unchanged through both changes, and that is the finding.** Its
  citation assertion was `contains("k12maker.mit.edu") && contains("\"trust\":\"consensus\"")` —
  the #15139 two-loose-needles shape — and `k12maker.mit.edu` is a substring of the archive URL as
  well as the original, so swapping a never-existent address for a real capture moved nothing it
  could see, and neither did converting one envelope span into seven row spans. It is now one
  contiguous span including the whole locator, joined by four new tests: per-row warrants with a
  negative arm on single-answer queries, the envelope reaching exactly one answer (the battery
  row's own copy, so 2 occurrences and not 14), a structural read of the shipped file, and a named
  regression guard forbidding the conflated filename **as a locator value** — narrowed from "
  anywhere in the file", because the header now quotes it while explaining the defect.

  Writing that guard bluntly first is what found a **stale provenance listing seventy-five lines
  below**, still naming the dead URL as "the same primary source" and still describing the
  pre-conversion design. Same prose-contradicts-data defect as `em-spectrum`. (An earlier wording
  said "two hundred lines below". The file is 180 lines long, so nothing in it can be.)

  **Security review then found the twin that sweep missed.** The provenance block still
  attributed the pairs to a handout titled *"Circuit Basics and Components"* — a title occurring
  **zero times in either PDF** (`circuit_components.pdf` is "Circuit Components",
  `circuit_basics.pdf` is "Circuit Basics"). The same run-together mistake as the filename, thirty
  five lines above the paragraph that corrects the filename: I swept the URL and never grepped the
  prose for its twin. Review also found a **non-verbatim quote inside a block headed "the verbatim
  span"** — the `switch` entry opened with *"This switch serves as an On/Off switch in a
  circuit."*, which occurs zero times in either PDF; the source's own wording is ungrammatical
  (*"Toggle SPST switches are often serve as an On/Off switches in a circuit."*) and was silently
  tidied at some point into something that reads well and was never written. The **shipped row was
  always the verbatim second sentence**; only the listing overstated. And the claim that the
  capture "names the original URL inside it" is false — `k12maker` occurs zero times in the
  captured bytes; the Memento response headers carry that.

  8 of 8 mutants killed, green baseline before and after, file verified byte-identical
  afterwards; both controls behaved as intended — the positive one (a phrase certain to be in the
  document) and the negative one (a fabricated sentence). Local scratch harness, so that count is
  not reproducible from the repo. Security review independently built its own 13 mutants and
  killed 13, and independently confirmed the CDX result with a folder-level control (3,244 rows
  captured under that directory, **zero** for the conflated filename) — so the absence is a real
  absence and not a gap in the index.
  8 of 8 mutants killed, green baseline before and after, file verified byte-identical afterwards.
  Local scratch harness, so that count is not reproducible from the repo.
