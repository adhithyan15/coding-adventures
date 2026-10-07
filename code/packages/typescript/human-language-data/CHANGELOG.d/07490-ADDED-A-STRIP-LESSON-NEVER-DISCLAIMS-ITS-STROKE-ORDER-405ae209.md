### Added — a strip lesson never disclaims its stroke order

- New figure-targets case `a-strip-lesson-never-disclaims-its-stroke-order`.
  Any lesson that prints a filmstrip fails it if its text still carries the
  recognition template's "does not yet tell you where to start" or
  "copy what you see". A control case requires lessons without a strip to keep
  that honest disclaimer, so a wording drift cannot let the check pass
  vacuously. Run against the corpus before this release's prose fix, it named
  the contradicting lessons (BN-W41-bisarga, GU-W45-ai-sign, HI-S01-letter-ma,
  ...). The recognition authoring script can still write the template, because
  it cannot see the filmstrip ledger; this case is what stops it.
