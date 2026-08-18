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

## Chapter 2 — Thanks, and the speech radical *(planned)*

**谢谢** *xièxie*. Introduces the 讠 "speech" component and the reduplication
that turns 谢 into an everyday thank-you, plus the neutral tone on the second
syllable. Realises `SPINE-COURTESY-THANK`.

## Yes, no, and 不 — *partly delivered, chapter 4*

**Delivered (HL-C239, chapter 4).** 不 *bù* alone, taught as the negator, with
不好 as the worked example, and 口 / 日 alongside it. `SPINE-RESPOND-BASIC` is
**realised** by this: `RESPONSE-NO` is now claimed, because Mandarin answers a
yes-or-no question by negating the word it asked about rather than with a bare
reply word.

**Still owed, and deliberately deferred.** 是 *shì* and 不是 *bú shì*, and with
them the **不 tone-sandhi rule** — 不 shifts to *bú* before a fourth tone. The
chapter's only collocation is 不好, where 好 is third tone and the rule does not
fire, so nothing taught is wrong; but the rule is not yet anywhere in the track's
lessons and `pronunciation-reference.md` still carries it as unused. Teach it
with the first fourth-tone partner, which 不是 supplies.

Mandarin has no bare word for "yes" either, which remains a spine-shaped
divergence worth its own lesson.

## Chapter 4 — Names *(planned)*

**我** *wǒ* → **名字** *míngzi* → **什么** *shénme* → the name question.
Realises `SPINE-EXCHANGE-NAMES`. 名 (夕 evening + 口 mouth) and 字 (宀 roof +
子 child) both carry good component hooks. This is the first planned chapter that
needs a character the vendored font subset does not yet contain (叫 *jiào*), so it
must re-subset before it can be typeset — see the note at the foot of this page.

## Chapter 5 — Taking leave *(planned)*

**再见** *zàijiàn*, "see again". Realises `SPINE-TAKE-LEAVE`, and is the first
word in the track whose two components combine into a transparent, honest
literal reading.

## Chapter 6 — Times of day *(planned)*

**早上好** *zǎoshang hǎo*, reusing 好 in the productive `X + 好` greeting frame.
Realises part of `SPINE-TIME-OF-DAY` and pays back Chapter 1 by showing that 好
was a building block, not a one-off.

## Part II onward *(sketch)*

Measure words, the 是…的 frame, location with 在, the aspect marker 了, and
the resultative complements — each introduced on the first word that needs it.
Traditional characters are a labelled future variety, not a silent substitute for
the simplified forms taught here.

## The font constraint, recorded because it shapes the plan

The vendored `_fonts/NotoSansSC-Subset.ttf` is a subset covering exactly the
characters in [`data/scripts/chinese.json`](../data/scripts/chinese.json). Adding
a character to a lesson means adding it to that file and re-running
[`_fonts/subset-cjk.sh`](../_fonts/subset-cjk.sh), which needs network access to
fetch the ~17 MB upstream Noto Sans SC. Chapter 1 was authored **inside the
existing subset** for that reason, and Chapters 2, 3, 5 and 6 above were ordered
the same way — every character they need is already in the inventory. Chapter 4
is the first that is not, and it must re-subset before it can be typeset. That is
a real constraint on chapter order that no alphabetic track in this curriculum
has, because a Noto subset for an alphabet covers the whole writing system at
once.
