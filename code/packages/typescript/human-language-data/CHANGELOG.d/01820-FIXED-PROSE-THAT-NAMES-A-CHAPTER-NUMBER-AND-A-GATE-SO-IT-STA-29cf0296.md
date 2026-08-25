### Fixed - prose that names a chapter number, and a gate so it stays fixed (HL-C102)

- Chapter numbers move on every split; lesson ids never do. A sentence like
  "you learned this in chapter 14" is right when written and wrong three
  renumbers later, and **nothing fails** -- the reader just follows a pointer
  into the wrong chapter.
- Three Spanish references were already wrong: `ES-C09-esta-en` sent the reader
  to "Chapter 7" for a question now taught in 24; `ES-C41-explicar` placed
  `contar` in "chapter 38" when it had reached 71; and a comment in `ES-C19-no`
  that was **corrected two PRs ago** had gone stale again, because HL-C101 moved
  the lesson it named.
- All 32 Spanish references now name the thing instead of a number -- "since the
  repair kit", "when you first met them", "the next chapter". Spanish is at zero.
- Adds `tests/chapter-references.test.ts`, which counts **cross-chapter**
  references only: a lesson naming its own chapter (an `# Chapter 2` heading)
  points nowhere else and cannot rot. Spanish is held at zero; the other 19
  tracks are pinned at their current 710 so the debt cannot grow while they are
  stable, and should be cleared before they start splitting chapters.

