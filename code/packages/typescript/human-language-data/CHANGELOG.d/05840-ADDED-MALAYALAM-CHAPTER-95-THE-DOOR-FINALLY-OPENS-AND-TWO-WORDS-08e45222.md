### Added — Malayalam chapter 95, the door finally opens, and two words buy four forms

- `ML-A1-LEX-51` closes. Malayalam A1 coverage 201/243 -> **202/243 (83%)**, 41
  points unmapped.
- The point's note was right and understated: *"the door is taught and neither
  opening nor closing it is."* **വാതിൽ** has been named since chapter 52 and
  nothing could be done to it or about it.
- All six candidates -- *thurakkuka, adaykkuka, thurakkoo, adaykkoo, akatthu,
  purathu* -- were grepped as **tokens** first and every one had **zero**
  occurrences, so the chapter carries no forward-reference debt. The glyph
  pre-check was run with its control validated first (U+0D7A must report *not*
  covered, U+0D7B covered) and came back clean.
- **Two words buy four forms.** The asking ending is the long **ൂ** already owned
  from **കേൾക്കൂ** among the first commands, and the three tense endings come
  from a verb learned long before -- so *thurakkunnu / thurannu / thurakkum*
  arrive already known.
- `ML-C95-thurakkuka` keeps the object **bare**: വാതിൽ is a thing and not a
  person, so chapter 87's rule holds unchanged while the verb is new.
- `ML-C95-akathu-purathu` gives the label's second half, *inside* and *outside*.
- **A draft claim was cut, and it is the kind nothing in this repo reads.** That
  lesson said അകത്ത് / പുറത്ത് end like the place words for *above* and *below*,
  making one family. **They do not:** മുകളിൽ and മുന്നിൽ end in **-ിൽ**, താഴെ in
  **-െ**, അകത്ത് and പുറത്ത് in **-ത്ത്**. Three endings, not one. Verified
  against the owning lessons rather than from memory.
- The replacement is true and teaches better: അകത്ത്/പുറത്ത് share a **tail** and
  part at the front, while ഇന്ന്/ഇന്നലെ one chapter earlier shared an **opening**
  and parted at the tail. **One end holds still while you listen to the other**,
  and the two chapters now point at each other.
- `validate` reads frontmatter and atom comments; the gates read structure; the
  suite reads banned words, glyph coverage and pins. **None of them reads an
  assertion about how words are built.** Same family as `HL-C399`.
- Also caught by `validate`: **ആണ്** was used in an example without being
  declared. It is owned by `ML-C02-aanu`, so its atoms are now in `requires`,
  the prerequisites and an assessing block rather than being leaned on silently.
