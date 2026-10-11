## Unreleased — a copy lesson shows its filmstrip in its guided practice, and the stage directive is hidden

- The app placed a lesson's stroke-order filmstrip only in a Writing or
  Script section, so the 25 lessons that now print a strip in their guided-
  or delayed-copy practice in the book (the Chinese copy pairs of chapters
  16-19, the Gujarati place words, HI-W01-na-ma) would have shown none.
  `filmstripSectionIndex` in `lessonbody.ts` applies the book's rule:
  Writing, else Script, else the first section whose writing stage is
  observe-trace, guided-copy or delayed-copy. A dictation, a composition or
  a section with no stage never gets one. `MODELLED_WRITING_STAGES` is held
  equal to human-language-data's by a test.
- `lessonSections` now reads `<!-- hl-writing-stage: … -->` into the
  section's `writingStage` instead of printing it. Before, every section
  with a writing stage showed the raw directive as a line of lesson text,
  because only the knowledge and activity directives were filtered.
