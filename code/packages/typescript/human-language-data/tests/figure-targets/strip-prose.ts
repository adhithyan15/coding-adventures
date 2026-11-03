// The two pieces of lesson prose the real-corpus filmstrip cases look for.
//
// They live here, not in the cases that assert on them, because the real
// corpus is read ONCE for the whole suite (see real-corpus-fixture.ts) and the
// fixture has to know, while each track is in memory, which lessons' bodies
// match. The cases still own the claims: each reads the matching lesson ids and
// the drawn targets and states its own assertion.

/**
 * The recognition segments' honest-at-the-time disclaimer, "This book does not
 * yet tell you where to start the character ... copy what you see"
 * (data/scripts/author_recognition_segments.py, `writing_block`). Tolerant of
 * the bold markers and line wrapping it is written with ("does not yet tell you
 * **where to start", across a `> ` line break), plus its heading "— copy what
 * you see".
 */
export const STROKE_ORDER_DISCLAIMER = /does not (?:yet )?tell you[\s>*]*where to start|copy what you see/;

/**
 * The prose written for strip lessons ("follow the numbered strip", "The strip
 * shows where to start"), which must not be left on a lesson whose strip has
 * gone.
 */
export const STRIP_PROSE = /numbered strip|\bthe strip (?:shows|names|writes|gives|draws)\b/i;
