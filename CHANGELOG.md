# Changelog

All notable changes to the coding-adventures monorepo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Atbash language-neutral fixture consumers

- Added a bounded stdlib-only generator and fail-closed drift gate that turns
  all six normative Atbash objects into native tests for every established
  implementation lane, with complete expected-text assertions and no new
  production authority.

### Vigenere language-neutral fixture consumers

- Added a bounded stdlib-only generator and fail-closed drift gate that turns
  all 26 normative Vigenere objects into native tests for every established
  implementation lane, with complete expected-object and normalized-error
  assertions and no new production authority.

### Scytale language-neutral fixture consumers

- Added a stdlib-only generator and drift gate that turns all 18 normative
  Scytale objects into native tests for every established implementation lane.
  The generated tests compare complete text, error, and ordered-candidate
  results without adding production dependencies or runtime capabilities.
- Hardened the generator's source boundary with safe identifiers, bounded
  fixture and output reads, interpolation-safe native string construction, and
  fixture-derived resource-limit inputs and normalized error assertions.

### Security — the books workflow ran `latexmk` unhardened, and its compile gate was never wired up

- **Fixed arbitrary code execution in `.github/workflows/human-languages-books.yml`.**
  The job compiled every book with `latexmk -xelatex … book.tex` after `cd`-ing
  into `code/learning/human-languages/<track>/book`. `latexmk` reads `latexmkrc`
  / `.latexmkrc` from its working directory and hands them to Perl's `eval`, and
  that directory is repository content — so a pull request adding
  `<track>/book/latexmkrc` executed arbitrary Perl on the runner before any TeX
  was parsed. The same invocation ran with TeX Live's *restricted* shell escape
  rather than none. Demonstrated by execution, not by inspection: a benign
  marker `latexmkrc` was written by the old command line and not by the new one.
- **Root cause was drift, so the fix removes the second call site rather than
  copying flags into it.** `-norc -r code/scripts/latexmk-safe.rc` was already
  correct in `check-book-compile.sh`, `build-books-locally.sh`,
  `verify-human-languages.sh` and every track's `book/build.sh` — and absent from
  the workflow, the only place the books are compiled at scale. The workflow now
  invokes `check-book-compile.sh` itself, so there is one hardened invocation in
  the repository and CI cannot drift from it again.
- **Added `code/scripts/check_no_book_latexmkrc.py`**, a repository lint that
  fails if any casing of `latexmkrc` / `.latexmkrc` appears under the book tree,
  plus 15 unit tests. Defence in depth: a flag protects the call sites somebody
  remembered to type it at, and this protects the ones not written yet. It
  reports an unreadable directory as `COULD NOT DETERMINE` with its `errno`
  named, and exits non-zero — never as "clean".
- **Added a CI step that verifies the hardening was in effect** by grepping every
  `book.log` for XeTeX's own `write18 enabled` banner, rather than trusting that
  the flags were typed. This catches a moved rc file and the older-`latexmk`
  quirk where `-xelatex` overwrites `$pdflatex` and the rc's `$xelatex` is never
  consulted.
- **`check-book-compile.sh` gained `--strict`, and CI now runs it.** The script
  was the only gate proving the generated LaTeX actually compiles and no workflow
  referenced it; worse, with no SVG-to-PDF converter installed it printed
  `compiled 0, skipped 1, failed 0` and exited **0** — a gate reporting success
  having verified nothing. `--strict` turns every "could not verify" into a
  failure that names the missing dependency, and fails a run that compiled zero
  books. Local runs stay lenient and now say so in their own output, so a local
  pass is never mistaken for a CI-grade pass.
- **`check-book-compile.sh` now pins `openout_any=p`** rather than inheriting the
  local distribution's `texmf.cnf` default. `TEXMFOUTPUT` is deliberately left
  unset and the reason is recorded in the script: files under it are *exempt*
  from the paranoid check, so setting it would widen what TeX may touch.
- Changes to `check-book-compile.sh`, `latexmk-safe.rc` and the new lint now
  appear in the workflow's path filters. They did not before, so a pull request
  editing the script that compiles every book never ran it.
- **Split the write-scoped token away from the job that executes pull-request
  code.** The old `build-and-publish` job held `permissions: contents: write`
  on a `pull_request` trigger while running `npm ci`, a TypeScript build, seven
  `node` checks and XeLaTeX over repository content — and `actions/checkout`'s
  default `persist-credentials: true` left that token in `.git/config` inside
  the workspace latexmk enters. Hardening the latexmk call while leaving that in
  place would have fixed one door in a room with no walls. Now `build` has
  `contents: read` and `persist-credentials: false`; a separate `publish` job
  holds `contents: write`, runs no repository code, and is gated at job level on
  a push to `main`.
- **The compile now emits a `--manifest` of what it actually built, and the
  collection step publishes from that.** Re-deriving the list with
  `find -type d -name book` asked a different question: `check-book-compile.sh`
  skips a directory with no `book.tex`, so a pull request adding nothing but
  `<track>/book/book.pdf` would have had an attacker-authored file uploaded as a
  build artifact and, on `main`, published to Pages and attached to the Release.
  Verified by planting exactly that file and watching it stay out of the
  manifest.
- Added a `book.pdf` symlink guard to `check-book-compile.sh` (the adjacent
  figure-PDF guard existed; this one did not) and a matching check on the
  consuming side, so the two steps need not trust each other.
- Deleted the workflow's own SVG-to-PDF conversion step. It duplicated what
  `check-book-compile.sh` does per track, and unlike the script it wrote to an
  unchecked derived path, so a committed `figures/diagram.pdf` symlink made
  `rsvg-convert` open an arbitrary runner-writable path for writing.
- The shell-escape verification step now counts the logs it read and fails if it
  read fewer than the compile reported. A loop over zero files left `offenders`
  at 0 and printed a confident all-clear — the same hollow-gate shape as the
  `--strict` fix, in the step meant to prove the fix works.
- The latexmkrc lint runs twice: early for a fast failure, and again immediately
  before the compile. Between the two, the job runs `npm ci` and seven `node`
  scripts, all pull-request-controlled and all able to write a `latexmkrc` after
  the early check has passed.
- `npm ci` for `human-language-data` now passes `--ignore-scripts`, matching its
  four sibling installs. It was the only one without it, so a dependency's
  `postinstall` ran on a pull-request runner.
