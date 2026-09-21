## Unreleased — Chapters 53-59: Thirty-five more words, round two

The first tranche took Kannada from 47 pre-A1 headwords to 82. That left it last
of the seven tracks that are being carried to the 300-word floor together. These
seven chapters answer with thirty-five more, on the same terms as before: **one
new word per lesson**, and unlimited reuse of everything already taught.

  53 The Sky                    ಆಕಾಶ ಬಿಸಿಲು ಚಂದ್ರ ಚುಕ್ಕಿ ಮೋಡ
  54 The Tree                   ಮರ ಕೊಂಬೆ ಎಲೆ ಬೇರು ಬೀಜ
  55 River and Road             ನದಿ ಕೆರೆ ಬೆಟ್ಟ ಹಳ್ಳಿ ದಾರಿ
  56 In the House               ಚಾಪೆ ಬುಟ್ಟಿ ಚಾಕು ಪಾತ್ರೆ ಪೆಟ್ಟಿಗೆ
  57 More of the Body           ಕುತ್ತಿಗೆ ಬೆನ್ನು ತುಟಿ ಭುಜ ಮೂಳೆ
  58 More Short Replies         ಸ್ವಲ್ಪ ಹೆಚ್ಚು ಕಡಿಮೆ ಬೇಡ ಅಷ್ಟೇ
  59 Weather and Ground         ಗಾಳಿ ಮಂಜು ಮರಳು ಕೆಸರು ಬೆಂಕಿ

Attainment 82 → 117 of 300, a rise of exactly thirty-five. All seven pre-A1
spine nodes are used a second time, one per chapter, and the chain runs unbroken
from ಹೂಮಾಲೆ at the end of the welcome chapter to ಬೆಂಕಿ at the end of this run —
each chapter's second lesson still practising the previous chapter's last word.

The ramp got gentler again rather than steeper. R1 falls 0.2869 → 0.2843 with
its numerator held at exactly 1106: not one of the thirty-five new atoms misses
its first reinforcement window, so the whole improvement is a larger denominator.

Every candidate was checked against the whole track before it was written, in
both directions and inside existing headwords. That check is what kept ಸೂರ್ಯ out
of the sky chapter: the ordinary word for the sun is already on the page, whole,
inside ಸೂರ್ಯೋದಯ in the good-morning lesson, so teaching it here would have made
that earlier page point forwards. ಬಿಸಿಲು took the slot instead, which is the more
useful word anyway — a Kannada sentence about the afternoon is far likelier to be
about the sun's heat than about the sun. ಮಳೆ went the same way, already present
inside ಮಳೆಗಾಲ. The corpus figure for forward references holds at its 500 ceiling
and rule statements hold at thirty.

Every headword is spelled entirely from glyphs the track's own writing lessons
teach, so the tranche adds no script-closure violation and nothing is laundered
through an exposure-exempt headword: Kannada's exempted-glyph count is unmoved at
162. The same filter shaped the prose — ಊಟ appears in romanization rather than in
script, because ಊ is a letter no writing lesson has taught yet.

Two sound laws now carry the run rather than sitting in one remark each. ಬೇರು,
ಬೆಟ್ಟ and ಬೆನ್ನು put Kannada's *b* against the family's *v* in three separate
chapters, and ಹಳ್ಳಿ shows the *p* that became *h* written on road signs the
length of the state, where the unchanged *-palli* survives across the border.

Chapter 59 closes on a deliberate refusal: ಮರಳು has ಮರ standing at the front of
it and is not related to it, and the lesson says so, because knowing when an
etymology is a coincidence is part of what this book is teaching.

Fixed, in `roadmap.md`: the Tamil word cited beside ಬರೆ carried a KANNADA LETTER
RA (U+0CB0) in place of the Tamil one, so a Tamil word was rendering with one
letter drawn from the Kannada font. Both fonts load and nothing was missing, so
the build had always exited 0 with zero warnings while printing the wrong glyph.
Found by decomposing every word in the tree into Unicode letter names from the
FILE BYTES and flagging any word naming two scripts (backlog HL-C202).

