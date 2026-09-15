- `astronomy/comet-tail-type.adj` (new) — a sibling library to the already-shipped `comet-part.adj`:
  a new `comet_tail_type(tail, description)` table names the two separate tails a comet actually
  has (dust_tail, ion_tail) and the defining path each one traces, quoted verbatim from the SAME
  NASA Space Place "What Is a Comet?" page `comet-part.adj` already cites — discovered while
  investigating that page's own header note that it "goes on to further sub-divide the tail into
  a dust tail and an ion tail," a genuinely new sub-question (not a `comet-part.adj` extend, since
  dust/ion are sub-types of the single `tail` part, not new peer-level physical parts) using an
  already-fetched source, re-verified live before writing. Honest abstention on `coma`/`nucleus`
  (real comet parts, already tabled in the coarser sibling `comet-part.adj`, not tail sub-types).
  This is genuinely a CLOSED two-way split — the source's own sentence states a comet has exactly
  two separate tails. New manifest objective `adj.science.3to5.comet_tail_type` (168 total,
  prerequisite on `adj.science.3to5.comet_part`) — the first genuinely NEW-topic content slice in
  a while, after a run of extend-pattern wins; a full sweep of ~20 other science-lane tables this
  cycle found nothing else extendable (all closed/exhaustive sets or documented considered
  exclusions). New e2e test `facts_comettailtype_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on a different physical part).