- **Banned symlinks outright under the book tree, replacing a filename-based
  guard that covered one of eight cases.** A XeLaTeX run writes `book.aux`,
  `book.log`, `book.toc`, `book.out`, `book.xdv`, `book.pdf` and latexmk's own
  `book.fdb_latexmk` and `book.fls` into the book directory. `openout_any=p`
  vets the *name* and then opens it, so it follows a symlink for every one of
  them — and the last two never see a TeX-side check at all, being written from
  Perl. `<track>/book/book.aux -> ~/.ssh/authorized_keys` was therefore an
  arbitrary write as the build user, from a pull request, needing no shell
  escape. Guarding `book.pdf` alone locked one door of eight. Enumerating the
  dangerous cases loses to banning the category; there are zero symlinks under
  the tree today, so the ban costs nothing. `.gitignore` is not a boundary here
  — those names are ignored, but `git add -f` commits them anyway.
- The lint is renamed `check_book_tree_hygiene.py` to match what it now
  enforces, and reports a symlink with its target rather than only its path.
- **The shell-escape verification step now iterates the manifest** rather than
  re-finding logs with `find -path '*/book/book.log'` and comparing counts.
  `-path` matches at any depth, so a committed decoy log could pad the count and
  mask a real one that had dropped out; a count was standing in for an identity
  check. Iterating the manifest removes the count comparison entirely.
- A failed manifest append is now fatal. The script runs `set -uo pipefail`
  without `-e`, so a silent failure would have left a book out of the manifest
  while the summary still read "every selected track was compiled and verified"
  — the vacuous-pass class this work exists to remove. `--manifest=` with an
  empty value is now an error rather than a silent no-op.
- Added `code/scripts/tests/test-check-book-compile-guards.sh`, which runs the
  real script against a real symlink and a real planted `book.pdf`. It skips the
  symlink case where the filesystem cannot create one and runs it on CI.
  Confirmed non-vacuous by mutation: removing the `book.tex` guard turns it red.
- **`check-book-compile.sh` now sweeps its own book directory for symlinks**
  before writing anything, rather than relying on a lint only CI invokes. The
  script is documented as the one a human runs locally, and locally nothing runs
  that lint — so `git checkout <branch> && ./code/scripts/check-book-compile.sh`
  on Linux or macOS still wrote through `<track>/book/book.aux -> ~/.ssh/…`,
  which is author-controlled content, not merely a destructive overwrite. The
  PR's own thesis one level up: a control protects only the call sites that
  invoke it.
- **A skipped symlink test is now a failure where the capability must exist.**
  Both symlink suites skip on a filesystem that cannot create links, and nothing
  asserted the probe had succeeded on the runner — so a broken runner image
  would silently stop exercising the security tests while the step stayed green.
  CI sets `REQUIRE_SYMLINK_TESTS=1` and the skip becomes fatal. Verified in both
  directions.
- **A symlink named `latexmkrc` is now reported under both bans, not just one.**
  The symlink advisory ends "Replace the link with the real file, or delete it",
  which for that path instructs the author to create a real `latexmkrc` — the
  artefact the other ban exists to keep out. De-duplicate the failure, never the
  guidance.
- `--book-root=` with an empty value is rejected rather than silently reverting
  to the real corpus (the same bug as `--manifest=`, one line apart), and the
  seam is gated behind `CHECK_BOOK_COMPILE_SELF_TEST=1`.
- The shell-escape verification step checks `-L` before `-f` on each `book.log`,
  since `-f` follows a link and would read an attacker-chosen file.
- **Fixed `check-book-compile.sh` and `verify-human-languages.sh` shipping mode
  `100644` while documenting `./code/scripts/<script>.sh` as their usage.** The
  documented invocation had never worked on a fresh Linux or macOS clone —
  `Permission denied`, exit 126. Hidden by two things at once: Windows does not
  model the executable bit, so the authoring platform could not detect it, and
  every automated caller used `bash <script>`, which works either way. So the
  only broken invocation form was the one only humans use, and no gate used it.
  Found by the new guards suite, which runs the script the way the docs say to.
  Fixed by mode rather than by changing the caller: `bash "$SCRIPT"` would have
  gone green while leaving the documented command broken. The suite now asserts
  the **git index** mode, since a filesystem `-x` test is vacuous on Windows.
- Read-side TeX exposure (`openin_any`) and third-party action SHA pinning are
  tracked separately rather than guessed at here — see the linked issues. An
  unverified control is worse than an acknowledged gap.

### Added - Python uv BUILD-front idempotence audit

- Added a versioned specification, deterministic JSON/Markdown reporter, and
  repository regression for Python package fronts that recreate `.venv`
  without `--clear` or a compatible interpreter pin.
- Classified the exact 17-front corpus without modifying package recipes, and
  decomposed it into nine dependency-shaped follow-up owners with complete,
  machine-checked coverage.

### Added — HL-C136: pre-A1 lexicon wave I, "Pointing, and Asking"
- **42 lessons and 6 new chapters**, one in each Indic track — the first tranche
  of the drive order, and the first time all six move together on vocabulary.
- Six words: **this, that, here, there, who, where.** The reader could already
  name things and could not point at them; now everything already in the book
  becomes a sentence they can use. Chosen on HL10 §9's order — function first,
  frequency second, cognate leverage third — which these win on all three.
- A seventh lesson per track carries the reason they are one chapter. All four
  Dravidian languages build near, far and question from the same three vowels
  with nothing else moving — ಇಲ್ಲಿ / ಅಲ್ಲಿ / ಎಲ್ಲಿ. Hindi runs the same machine on
  य/व/क and Sanskrit on अ/त/क, which is a rhyme rather than a match, and the
  page says so rather than flattening it.
- **The whole wave is `voice`.** `pen` and `sight` do not move at all: not one of
  the 42 needs eyes. All six new chapters are fully drivable end to end —
  `fullyDrivableChapters` 504 → 510, `drivablePrefixTotal` 1175 → 1217 — and
  `drivablePercent` **holds at 67** rather than falling. That is the drive
  order's own prediction: 42 eyes-free lessons enter numerator and denominator
  together, so a meaning ramp costs the driving edition nothing; only a decoding
  ramp moves this number. (An earlier draft of this entry claimed 66 → 67. That
  was true against the base this wave was authored on and stopped being true
  while it sat behind 152 commits: main reached 67 first, on HL-C128 step 4. The
  claim is corrected rather than kept, because the number is a measurement.)
- Headwords are **authored, not cited** — this repo has no dictionary, and the
  corpus's other 400-odd word lessons stand the same way. What is checked
  mechanically: every character is a real letter *of the right script* (a Telugu
  glyph in a Kannada word would render and be silently wrong), the romanization
  is plausible against the headword, and nothing duplicates a word the track
  already teaches. All 36 pass.


