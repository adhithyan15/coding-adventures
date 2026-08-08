# Urdu — Pronunciation & Script Reference

A **reference**, not a chapter. Use it when a lesson names a sound; do not clear
it as an alphabet gate. Urdu script is introduced through useful expressions,
with only the shapes and sound contrasts needed on that page.

The track shows a learner romanization beside new Urdu. A line above a vowel
marks length (**ā, ī**), and a tilde marks nasalization (**ā̃, ī̃**). Cover the
romanization once the Urdu form is familiar.

## Script, spelling, and type style

- **Read right to left** (`rtl`). Start each word at its right edge.
- **Letters usually join** and change shape by position. Some letters, including
  **ا** and **ر** in current words, break the connection to the following shape
  on their left.
- Urdu is an **abjad**: some vowels are unwritten and inferred from the learned
  word (`short-vowels-unwritten`). Long vowels use letter shapes such as **ا**,
  **و**, and **ی**; **ے** can carry *e/ai*.
- Urdu is traditionally printed in **Nastaliq**, whose connected words descend
  in a flowing slope. The book and app use the vendored static Noto Nastaliq Urdu
  family, including its Urdu OpenType localization and contextual joins. Naskh
  remains an accessibility fallback when a learner's browser cannot load the
  course font; it is no longer the normal presentation.

## Sound ids used by the starter lessons

| Sound id | What to do | First anchor |
|---|---|---|
| `rtl` | follow the word from its right edge leftward | **سلام** *salām* |
| `long-a` | hold *ā* steady; **ا** carries it in **سلام** | **سلام** *salām* |
| `alif-madd` | read **آ** as word-initial long *ā* | **آپ** *āp* |
| `short-vowels-unwritten` | supply the learned short vowels even when vowel marks are absent | **شکریہ** *shukriyā* |
| `long-i` | hold **ی** as *ī* in this word | **جی** *jī* |
| `long-u` | hold **و** as long *ū* in the learned word | **ہوں** *hūṅ* |
| `nasal-vowel` | let air pass through the nose; final **ں** does not add a full English *n* | **ہاں** *hā̃*, **نہیں** *nahī̃* |
| `long-vowels` | read the word's written long-vowel cues rather than stretching every vowel | **میرا نام** *merā nām* |
| `hai-diphthong` | say **ہے** as one *hai* syllable, ending in an *ai* glide | **ہے** *hai* |
| `sh` | read three-dotted **ش** as *sh* | **شکریہ** *shukriyā* |
| `kh` | make **خ** farther back than English *h* | **خوشی** *khushī* |
| `consonantal-ye` | let **ی** supply consonantal *y* before a vowel | **کیا** *kyā* |
| `ai-diphthong` | say *ai* as one glide in the learned question word | **کیسے** *kaise* |
| `final-ye` | distinguish broad final **ے** *e* from final **ی** long *ī* | **کیسے / کیسی** *kaise / kaisī* |
| `retroflex-aspirated-th` | curl the tongue slightly back for **ٹ**, then release the aspiration marked by **ھ** | **ٹھیک** *ṭhīk* |
| `urdu-question-mark` | recognize **؟** at the left end of a right-to-left question | **آپ کا نام کیا ہے؟** |
| `be-vs-pe-dots` | count the dots under the shared low scoop: one below is **ب** *b*, three below are **پ** *p* | **بولنا** *bolnā* against **آپ** *āp* |
| `geminate-nun` | hold the *n* long when two **ن** letters are written in a row | **جاننا** *jānnā* |
| `che-vs-jim-dots` | count the dots below the *jīm* body: one is **ج** *j*, three are **چ** *ch* | **سوچنا** *sochnā* against **جانا** *jānā* |
| `do-chashmi-he` | give **ھ** no sound of its own; let it aspirate the letter to its right | **سمجھنا** *samajhnā*, **لکھنا** *likhnā* |
| `retroflex-flap-rre` | flick a curled-back tongue once for **ڑ**; do not roll it | **پڑھنا** *paṛhnā* |
| `majhul-e` | read medial **ی** as long *e* where the learned word calls for it | **لینا** *lenā* |

## Shape families already in use

- **س** *s* and **ش** *sh* share a skeleton; three dots distinguish **ش**.
- **ن** is consonantal *n*. Final dotless **ں** is *nūn-e ghunna* and marks
  nasalization in **ہاں** and **نہیں**.
- **ی** supplies the long *ī* in **جی** and **نہیں**. Final **ے** participates
  in *hai* in **ہے**. Learn each role in its word before generalizing.
- **کیا** gives **ی** its consonantal *y* job; **کیسے / کیسی** then contrast
  broad final **ے** with long-*ī* final **ی**.
- **ٹھیک** introduces the Indic retroflex-plus-aspiration sequence **ٹھ** only
  when the wellbeing reply needs it.
- **خدا حافظ** reuses **خ** and long **ا**, adds a clear initial *h* in **حافظ**,
  and reads final **ظ** as *z*. Urdu conventionally keeps the two words visibly
  spaced, even though Persian normally joins its local spelling.
- **بولنا** adds the only new consonant the verb chapter needs: **ب** *b*, one
  low scoop with a single dot below. It shares that skeleton with **پ** *p*,
  already read in **آپ**, which carries three dots below instead. Dot count is
  the whole difference, and it is load-bearing throughout the alphabet.
- **جاننا** writes two **ن** letters in a row and pronounces both, which is the
  only thing separating it from **جانا** *jānā*. Nothing about the doubling is
  decorative: **جانا** is “to go” and **جاننا** is “to know.”
- **چ** *ch* is the **ج** *j* body with **three dots below** instead of one — the
  same dot-count contrast as **ب** against **پ**, on a second skeleton.
- **ھ** is *do-chashmī he*, “two-eyed he.” It is never a consonant on its own;
  it aspirates whatever letter stands to its right. **ٹھیک** used it first, and
  **سمجھنا**, **پڑھنا**, **پوچھنا** and **لکھنا** use it again in **جھ**,
  **ڑھ**, **چھ** and **کھ**. Keep it apart from round **ہ** *gol he*, which is
  a real *h* and is the letter in **ہوں** and **ہونا**.
- **ڑ** is **ر** *r* wearing the small retroflex mark that also rides on **ٹ**.
  One diacritic, one instruction — curl the tongue back — now on two letters.
- **ی** takes a **third** value in **لینا** *lenā*: a long *e*. With
  consonantal *y* in **کیا** and long *ī* in **جی**, and broad final **ے**
  beside it, the *ye* family covers *y*, *ī* and *e*; the learned word decides.

## The Persian-Arabic and Indo-Aryan layers

Script does not determine a word's history. **سلام** and **شکریہ** expose the
Persian-Arabic literary layer; **نہیں**, **میرا**, **نام**, and **ہے** expose the
inherited Indo-Aryan core that Urdu shares with Hindi. Mixed-language practice
may use that relationship as a bridge, but the Urdu script remains the assessed
form for this track.

## Sources

- [*Zero Zabar*: How the Urdu script works](https://openbooks.library.northwestern.edu/zerozabar/front-matter/introduction/)
- [*Zero Zabar*: The Urdu alphabet](https://openbooks.library.northwestern.edu/zerozabar/chapter/the-urdu-alphabet/)
