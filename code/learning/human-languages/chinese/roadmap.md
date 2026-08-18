# Mandarin Roadmap — Absolute Beginner toward B1

Mandarin follows the shared communicative spine and extends it where the language
genuinely needs different machinery: tone, character composition, and the
character-versus-word distinction. The unit is one expression per lesson, under
five minutes. Characters are learned inside useful words, never as a gated
character list.

This track exists as a **scale test**. Every other track in this curriculum is
Indo-European or Dravidian, and the method's signature device — anchoring a new
word to English words the reader already owns through a shared ancestor — depends
on that shared ancestry. Mandarin has none. What replaces the cousin web, and what
breaks when it is removed, is recorded honestly here and in
[`README.md`](./README.md) rather than papered over.

## Chapter 1 — Nǐ hǎo: hello, character by character *(authored)*

**mā má mǎ mà** → **你** *nǐ* → **好** *hǎo* → **你好** *nǐ hǎo* → third-tone
sandhi → **好** *hào* → one two-line exchange.

Seven lessons that between them establish everything structurally new about this
language, on the smallest possible vocabulary:

- a character is a block built from named **components** (你 = 亻 + 尔);
- component glosses are the **memory hook** that replaces the cousin web
  (好 = 女 beside 子), flagged as a traditional gloss rather than proven history;
- a **word** may be more than one character (你好 is two characters, one word);
- **pitch is part of the word** (好 hǎo "good" against 好 hào "to be fond of");
- and a neighbouring word can **change that pitch** without changing a stroke
  (nǐ hǎo is spoken ní hǎo).

The chapter deliberately teaches two characters and one greeting. The ramp is
slow on purpose: the new machinery, not the vocabulary, is the load.

## Thanks, and the speech radical *(planned)*

**谢谢** *xièxie*. Introduces the 讠 "speech" component and the reduplication
that turns 谢 into an everyday thank-you, plus the neutral tone on the second
syllable. Realises `SPINE-COURTESY-THANK`.

## Yes, no, and 不 — *delivered across two chapters*

**Delivered (HL-C239).** 不 *bù* alone, taught as the negator, with 不好 as the
worked example, and 口 / 日 alongside it. `RESPONSE-NO` is claimed, because
Mandarin answers a yes-or-no question by negating the word it asked about rather
than with a bare reply word.

**Delivered (HL-C240), discharging the debt recorded here.** 是 *shì* and 不是
*bú shì*, and with them the **不 tone-sandhi rule** — 不 shifts to *bú* before a
fourth tone, which 不是 is the first collocation to force. `RESPONSE-YES` is now
claimed as well, by the same mechanism in the other direction: asked whether
something 是, you answer 是. `SPINE-RESPOND-BASIC` is realised.

对 *duì*, the commoner bare affirmative, is **not** taught, and the reason is the
font constraint below rather than pedagogy — it is absent from the vendored
subset and cannot be typeset. The lessons scope themselves accordingly and say
outright that 是 is not the only way Mandarin says yes.

Mandarin has no bare word for "yes" either, which remains a spine-shaped
divergence worth its own lesson.

## Taking leave — 再见 *(delivered, HL-C241)*

**再见** *zàijiàn*, "see again". Realises `SPINE-TAKE-LEAVE` and claims
`FAREWELL` — the plain parting word, which the shared taxonomy files alongside
*adiós*, *au revoir* and *auf Wiedersehen*. The time-specific partings remain
omitted.

It is the first word in the track whose two components are each a **taught
character carrying an independent meaning that survives into the compound**, and
the lessons make that the point. Every earlier *component* gloss was labelled a
memory hook, offered in place of the cousin web this track cannot have — 亻 at
the left of 你, 女 beside 子 in 好. 再 "again" + 见 "see" is not a hook. The parts
add up, and the reader is told to start expecting that.

The claim is scoped to sub-character components deliberately. 不是 in the
previous chapter is also read literally, 不 in front of 是; what is new here is
that both halves are characters the reader has written, each with a meaning of
its own outside this word.

It is also the chapter where the tone story pays off. 你好 moves, 不是 moves,
再见 does not — two falls, written as two falls — which is the cleanest available
demonstration that the writing records the words and not what happens between
them.

## Names — 我 / 名字 / 什么 *(planned; blocked on a re-subset)*

**我** *wǒ* → **名字** *míngzi* → **什么** *shénme* → the name question.
Realises `SPINE-EXCHANGE-NAMES`. 字 (宀 roof + 子 child) is ready: both pieces are
inventoried and both are already taught.

**名 needs care but is not blocked.** 名 itself is inventoried and has a ductus.
**夕**, the piece above 口 that makes 名 decomposable, is *not* inventoried and
has no ductus — but it **is typesettable today**, because the subset script
collects every CJK codepoint appearing anywhere in
[`data/scripts/chinese.json`](../data/scripts/chinese.json), and 夕 appears there
inside 名's own `components` text. Verified by reading the subset's cmap, not by
trusting this file.

So there are two different costs, and they should not be confused:

- **Naming and showing 夕 as a component** costs nothing new. This is exactly how
  儿 is handled in the 见 lesson — printed, named, related to a character the
  reader already knows, with no `letters` entry and no ductus.
- **Teaching 夕 as a glyph with its own stroke order** means a `letters` entry in
  `chinese.json`, which **obliges a hand-authored, font-checked ductus** in
  `script-ductus`. Only this path costs real work, and re-running the subset
  script (which needs network) is only required for a character that appears
  *nowhere* in the file.

Chapter 1 promises the reader "never a character you have not been shown the
parts of first", so 名 must show 夕 either way. The component route satisfies that
promise at no cost.

## Times of day *(planned)*

**早上好** *zǎoshang hǎo*, reusing 好 in the productive `X + 好` greeting frame.
Realises part of `SPINE-TIME-OF-DAY` and pays back Chapter 1 by showing that 好
was a building block, not a one-off.

## Part II onward *(sketch)*

Measure words, the 是…的 frame, location with 在, the aspect marker 了, and
the resultative complements — each introduced on the first word that needs it.
Traditional characters are a labelled future variety, not a silent substitute for
the simplified forms taught here.

## The font constraint, recorded because it shapes the plan

The vendored `_fonts/NotoSansSC-Subset.ttf` covers every CJK codepoint that
appears **anywhere** in [`data/scripts/chinese.json`](../data/scripts/chinese.json)
— not merely the inventoried `letters`, but also every character named inside
another character's components, stroke notes or citations. That is 51 characters
against 29 inventoried glyphs, and the gap is load-bearing: 夕, 十, 目, 儿, 寸,
身 and 青 are all printable without being taught. **Read the font's cmap before
concluding a character is unavailable** — this file previously asserted the
narrower rule and sent a chapter down a re-subset path it did not need. A character
mentioned nowhere in that file does need an entry plus a re-run of
[`_fonts/subset-cjk.sh`](../_fonts/subset-cjk.sh), which needs network access to
fetch the ~17 MB upstream Noto Sans SC. 对 *duì* is the live example: absent
entirely, hence untypesettable, hence untaught.

Every chapter delivered so far was authored inside the existing subset, and the
planned ones above were ordered the same way. This is still a real constraint on
chapter order that no alphabetic track in this curriculum has, because a Noto
subset for an alphabet covers the whole writing system at once — but it is a
narrower constraint than this file used to claim.