### Added — HL12 payment two: Hindi joins, and Devanagari's citations get used
- **8 Hindi script segments** in chapters 6-13. Hindi had eleven writing lessons
  and **not one reached the page** — all eleven sit in the handwritten chapters
  1-5 and rendered only in the answer key, while chapter 1's prose promised
  *"Each lesson introduces the letters its word needs."* These are the first
  Hindi script lessons a reader of the book actually meets.
- **अ and आ now carry a cited stroke order** in both Hindi and Sanskrit: the
  numbered pen path, the pen-lift count, the source and its variation note, all
  read from `devanagari.json`. The first pass shipped them as tracing because it
  never looked for a citation. Nine of Devanagari's 28 letters are cited; the
  rest still trace and still say why.
- Every Devanagari vowel-sign segment shows the script file's worked example —
  **न + ◌ा = ना** *nā* — and every letter its component breakdown. Devanagari
  records these and the three Dravidian files do not, so their segments print
  what their script has rather than the least common denominator.
- Example words are now chosen so the reader can **say** them: a headword with no
  romanization is script they are only now learning to decode. Malayalam's first
  segment page had two of four bullets in exactly that state.
- Non-words are no longer offered as examples. A lesson whose headword is the
  character itself (Hindi's inherent-vowel lesson, headword अ) or a list of marks
  (`ा, े`) is not a word to find a character inside.
- The eleven existing `HI-W*` lessons now declare `delivery: script`, which they
  always were — adopting the marker made the *"script strand is declared, not
  inferred"* check apply to Hindi and it found all eleven at once.


### Added - HL12: the Indic tracks, pre-A1 to C2
- `code/specs/HL12-indic-pre-a1-to-c2.md` is HL10's counterpart for Tamil,
  Telugu, Kannada, Malayalam, Hindi and Sanskrit - the ladder Spanish already
  has, for six tracks whose reader must also be taught the script.
- Measured starting position: every one of the six **points at A2 and has
  attained nothing**, not even pre-A1, with 53-86 words against pre-A1's ~300.
  The ladder has not been climbed at all, and the first rung is four times taller
  than what exists.
- **The governing idea is the owner's**: decoding becomes second nature, and then
  meaning becomes the problem. Those are two ramps that behave differently - the
  script one is finite and *ends*; the meaning one is the whole climb to C2.
- The rule that follows: **a lesson may sit at the frontier of decoding or of
  meaning, never both.** When a reader stumbles on a lesson that is new in both,
  they cannot tell which one they failed, and neither can the curriculum - and
  those need opposite remedies. **59 of the six tracks' 577 lessons** ask for
  both at once today.
- The decoding ladder must **name the lesson where it closes**, after which the
  script is never a topic again - otherwise a learner who reads fluently and
  understands little concludes they are bad at the script, rather than
  recognising the expected halfway house.
- **Romanization is scaffolding with a scheduled removal**: on every headword at
  pre-A1, first use only at A1, absent from A2. HL11's exposure exemption is a
  pre-A1 device, and as it is withdrawn closure stops being report-only and
  becomes a gate. The scaffolding comes down as the structure takes the load.
- Records two owner directives: the handwritten chapters are unpublished drafts
  and may be rewritten, and **page count is never a constraint** - no rule may be
  relaxed and no lessons merged to keep a book short. `HI-W01`, which teaches
  twelve Devanagari glyphs at once, is what economising looks like.

### Added — HL12: the four silent tracks start teaching their script
- **30 recognition segments** — Telugu, Kannada and Malayalam 8 each, Sanskrit 6 —
  one character per lesson. Before this, four of the six Indic tracks taught **no
  letter at all** while printing every word in a script the reader had no way in
  to. That is the gap HL12 §1 measured and this closes the first part of it.
- Each segment names one character, says what it carries, and shows it inside
  four words the reader **already says**. So it sits at the frontier of decoding
  and never at the frontier of meaning — HL12 §2.1's rule, satisfied by
  construction rather than by inspection.
- **Recognition, not writing, and that is a sourcing fact.** These three scripts
  have zero cited stroke orders (their own script files say *"Recognition
  only"*), and HL11 §5 forbids a pen path without a citation: a learner cannot
  tell an invented stroke order from an attested one and will drill it for years.
  The reader traces the printed shape — which needs no source — and the book says
  plainly that where to start and which way to travel are not written down yet.
- **Placement was measured, not chosen by taste.** Second-in-chapter cost 11
  lessons from the drivable prefixes; last-in-chapter cost **none** —
  `drivablePrefixTotal` is unchanged at 1136 and `unstartableChapters` holds at
  173. Last is also better teaching: the character arrives after every word in
  the chapter that contains it.
- The generator refuses a chapter that cannot afford one more atom. Sanskrit's
  chapters 6 and 7 sit at 15 and 12 against HL08's budget of 12, so they get no
  segment and Sanskrit takes six rather than eight. A chapter that cannot afford
  a letter does not get one.

### Fixed
- `human-language-data`: a dotted circle carrying a combining mark now joins that
  mark's script run in the generated LaTeX. U+25CC is `Script_Extensions=Common`,
  so it was handed to the Latin body font, which has no such glyph — the first
  build of these segments logged **184 "Missing character" warnings** and left a
  hole exactly where the character being taught should have been.


### Added — HL11: Tamil's Script Drizzle, One Letter at a Time
- Nine new Tamil lessons, each teaching **exactly one letter**: வ, ண, ன, ந, ற,
  க, ம, the puḷḷi, and the i-sign. Every one carries the letter's components,
  its full pen path, its pen-lift count and its citation — all read from
  `data/scripts/tamil.json`, none of it asserted by hand.
- This is the thing HL11 was written for and the corpus did not have. Tamil had
  a writing strand of 24 lessons, but every one of them is word-shaped — *write
  வணக்கம்*, *read peyar* — putting four to sixteen letters in front of the
  reader at once, with the first at sequence 270. A strand, and no drizzle.
- Each segment sits immediately **before** the word-writing lesson that uses its
  letter, so the reader meets a letter alone and then meets it inside a word.
  Tamil closure violations fall 50 → 42.
- **The drizzle costs the driving edition nothing.** `drivableLessons`,
  `drivablePercent`, `drivablePrefixTotal` and `fullyDrivableChapters` are all
  unchanged: no existing lesson became undrivable and no chapter lost a lesson
  from the prefix a commuter can do. That is HL11's own falsification test.
- It only passes because of where they sit. An earlier revision placed them in
  chapters 1–3, where the payoff is soonest, and two things broke: those
  chapters are **handwritten and protected from generation**, so the segments
  existed in the corpus and never reached the page; and one landed as chapter
  3's first lesson, leaving that chapter impossible to begin in the car —
  caught by `unstartableChapters` moving 173 → 174.
- First use of HL-C41's block-level modality by real content:
  `lessonsWithWritingSegments` moves 0 → 9 after existing only in fixtures.
- The book was rebuilt: 259 pages, exit 0, **zero** overfull, underfull or
  missing-character warnings. Fixed one page defect found by reading the PDF —
  the renderer has no Markdown ordered-list conversion, so numbered stroke steps
  collapsed into a run-on paragraph, which for a pen path is not cosmetic.

### Fixed — HL11: five tracks did not know what order they were in
- Hindi, Telugu, Kannada, Malayalam and Sanskrit each carried ~30 lessons with
  **no `sequence` at all** — every word lesson of their first five chapters. So
  the corpus believed their words came *after* their script lessons, which is
  the opposite of what their books print, and the only reason the books read
  correctly is that those chapters are hand-typed LaTeX.
- HL09 §4 has required `sequence` since it was written, and everything measured
  since — closure, continuity, the ramp windows, the drivable prefix, the app's
  reading order and the narration export — is a claim about order. For these
  five tracks those claims were being made against an order nobody had declared.
- The order is **recovered, not invented**: each book's own
  `\label{lesson:...}` sequence is the only place it was ever written down.
  Corpus lessons without a declared order: **477 → 322**.
- The knock-on numbers are the point. Forward prerequisites **225 → 143** and
  forward reviews **267 → 168**: most were artifacts of an undeclared order, not
  real gaps in the ramp. Tracks with unordered lessons **17 → 12**.
- Hindi also loses the title of steepest lesson in the corpus. `HI-W01` still
  shows twelve glyphs at once, but Hindi's *words* now come before it, so it is
  no longer the first place those glyphs appear. Marathi inherits the record
  with the same twelve — and Marathi still has no declared order, which is why.
- One matching bug caught before it landed: `HI-W04-ra-sa-mera-naam` ends with
  "naam" and was matched to the lesson `naam`, which jumped it ahead of its own
  prerequisite. Labels are now compared against a lesson id's whole tail.
- All five books rebuilt: exit 0, **zero** warnings — 183, 169, 167, 192 and 100
  pages.

### Added — HL11: 184 Words a Reader Can Now Say
- Paying down the closure debt. A lesson's `romanization` is what lets a reader
  use a word before they can read it — HL11 calls a headword shown beside one
  *exposure*, and it is the whole mechanism by which a book about an unfamiliar
  script is useful from page one. **489 headwords had none.** 184 now do, across
  all six Indic tracks; corpus closure violations fall 932 → 873.
- **These are recovered, not derived, and the difference is the point.** A
  mechanical ISO-15919 transliteration agreed with only **71%** of the 195
  romanizations these tracks' authors had already written by hand — and every
  disagreement was the machine being faithful to the spelling and wrong about
  the sound. Tamil was worst at 61%: it writes one letter for each of k/g/h,
  c/s, t/d and p/b, so transliteration says *cāppiṭu, paṭi, cukam, pēcu* where
  the words are said *sāppiḍu, paḍi, sugam, pēsu*. Publishing 344 derivations
  would have published 344 confident mispronunciations into a field the book,
  the app and the narration export all read aloud.
- So each one is recovered from what its lesson already tells the reader in
  prose — *"Say it va-ṇak-kam"* — a human's judgement, already reviewed and
  already shipped in the book, moved into the field that consumes it.
- A wrong grab is caught by a **skeleton** check: the word with every
  distinction the script does not record folded away, compared against the
  headword's own transliteration. Where nothing matches, the tool recovers
  nothing and says so. **160 headwords still need a human**, which is the
  correct output rather than a gap.
- `data/scripts/recover_romanization.py` carries the method, the measured
  per-script agreement rates, and the reason derivation was rejected.

### Added — HL11 Letter Ledgers: what order to meet the letters
- `data/scripts/<script>-ledger.json` records, for Tamil, Telugu, Kannada,
  Malayalam and Devanagari, the order a reader meets the letters — ordered by
  the words each one makes writable rather than by the traditional recitation
  order, which front-loads independent vowels that unlock almost nothing.
- Measured over the six Indic tracks' opening lessons: this reaches **Tamil's
  "thank you" at the tenth glyph and its greeting at the eleventh**, and
  Devanagari's नमस्ते at the twelfth. The same walk in recitation order completes
  **zero** words after twelve glyphs. Roughly a third of each track's opening
  vocabulary is writable within 24 positions.
- `propose_letter_ledger.py` computes a candidate order and shows its work.
  Families are extracted mechanically wherever a script file's `components`
  already name another letter ("ध: like द with an extra inner loop"); a family
  stated only in prose carries the sentence that justifies it. Not one
  target-script character is typed into the generator — every glyph is looked up
  by its official Unicode name, so a maintainer who cannot read the script can
  audit each line.
- `validateLetterLedger` checks a committed ledger against the corpus and never
  rewrites it. The check that earns its keep is that every claimed unlock names
  a lesson that exists: a ledger and a curriculum drift apart silently, and the
  ledger keeps asserting a payoff long after the lesson delivering it was
  renamed.
- Two rules the payoff ordering may not override: a vowel sign cannot precede
  the first base letter, because in an abugida a mark modifies a letter and a
  ledger opening on one describes a lesson that cannot be written down; and
  letters that share a shape are taught together.
- `chapter-policy.json` gains the drizzle itself — one new letter per script
  segment, at least two lessons between segments, and the unspent-letter window.

### Added — HL11 Script Closure: were the letters ever taught?
- `measureScriptClosure` asks what HL08's glyph budget cannot. That budget caps
  how FAST new glyphs arrive; a track satisfies it perfectly while teaching no
  letters at all, and most non-Latin tracks do exactly that.
- First measurement, now in the gap report: **932 lessons across 16 non-Latin
  tracks ask the reader to decode a glyph nobody taught them**, and **12 of those
  16 tracks teach no letters at all**. The pace budget flags 61 lessons. The gap
  between 61 and 931 is the argument for the measurement.
- The defect is not confined to the six Indic tracks it was written for: Arabic,
  Bengali, Marathi, Russian, Punjabi, Gujarati, Persian, Urdu, Japanese and
  Chinese show it too.
- Exposure keeps the rule honest and is drawn mechanically: a headword is
  exposure when its lesson declares a `romanization`, because that is the promise
  the reader can use the word without reading it. **489** native-script headwords
  carry none. Each becomes exempt the moment somebody writes down how to say it —
  the rule names its own remediation, which is why it is the right one.
- Two numbers watch the exemption rather than one. 49 lessons are clean *because
  of* it; **1,997 glyphs** were removed by it, counting the ones it shaved off
  lessons that violate anyway. The lesson count alone cannot see a lesson
  reporting five untaught glyphs while fifteen more were exempted, and the glyph
  count is what would move if an author started laundering script through the
  headword once 932 becomes a burn-down target.

### Added — HL11: The Drizzled Script Ramp
- `code/specs/HL11-drizzled-script-ramp.md` specifies the curriculum ramp for a
  reader who does not already know the target alphabet. The book stays useful from
  page 1 — greetings carried by romanization — while the script drizzles in one
  letter at a time behind it.
- The governing rule is closure on *load-bearing* script only: a lesson may ask the
  reader to decode or produce target-script text only when every glyph in it has
  been taught, while script the reader is merely shown is exposure and is counted,
  reported, and never required.
- Measured over the six Indic tracks: seven or eight glyphs unlock five real words
  in every one of them, when letters are ordered by the words they complete rather
  than by recitation order — which completes zero words after twelve glyphs.
- Records the sourcing rule for handwriting figures: the pen path's shape is
  verified against the shipped font, its order must be cited, and no citation means
  no pen path and no figure, with the gap reported as debt rather than invented.

### Fixed — TypeScript ZIP Reads Real-World DEFLATE
- `zip`'s inflater rejected dynamic Huffman blocks (BTYPE=10) outright, which is
  what zlib and Info-ZIP emit for anything but the smallest input — so the reader
  failed on most archives the world actually produces. It now decodes all three
  RFC 1951 block types via a canonical Huffman table builder, the code-length
  alphabet with its run-length escapes, and the permuted code-length order.
- Length symbol 285 was missing from the length table. RFC 1951 spells length 258
  either as symbol 284 with five extra bits or as symbol 285 with none, and a run
  of identical bytes reliably produces the cheaper form, which was rejected as an
  invalid symbol.
- Decoder conformance is now checked against Node's `zlib` as an oracle, because
  round-tripping our own encoder through our own decoder only proves the two
  agree with each other.
- Huffman tables are now checked against Kraft's inequality. Over-subscribed
  tables are rejected outright and incomplete tables everywhere RFC 1951 forbids
  them, so the decoder no longer accepts streams zlib refuses -- a difference
  between two readers of the same bytes is the shape of a content-inspection
  bypass.
- The inflate output cap now counts bytes. It previously counted elements of a
  `number[]`, where V8 spends four to eight bytes each, so a 256 MB ceiling
  allowed one to two gigabytes of backing store and the process died before the
  limit was reached. `rawInflate` takes a caller-supplied ceiling, and
  `ZipReader.read` passes the SMALLER of the entry's declared uncompressed size
  and the reader's own — the declared size is four bytes the archive chose, so
  trusting it alone would swap a fixed limit for an attacker-chosen one.

### Added — IC18: PNG Encoder and Decoder
- `image-codec-png` turns a `PixelContainer` into a real `.png` and back:
  chunk framing with CRC-32, the RFC 1950 zlib wrapper with Adler-32, and all
  five RFC 2083 scanline filters with per-row selection by the PNG spec's own
  minimum-sum-of-signed-bytes heuristic.
- The hard layer is not in the package. RFC 1951 DEFLATE and PNG's CRC-32 both
  come from `zip`, because the bit stream inside `IDAT` is the one inside a ZIP
  entry and the polynomial is the same. A second copy would be a second place
  for the same class of bug to hide.
- Encodes 8-bit truecolour with alpha, which is exactly what a `PixelContainer`
  holds, so the round trip is lossless by construction. Decodes 8-bit colour
  types 0, 2, 4 and 6, any number of `IDAT` chunks, skipping unknown ancillary
  chunks and refusing unknown critical ones as the spec requires. Palette
  images, 16-bit depths and Adam7 interlacing are refused by name.
- Conformance is tested against foreign implementations, not just against
  itself: Node's zlib inflates our `IDAT`, and the decoder reads PNGs assembled
  by hand from RFC 2083. The output was confirmed readable by `file`, macOS
  `sips` and Python's `zlib`.
- Refuses four shapes of file that decode to exactly the right image while
  carrying bytes the image does not need: a payload inside `IEND`, anything
  after `IEND`, a chunk wedged between two `IDAT`s, and the `IDAT` cavity —
  the dead space between where the DEFLATE stream announces its own end and
  where the Adler-32 begins. The picture is identical either way, which is
  exactly why tolerating them makes a valid-looking PNG into free carriage.
- Caps the total pixel count, not just each edge: 16384 x 16384 passes an edge
  cap and is 268 million pixels, roughly 3 GiB of peak allocation for about a
  megabyte of input. BMP survives on an edge cap because its pixels have to be
  in the file; PNG amplifies, and that is the whole difference.
- `zip` gains `rawInflateCounted`, which reports how many input bytes a stream
  actually used. Without it the `IDAT` cavity is undetectable.
- Adds `IC18-image-codec-png.md`. PNG had fallen out of the IC series
  entirely: IC00's roadmap reserved IC04 for it, that number went to JPEG, and
  the table was never corrected — while IC08 (ICO) still names PNG as a
  dependency for its 256x256 frames. IC00's roadmap now matches the specs that
  exist.

### Added — Raw RFC 1951 DEFLATE, Exported
- `zip` now exports `rawDeflate` / `rawInflate`: the DEFLATE codec with no ZIP
  framing. The same bit stream sits inside `zlib`, `gzip`, and PNG's `IDAT`, so
  exporting it keeps those formats from each carrying a second copy of the same
  bit-packing code. The encoder is unchanged and byte-stable; only the reader grew.
- CMP09 records the export as optional-per-port, and states the asymmetry as a
  rule: an encoder may emit fixed blocks only, but a decoder must read all three.

### Added — Chief Host Authenticated Data Plane
- The existing per-spawn secure host session now carries bounded, serialized
  channel receive/publish/acknowledge and provider-neutral text completion
  exchanges with monotonic request IDs and exact response correlation.
- The process supervisor exposes real-pipe child exchange helpers and retains
  authenticated pending requests for injected daemon service adapters, closing
  the missing data-plane seam before the production Chief host composition.

### Fixed — Ruby Canonical Starlark BUILD Compatibility
- Ruby's Starlark stack now closes indented files in the specified token order,
  preserves `r`/`b`-leading identifiers, binds mixed keyword calls, and keeps
  defining-module globals across nested loads.
- The Ruby build tool injects the normalized v1 evaluation context, validates
  structured commands, and fails closed after Starlark classification instead
  of silently falling back to raw shell lines; evaluation errors redact the
  checkout root.

### Fixed — Venture Windows CI Acceptance
- The pull-request Windows runner now derives a dedicated Venture acceptance
  flag from the shared build plan, installs MSVC and .NET only when that slice
  is affected, and executes the package-owned Rust/WinUI integration test
  instead of reporting a green job whose general build step was skipped. A
  focused detector test ratchets force, package, unrelated, and malformed plans.

### Added — OCaml CI Toolchain Evidence
- Added OCAML03, a closed evidence manifest, and digest-checked transitive opam
  solver locks plus installed-package receipts for Ubuntu x64, macOS arm64, and
  Windows x64.
- Added a commit-pinned, read-only three-platform workflow that fresh-solves
  against one reviewed opam-repository commit, compares checked evidence, then
  performs a separate locked install.
- Added real line-oriented execution of the library and program scaffold
  `BUILD`/`BUILD_windows` contracts, including formatting, Alcotest, and
  measured `bisect_ppx` coverage on every runner family.
- Added an offline validator and CI unit tests for closed keys, action/repository
  identities, safe evidence paths, digests, direct versions, workflow security,
  and platform command-shell dispatch.

### Added — OCaml Scaffold Infrastructure
- Added the OCAML02 contract, lane README, repository ignores, and shared
  byte-exact library/program fixture trees for the emerging OCaml lane.
- Added matching Go and TypeScript scaffold-generator support with exact direct
  OCaml/opam/Dune/Alcotest/`bisect_ppx`/`ocamlformat` metadata, resolved local
  dependency pins, real formatting/test/coverage build commands, and
  schema-valid capability profiles.
- Added OCaml-specific metadata serialization and `*)` injection hardening
  before any scaffold directory is written.
- Rejected Dune `%{...}` interpolation openers before output generation.
- Added a shared OCaml/opam string encoding contract so accepted Unicode remains
  byte-identical raw UTF-8 across the Go and TypeScript front doors.
- Added dependency-reader regressions for self-name/metadata decoys,
  program-to-library pin paths, and direct/transitive symlink rejection.

### Added — OCaml Emerging-Lane Contract
- Added OCAML01 to define OCaml as a known emerging implementation lane that
  remains outside the established 15-language parity denominator until its
  package, build, security, documentation, and three-platform promotion gates
  pass.
- Made the package-parity reporter derive its high-consensus completion-band
  upper bound and Markdown missing-slot heading from the established-language
  count instead of embedding `15`.
- Advanced the package-parity JSON output to schema version 3 for its explicit
  denominator and ordered completion-band metadata, and documented the CSV
  presence matrix as header-addressed when emerging buckets add columns.
- Added reporter conformance coverage proving OCaml packages are inventoried
  without creating unknown buckets, established identities, completion-band
  entries, or missing slots.

### Added — Learning Coverage Backfill
- Added a generated inventory that maps all 1,155 package concepts to dedicated,
  related, index-only, or missing learning material and prioritizes the backlog
  by cross-language implementation breadth.
- Added the first backfill lessons for tree and probabilistic data structures,
  dictionary and entropy compression, cryptographic composition, and
  intermediate representations.
- Added a tested `code/scripts/learning_coverage_report.py` command so the inventory
  can be regenerated as packages and learning material evolve.

### Added — SPICE Berkeley Mosaic App Startup Summary
- `spice-netlist-parser` now exposes Berkeley Mosaic app startup summaries plus
  JSON helpers. The summary derives a compact ready/blocked route from the
  bootstrap payload, including package name, source fingerprint, repaired
  editor-state IDs, stale-state flags, active panel, diagnostic count, and
  blocking reason.
- The summary helpers reuse the run and non-run bootstrap paths so product
  shells can make startup routing decisions without walking the full host
  panel payload or duplicating simulator internals.

### Added — SPICE Berkeley Mosaic App Bootstrap Snapshot
- `spice-netlist-parser` now exposes schema-versioned Berkeley Mosaic app
  bootstrap snapshots plus JSON helpers. The bootstrap payload combines the
  static package manifest with the deck-specific host-surface wire export so
  WebAssembly and product shells can load package capabilities, repaired
  editor-state metadata, active panels, diagnostics, and run availability from
  one startup envelope.
- The run and non-run helpers preserve the same blocked-deck diagnostic surface
  as host-wire exports while keeping the package manifest stable and derived
  from the Rust app facade contract.

### Added — SPICE Berkeley Mosaic App Package Manifest
- `spice-netlist-parser` now exposes a schema-versioned Berkeley Mosaic app
  package manifest plus JSON helper for WebAssembly and product-shell
  packaging. The manifest advertises the Berkeley grammar version,
  host-surface wire schema, source-fingerprint algorithm, panel kinds, editor
  action kinds, command targets, runnable analysis directives, and artifact
  capabilities before a host opens a deck.
- The manifest keeps packaging metadata derived from the same Rust app facade
  contract as host surfaces and host-wire exports, avoiding a separate product
  registry while the public parser contract remains language-aligned.

### Added — SPICE Berkeley Mosaic Host Wire Export
- `spice-netlist-parser` now exposes schema-versioned Berkeley app host-surface
  wire snapshots for Mosaic packaging and WebAssembly embedding.
  `host_surface_wire()`, `run_host_surface_wire()`, and their JSON helpers
  flatten the host panel contract into stable lower-case panel kinds,
  diagnostics, active-panel IDs, and repaired persisted editor-state metadata.
- The JSON helpers avoid exposing simulator internals to product shells while
  preserving the Rust app substrate over the public Berkeley parser contract.

### Added — SPICE Berkeley Mosaic Host Surface
- `spice-netlist-parser` now exposes Berkeley app-deck host surfaces for Mosaic
  shell integration. `host_surface()` and `run_host_surface()` derive stable
  source, diagnostics, analysis, table, and waveform panel descriptors from
  persisted editor state, including panel IDs, target names, enabled states,
  active state, and disabled reasons.
- The surface stays Rust-only app substrate over the public Berkeley parser
  contract, so Python and TypeScript remain aligned when parser behavior
  changes while Mosaic hosts can wire panels without reinterpreting simulator
  internals.

### Added — Twig LANG-FULL E4 Multi-Parameter String Evidence
- `twig-ir-compiler` 0.42.0 now proves one conservative direct call can infer
  multiple otherwise-unannotated string parameters at once. `(define (same a b)
  (if (string=? a b) 42 0)) (same "OK" (string-append "O" "K"))` lowers the
  function body through typed E4 `str_eq` without synthesizing refinement
  annotations.
- `lang-aot` adds the multi-parameter string-equality proof across native-AOT,
  LLVM, WASM, JVM, CLR, VM, and JIT.

### Added — Twig LANG-FULL E4 Static String Expression Parameter Evidence
- `twig-ir-compiler` 0.41.0 now proves conservative direct-call evidence for
  otherwise-unannotated string parameters can come from static string expression
  actuals, not only literals or named/lexical string values. `(define (strlen x)
  (string-length x)) (strlen (substring (string-append "HE" "LLO!") 0 5))`
  runs through typed E4 `str_concat` + `str_slice` + `str_len` without
  synthesizing refinement annotations.
- `lang-aot` adds the static-expression-actual proof across native-AOT, LLVM,
  WASM, JVM, CLR, VM, and JIT.

### Added — Twig LANG-FULL E4 Derived Let Star String Parameter Evidence
- `twig-ir-compiler` 0.40.0 now proves sequential lexical `let*` string actuals
  derived from earlier string locals can seed conservative direct-call evidence
  for otherwise-unannotated string parameters. `(define (strlen x)
  (string-length x)) (let* ((a "HE") (b (string-append a "LLO"))) (strlen b))`
  stays on the typed E4 `str_concat` + `str_len` path without synthesizing
  refinement annotations.
- `lang-aot` adds the derived `let*` lexical-actual proof across native-AOT,
  LLVM, WASM, JVM, CLR, VM, and JIT.

### Added — Twig LANG-FULL E4 Lexical String Parameter Evidence
- `twig-ir-compiler` 0.39.0 now lets conservative `main`-level direct-call
  evidence for otherwise-unannotated string parameters use lexical `let`/`let*`
  string actuals, so `(define (strlen x) (string-length x)) (let ((s "HELLO"))
  (strlen s))` runs through the typed E4 `str_len` path.
- Lexical evidence is scoped: dynamic shadows and non-string local bindings
  still block inference and remain on the dynamic path without synthesizing
  refinement annotations.
- `lang-aot` adds the lexical-actual proof across native-AOT, LLVM, WASM, JVM,
  CLR, VM, and JIT.

### Added — Twig LANG-FULL E4 Named String Parameter Evidence
- `twig-ir-compiler` 0.38.0 now lets conservative `main`-level direct-call
  evidence for otherwise-unannotated string parameters use non-escaping
  top-level string value actuals, so `(define s "HELLO") (define (strlen x)
  (string-length x)) (strlen s)` runs through the typed E4 `str_len` path.
- The inference pass stays source-order and escape-analysis aware: captured,
  shadowed, conflicting, unobserved, and closure-derived values remain on the
  dynamic path and do not synthesize refinement annotations.
- `lang-aot` adds the named-actual proof across native-AOT, LLVM, WASM, JVM,
  CLR, VM, and JIT.

### Added — HTML Parser Formatting Adoption
- `</b>` adoption across `<aside>` now preserves the html5lib `<em><foo><foo>`
  continuation during tree construction, retiring the old finish-time
  `<em>/<aside>` post-parse repair.

### Added — HTML Parser Browser Script Storage Access
- Browser-readiness summaries now expose script storage-access descriptors for
  inline references to Web Storage, cookies, IndexedDB, CacheStorage/service
  workers, StorageManager, storage-event hooks, and fallback blockers such as
  `nomodule`.

### Added — HTML Parser DOCTYPE Fragment Contexts
- Parser-approved initial tokenizer contexts now include seeded DOCTYPE
  continuation states for keyword, name, public/system identifier, bogus, and
  force-quirks recovery paths.
- DOM parser coverage now exercises parser/lexer handoff for partial DOCTYPE
  fragments while preserving lexer diagnostics and following body content.

### Added — SQL Auto-Index: Composite Multi-Column Index (IX-8)
- **`IndexScan.columns: tuple[str, ...]`** in `sql-planner` — replaces
  `column: str`; single-column scans produce a 1-tuple, composite scans an
  n-tuple matching the leading prefix of the index used.
- **Multi-column bounds** — `IndexScan.lo` / `IndexScan.hi` widened to
  `tuple[object, ...] | None`; `OpenIndexScan` in `sql-codegen` and the VM
  decode them with `list(ins.lo)` for prefix-key comparison in the backend.
- **`_extract_multi_column_bounds`** planner helper — chains `_extract_index_bounds`
  across consecutive index columns; EQ extends the chain, range terminates it.
- **Best-match index selection** — `_try_index_scan` evaluates all indexes and
  picks the one covering the most predicate columns.
- **`IndexAdvisor` pair tracking** — `_pair_hits` accumulates `(table, col_a, col_b)`
  pairs from full-table scans; `_maybe_create_composite_index` creates a
  two-column index when the policy threshold is reached, skipping redundant
  creation when a leading-column single index already exists.
- **`_auto_index_meta`** — maps auto-index names → `(table, columns_tuple)` for
  correct drop-loop bookkeeping without name parsing.
- **21 new tests** in `mini-sqlite/tests/test_tier3_composite.py` covering
  advisor pair logic, planner composite selection, and end-to-end integration.

### Added — ALGOL 60 WASM Pipeline
- Advanced PL04 Phase 5 call-by-name lowering with integer array-element
  eval/store thunk descriptors, including repeated re-location of subscripted
  actuals on formal reads and assignments.
- Enabled read-only ALGOL expression thunks to read arrays, covering
  Jensen's-device terms such as `a[i] * i` through the WASM runtime path.
- Enabled read-only ALGOL expression thunks to call integer procedures, with
  nested procedure failures propagated through thunk helper state.
- Added Phase 5 wrap-up coverage and docs for the completed integer by-name
  subset and its remaining full-ALGOL exclusions.
- Added PL04 Phase 6 direct local labels and `goto` support through the
  ALGOL type-checker, IR compiler, and WASM compiler path, with guards for
  nonlocal and Phase 7 designational forms.
- Added PL04 Phase 7a local switch declarations, switch selections, and
  conditional designational `goto` support through type-checking, IR lowering,
  and WASM execution, while keeping nonlocal frame unwinding guarded.
- Added PL04 Phase 7b direct nonlocal block `goto` support with frame/heap
  unwinding inside one lowered function, while keeping procedure-crossing
  jumps and nonlocal designational forms guarded.

### Added — TypeScript Port + JavaScript/TypeScript Grammars (PR #14)
- **31 TypeScript packages** — complete port of the computing stack to TypeScript
- `javascript.tokens` + `javascript.grammar` — JavaScript grammar definitions
- `typescript.tokens` + `typescript.grammar` — TypeScript grammar definitions
- Cross-language packages: `javascript-lexer`, `javascript-parser`, `typescript-lexer`, `typescript-parser` in Python, Ruby, and Go
- D05 Core package (processor integration) in Python, Ruby, Go, and TypeScript
- Extended RISC-V simulator with full RV32I base integer ISA + M-mode privileged extensions

### Changed — Build System: Recursive Discovery + Rust Build Tool (PRs #16, #17)
- **Recursive BUILD file discovery** replaces DIRS-based routing in all build tools (Go, Python, Ruby)
- Build tools now walk the directory tree automatically — no DIRS files needed
- Added skip list for non-source directories (`.git`, `.venv`, `node_modules`, `target`, `.claude`, etc.)
- **Rust added as recognized language** — 6 Rust packages now properly discovered (were "unknown")
- **New: Rust build tool** — complete port with rayon parallelism, SHA256 hashing, git-diff detection
- **All 18 DIRS files removed** from the repository
- Total discovered packages increased from 77 (DIRS-routed) to 126+ (recursive)

### Added — Publish Workflow + Package Completeness (PR #13)
- `.github/workflows/publish.yml` — release publishing for PyPI and RubyGems
- PyPI publishing via OIDC Trusted Publishers (no API tokens)
- Native extension support via maturin: builds wheels on Linux, macOS (arm64 + x86_64), Windows
- Ruby gem publishing via `RUBYGEMS_API_KEY` secret
- Fixed 8 incomplete packages:
  - Go: README + CHANGELOG for assembler, python-lexer, ruby-lexer
  - Ruby: test suites for assembler, html_renderer, jit_compiler shell gems
  - Python: README/CHANGELOG for hello-world and pipeline-visualizer programs

### Added — Go Port (PR #12)
- **25 Go packages** — complete port of the computing stack to Go
- Go implementations of all hardware layers, simulators, lexer, parser, compiler, and VM
- Grammar-driven lexer/parser with cross-language packages (python-lexer, ruby-lexer)

### Added — Deep CPU Internals (PR #11)
- `cache` — L1/L2 cache simulation with LRU eviction in Python, Ruby, Go, and Rust
- `branch-predictor` — 1-bit, 2-bit saturating counter, branch target buffer in Python, Ruby, Go, and Rust
- `hazard-detection` — data, control, and structural hazard detection in Python, Ruby, Go, and Rust
- `clock` — clock generator, divider, multi-phase clock in Python, Ruby, and Go
- `fp-arithmetic` — IEEE 754 floating-point arithmetic in Python, Ruby, and Go
- Deep CPU architecture specs (D00-D05)
- Floating-point arithmetic spec (FP01)

### Added — Accelerator Computing Stack (PR #10)
- GPU/TPU/NPU computing stack specs and overview
- Accelerator architecture documentation (G00)

### Added — Build System (PR #9)
- **Directed graph library** in Python (73 tests, 98%), Ruby (77 tests, 100%), and Go (39 tests, 94%)
- **Build tool** in Go (primary), Python (reference), and Ruby (educational) — incremental, parallel, git-diff-based change detection
- **BUILD files** for all packages — declarative build commands per package
- **GitHub Actions CI** — compiles Go build tool, runs affected packages in parallel
- Go 1.26 added to mise.toml

### Added — Cross-Language Packages (PR #8)
- `ruby-lexer` (Python) — tokenizes Ruby source code via grammar files (42 tests)
- `ruby-parser` (Python) — parses Ruby source code via grammar files (21 tests)
- `python_lexer` (Ruby) — tokenizes Python source code via grammar files (32 tests)
- `python_parser` (Ruby) — parses Python source code via grammar files (15 tests)

### Added — Ruby Computing Stack (PR #7)
- Complete port of all 18 Python packages to Ruby as publishable gems
- Ruby 3.4.9 via mise, Minitest, SimpleCov, Data.define, Standard Ruby
- `ruby.tokens` and `ruby.grammar` grammar definitions
- 662+ Ruby tests, all packages ≥80% coverage (most 95%+)

### Added — JVM + CLR Simulators and Compiler Backends (PR #6)
- `jvm-simulator` — 26 JVM opcodes with real opcode values (81 tests, 97%)
- `clr-simulator` — 24 CLR IL opcodes with real opcode values (93 tests, 100%)
- `JVMCompiler`, `CLRCompiler`, `WASMCompiler` bytecode compiler backends (133 tests, 100%)

### Added — Software Layers Implementation (PR #5)
- `lexer` — hand-written + grammar-driven tokenizer (76 tests, 98%)
- `parser` — recursive descent + grammar-driven parser (54 tests, 99%)
- `virtual-machine` — general-purpose stack-based VM, 20 opcodes (99 tests, 96%)
- `bytecode-compiler` — AST to bytecode compiler (34 tests, 100%)
- `grammar-tools` — reads .tokens/.grammar files with EBNF (66 tests, 97%)
- `pipeline` — end-to-end orchestrator (40 tests, 100%)
- Grammar-driven lexer and parser that work with any language's grammar files
- `python.tokens` and `python.grammar` grammar definitions
- JIT compiler spec and shell package

### Added — Hardware Layers Implementation (PRs #3, #4)
- `logic-gates` — 7 gates + NAND-derived + multi-input variants (89 tests)
- `arithmetic` — half adder, full adder, ripple carry adder, ALU (34 tests)
- `cpu-simulator` — generic fetch-decode-execute cycle (34 tests)
- `arm-simulator` — ARMv7 subset: MOV, ADD, SUB (16 tests)
- `riscv-simulator` — RISC-V RV32I subset: addi, add, sub (14 tests)
- `wasm-simulator` — WebAssembly stack machine (28 tests)
- `intel4004-simulator` — Intel 4004 accumulator machine (21 tests)
- Layer renumbering from top-down (user perspective → hardware)

### Added — HTML Visualizer Design (PR #2)
- Replaced TUI visualizer with pluggable HTML visualizer architecture
- JSON data contract for cross-language pipeline reports
- HTML renderer package scaffold
- Pipeline visualizer program scaffold

### Added — Initial Repository Structure (PR #1)
- Repository scaffolding: CLAUDE.md, README.md, lessons.md, .gitignore
- 9 Python package scaffolds for the computing stack
- Specification documents for all layers (numbered 01-11)
- Python hello world program
- RISC-V simulator package scaffold
- Pipeline orchestrator and stack visualizer scaffolds

## [0.0.0] - 2026-03-18

### Added
- Initial commit with empty repository
