## Fixed — drivable lesson prose stops asking a driver to read or handle cards

Narration reads bare prose aloud as written, so a prose instruction to read
printed script, handle cards or cover the page reached a driver unhedged (issue
#12070, tenth pass): "Hear, picture the part, say, and read **おなか**." was
narrated word for word. Each such step now moves into a cue the narration
defers (`[YOU READ: …]`, `[YOU COVER: …]`, `[YOU CHECK: …]`, `[YOU FIND: …]`,
`[YOU WRITE: …]`: "once you have stopped driving — …"), in the authored order,
or is said for the ear and voice where the step was not about the page. Prose
that followed a new cue in the same paragraph now has a paragraph of its own.
The new prose check in human-language-data demands zero such spans in drivable
lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 10 drivable lessons; 13 `[YOU READ: …]` cues and 1 ear-and-voice
  rewrite.
- Warm-ups PT-C112-aula and PT-C113-palavra, notices PT-C137-a-fatura,
  PT-C137-o-anuncio, PT-C137-o-aviso, PT-C137-o-horario, PT-C28-indicador's
  "Read *o 3.º andar* aloud", and the PT-C29 reading lessons (with "Again — …")
  become READ cues.
- PT-C29-linhas "Read the two lines together and the agreement is visible" →
  "Say the two lines together and the agreement is plain".
- Left alone: "Read the panel in a Lisbon lift now — **1.º**, **2.º**, **3.º**
  — and you are reading this chapter" (a real-world scene, not a step).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (PT-C137-a-fatura, PT-C137-o-anuncio, PT-C137-o-aviso,
    PT-C137-o-horario) keep the notice in narrated prose and defer only the
    look: "[YOU READ: the bill]" then "The bill says **…** — …". The first pass
    had put the whole notice inside the deferred cue, so a listener heard the
    comment on a notice without the notice; that superseded form is the one
    described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (PT-C29-linhas,
    PT-C29-primeira-leitura): "[YOU READ: the six lines again, …]", "[YOU READ:
    the passage again, and count the joining words]".

