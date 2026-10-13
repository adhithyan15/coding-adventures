# Native writers' pen traces as a source (Bengali)

Most owners cite a teaching animation or a tracing guide. Bengali
(`strokes/bengali.ts`) cites something different: the tablet pen traces of
native Bengali writers that HP Labs India's LipiTk 4.0 Bangla recognizer keeps
as its prototypes (MIT licence). A trace records where the pen went down, which
way it moved and where it lifted, so counting traces gives the modal stroke
count, the part order, the start and the turning direction, each as a share of
the writers. It does not give proportions: the recognizer scales every trace to
a square. So the counts go into the citation's `variation`, no coordinate is
copied, and the path itself is fitted to the bundled Noto Sans Bengali outline
and checked against it at the default tolerances.

A letter is authored only where its body's order clearly wins. Bengali's
headline is the hard case: in isolated letters writers draw it first, last,
partly or not at all, depending on the letter. Where one placement wins a
majority and covers the printed bar, the path follows it (ব and র draw it
first). Where none does, the body is drawn in its majority order and the
headline is drawn **last**, as its own stroke, left to right, by a documented
convention: the way the Bengali track's own lessons teach the bar (body first,
bar last), since in running text it runs across the whole word, as in
Devanagari. Fourteen letters use it (ই চ ছ জ ড ত দ ন ফ ভ ম য ল হ); each
record says so with its counts, and `tests/strokes/bengali.test.ts` pins the
list. ক (its majority bar stops at the stem), আ গ ট ধ প স শ (no clear body
order) and ঝ (Noto splits its headline around the rising right stem) stay
out, with the reason recorded in `data/scripts/bengali.json`.
