## Unreleased — Chapters 46-52: Thirty-five everyday words, one per lesson

Malayalam stood at 49 headwords against the 300 the pre-A1 vocabulary floor asks
for. These seven chapters answer with thirty-five, and with nothing else — **one
new word per lesson**, and reuse of everything already taught, unlimited and on
purpose.

  46 Things You Ask For        പഴം തുണി വിളക്ക് ഉപ്പ് പുസ്തകം
  47 The Leg and the Tooth     കാൽ പല്ല് മുടി വിരൽ വയറ്
  48 Who Someone Is            അധ്യാപകൻ വിദ്യാർത്ഥി വൈദ്യൻ കർഷകൻ അതിഥി
  49 Short Replies             സത്യം മതി തീർച്ചയായും ഒരുപക്ഷേ അങ്ങനെ
  50 Taking Leave              ഇപ്പോൾ മറ്റന്നാൾ യാത്ര പുറപ്പെടുക വിട
  51 Courtesy                  കൃതജ്ഞത ഉപകാരം ബഹുമാനം അനുഗ്രഹം വന്ദനം
  52 Welcoming a Guest         വാതിൽ കസേര കോലം പൂവ് മാല

Each chapter sits on one of the seven pre-A1 spine nodes, five lessons sharing
it, and each lesson chains to the one before — so a word introduced by a
chapter's payoff lesson is still being practised two lessons into the next
chapter. That is why the ramp got *gentler* rather than steeper: the R1
reinforcement ratio falls 0.3034 to 0.3005 even though the corpus grew. Not one
of the thirty-five new atoms misses its R1 window.

Every headword was checked against all 142 existing lessons before it was
written, in both directions, so none of the thirty-five re-teaches anything and
none of them makes an earlier page point forward — the corpus forward-reference
figure holds at its 500 ceiling, and so does the thirty-count on rule statements.

Two threads run the length of the seven chapters rather than sitting in one
lesson. The first is the chillu, the consonant written without a vowel and given
a letter of its own: ൽ closes കാൽ, വിരൽ, വാതിൽ and ജനൽ; ൻ marks the three
masculine role-words; ൾ closes ഇപ്പോൾ and മറ്റന്നാൾ; ർ sits inside വിദ്യാർത്ഥി,
കർഷകൻ and തീർച്ചയായും. Each is written as its own atomic character rather than as
a consonant plus virama plus a zero-width joiner, because U+200D belongs to no
Unicode script block at all and so falls through every font selection the book
makes — a trap the Kannada tranche fell into and this one is written to avoid.

The second is the standing division of labour between an inherited word and a
borrowed one. തുണി against വസ്ത്രം, മുടി against രോമം, നന്ദി against കൃതജ്ഞത,
വിളക്ക് against the Sanskrit lamp Kannada took instead: the native word does the
daily work and the borrowed one does the ceremony, again and again, until it
stops being a fact about five words and becomes a fact about the language.

Chapter 49 spends its last lesson collecting on a debt from the chapter on
pointing: അങ്ങനെ is എങ്ങനെ with the far-pointer swapped in for the question one,
so the reader who saw i- / a- / e- once gets the fifth reply for nothing. Chapter
52 does the same with വായ്, the mouth-word from the chapter on the face, which
Tamil's door-word வாயில் shows sitting inside വാതിൽ.

Chapters 43, 44 and 45 also move from a bare `unicodeScript` to the track's
`malayalam-comparisons` script set, matching chapters 6-42 (backlog HL-C200).
Their rendered output is byte-identical and their book hashes did not move,
because they happen to cite no cousin script today; what changes is that a
comparison table added to them later can no longer drop its glyphs silently into
the Latin font at exit 0.

