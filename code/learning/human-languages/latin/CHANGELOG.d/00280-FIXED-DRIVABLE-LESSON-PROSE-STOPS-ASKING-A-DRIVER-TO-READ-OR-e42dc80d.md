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

- **Count:** 14 drivable lessons; 19 `[YOU READ: …]` cues.
- Warm-ups LA-C115-theatrum, LA-C116-vicus, LA-C119-miliarium, LA-C124-codex,
  LA-C160-habito ("Read the exchange aloud as one run: *Salvē! …*"), the
  notices LA-C164-inscriptio and LA-C164-venalis, and the reading lessons
  LA-C58-labels, LA-C58-phrases, LA-C58-prima-lectio, LA-C59-a-objects,
  LA-C59-dies-meus, LA-C59-place, LA-C59-social-exchange ("Read once for time:
  …", "Read again for the route: …") become READ cues; "look at where **est**
  falls" inside the cue is "notice where …".
- Left alone: "Read from the endings rather than from position"
  (interpretation), "Sort these three without looking" (a spoken sort), "Trace
  *nox* back" (etymology), the practice heading "… then read the Latin".
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (LA-C164-inscriptio, LA-C164-venalis) keep the notice in
    narrated prose and defer only the look: "[YOU READ: the stone]" then "The
    stone says **SPQR**, which is …". The first pass had put the whole notice
    inside the deferred cue, so a listener heard the comment on a notice without
    the notice; that superseded form is the one described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (LA-C58-phrases,
    LA-C58-prima-lectio, LA-C59-dies-meus, LA-C59-place,
    LA-C59-social-exchange): "[YOU READ: the account once for time — …]", "[YOU
    READ: the dialogue again for the two things handled …]", "[YOU READ:
    **scholam** after a tiny word — **ad scholam**]".

