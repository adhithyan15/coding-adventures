- **#14986: seven joints, and a header that had written down its own defect.**
  `anatomy/joint-types.adj` warranted all seven rows with the HINGE row's own examples sentence, so a
  recall of the HIP came back proved by *"Examples include the elbow, knee, ankle, and interphalangeal
  joints."* Each row now carries its own span. One page, so no row restates `locator` or `trust` —
  unlike `biology/hormone-glands.adj`, whose twelve rows sit on seven pages.

  ### The old header had already written down the defect — and so had the README

  Its `saddle → thumb` entry quoted *"This joint allows the thumb to flex and extend…"* and then
  explained, in a parenthetical underneath, **which joint "This joint" meant**. A quote that needs a
  note to say what it is about is precisely what the README rule ("A citation must name its own
  subject") exists to catch. The row now carries all four sentences — type, example, and the link
  between them — 369 characters, because the page never puts "saddle" and "thumb" in one sentence.

  **[`README.md:556`](README.md) already used this table's own sentence as its worked example of the
  defect**, under the comment *"WRONG — an example of WHAT?"*: `"One example is the joint formed by
  the trapezium and 1st metacarpal bone."` That sentence is inside the widened saddle span now, with
  the two sentences that answer the question. The rule was written against this file; this entry is
  the file catching up to it.

  (A draft of this section said the header described the defect "a year before it could be seen".
  Measured: the header landed 2026-07-17, RS-5e shipped **three days later** on 2026-07-20, and the
  fix is today, 2026-09-13 — 58 days after the mechanism existed. No reading of those dates is a
  year.)

  `hinge` and `planar` are widened for the ordinary reason: both of their sentences open *"Examples
  include…"* and name no joint type.

  ### Why this page allows widening and two others do not

  Probing the convertible queue turned up the distinction that decides whether a table is ordinary
  work. Here the type-defining sentences are **prose** (*"Planar joints are multiaxial but restricted
  by the surrounding ligaments. Examples include the acromioclavicular, intercarpal, and intertarsal
  joints."*), so widening reaches them.

  In `language/idiom-meaning.adj` the idiom is a numbered **heading** (`4. “Piece of cake”`) and the
  meaning a separate line; in `physics/energy-form-family.adj` the potential/kinetic grouping is a
  **section heading** and no sentence assigns a form to a family. No widening reaches a heading, so
  both are structure-grounded and neither is a mechanical conversion. Both had been listed as
  convertible on #15139 and are now withdrawn there.

  ### Pins — and a defect mutation found in my own tests

  **8 of 8 mutants killed, two controls counted separately.** Un-widening `hinge`, `planar` and
  `saddle`; removing the page's hyphenation from a `ball-and-socket` quote; rebinding `pivot`;
  breaking one of the two shared-sentence copies; dropping a row's source; repointing the locator.

  Two of those **survived at first, and the fault was in the test, not the data.** `hip` and
  `shoulder` are two rows sharing one sentence, and the test queried by joint TYPE — which returns
  both rows, so `contains(span)` was satisfied by whichever copy was still intact. Breaking only the
  shoulder row stayed green. Rewritten to bind the EXAMPLE, which returns exactly one answer and makes
  each needle belong to the row under test. The same shared-sentence shape (estrogen and progesterone)
  passed cleanly in the `hormone-glands` entry below **only because a hormone query returns one row** —
  luck, not design, and the reason that pattern is now stated rather than repeated.

  Controls: unmutated green, and a fabricated envelope green — disclosed, because once every row
  overrides `source` the envelope's wording is unreachable from any answer. The `trust` default
  disclosed in the entries below applies here unchanged.

  The existing citation assertion pinned only the host, the shape #15139 found rotting in eight files;
  it now pins the whole `"locator":"…","trust":"…"` pair.

