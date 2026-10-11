---
category: Testing & coverage
---

# A detector that reads text the way the renderer walks it cannot see what the renderer adds from outside that walk

**What went wrong.** The eyes-or-hands detectors in
`human-language-data/src/drivable-instructions.ts` read each section through
`narratedParts`, a copy of the narrator's own paragraph walk, precisely so
they would judge what the narration says. That walk drops HTML comments and
the classifier skips headings. But the narrator does not only speak the walk:
it compiles `<!-- hl-activity: … -->` comments and speaks their prompts as
questions, and it speaks every section title. A security review found 235
scored prompts ("Write son in Chinese characters.") and 95 headings ("Guided
Practice — read, copy, recall") spoken to drivers, in lessons the corpus gate
had just proved clean. The gate was green because it asserted exactly what the
walk could see.

**Fix.** The narration now judges what it adds itself: a prompt that needs eyes
or hands is deferred (`spoken: false`), a typed prompt is said "Say …", a
drivable lesson's heading is said as its label, and the stop guard precedes a
set-aside title. The new gate reads the COMMITTED narration output (JSON plus
the renderer's opening line), not the source walk.

**Do differently.** When a check mirrors a renderer's input walk, list every
place the renderer reads from (comments, front matter, headings, compiled
contracts) and ask which of them the walk drops. A gate on what is said should
read the rendered output, or at least every input the renderer consumes.
