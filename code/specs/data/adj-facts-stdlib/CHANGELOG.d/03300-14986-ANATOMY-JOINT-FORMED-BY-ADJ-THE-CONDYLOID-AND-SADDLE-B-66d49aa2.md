- **#14986: `anatomy/joint-formed-by.adj` — the condyloid and saddle bones stop being warranted by the pivot sentence.**
  The envelope was the pivot row's own sentence, *"The atlantoaxial joint, formed by the 1st (atlas) and
  2nd (axis) cervical vertebrae, is a pivot joint."*, so it was the primary source of all six answers.
  The condyloid and saddle bones were read across two table-level `cites`.

  ### What each row carries now

  Measured 2026-09-15 on the NCBI StatPearls "Anatomy, Joints" page (HTTP 200; a nonsense path on the
  same host returns 404), with inline tags removed without a space and only ASCII whitespace collapsed.
  The nonsense page holds none of the spans below.
  - **Both pivot rows** take the pivot sentence, which names the type and both bones.
  - **Both condyloid rows** take the knuckles sentence, which names the type and both bones.
  - **Both saddle rows** take the page's three contiguous sentences, *"A saddle joint is an articulation
    between 2 saddle-shaped bones, which are concave in one direction and convex in another. This joint
    type is biaxial. One example is the joint formed by the trapezium and 1st metacarpal bone."* No
    single sentence names the saddle joint and its bones: the bone sentence says only "One example",
    and the biaxial sentence sits between the two. This follows the two-sentence smooth-muscle span in
    `biology/muscle-types.adj`.
  - **The envelope** is now the page's *"Joint classifications offer a broad understanding of joints."*
    It names no joint type and no bone.
  - Each of the three spans and the envelope occurs **exactly once**, inside a `<p>`, both with script
    and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  The header's "reproduces, byte-for-byte ... no new WebFetch" provenance note is replaced by the
  measurement.

  ### Pins

  The test keeps every behaviour it shipped with: the two-answer pivot recall, the reverse recall to
  saddle, and the abstention on hinge. Its `contains("ncbi.nlm.nih.gov") && contains(trust)` check
  (#15209's shape) becomes two answers, each whose citations array holds exactly the pivot sentence's
  whole citation. Added:
  - every bone's answer is one answer whose citations array holds exactly its own span's citation, with
    no other joint type's span and not the envelope;
  - both saddle rows carry the whole three-sentence span, and neither carries the bone sentence alone;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the trapezium row cut to the bone sentence alone;
  - a condyloid row warranted by the pivot sentence;
  - the axis `source` demoted to `cites`;
  - the atlas row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a bone atom rebound;
  - a joint type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
