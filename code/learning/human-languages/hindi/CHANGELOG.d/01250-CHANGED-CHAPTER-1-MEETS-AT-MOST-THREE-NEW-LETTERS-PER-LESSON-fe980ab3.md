## Changed — chapter 1 meets at most three new letters per lesson

The script-ramp report (`ramp.script`, policy `maxNewGlyphsPerLesson: 3`)
counts every target-script shape the first time a lesson body shows it.
It listed HI-C01-namaste (6) and HI-C01-dhanyavad (4); no Hindi lesson is
listed now. Each word lesson still teaches the letters of its own word inline,
but only up to three new shapes, and says where the rest arrive.

| lesson | before | after (what the lesson reads) |
|---|---|---|
| HI-C01-namaste | 6 न म स ् त े | 3 न त े — **न** and **ते** |
| HI-C01-namaskar | 3 क ा र | 3 म स ् — **नमस्** and the *halant* |
| HI-C01-dhanyavad | 4 ध य व द | 3 ध व द — **ध** and the root **वद्** |
| HI-C01-shukriya | 3 श ु ि | 3 क र ि — the conjunct **क्रि** |
| HI-C01-alvida | 2 अ ल | 3 अ ल ा — the whole **अलविदा** |
| HI-W01-shirorekha-na-ma | 2 ो ख | 3 श ो ख |

- The three facts that read Devanagari are now taught across two lessons:
  *namaste* gives the inherent *a* and the *mātrā*, *namaskār* the *halant*.
- The whole **नमस्ते** first appears where the head-line lesson already traces
  it; **नमस्कार** first appears whole in the chapter practice. Both lessons
  say so. *Dhanyavād* and *shukriyā* stay spoken until later chapters spell
  them, and their headings are romanized.
- Every word lesson shows only shapes of its own headword, so script closure
  stays 0. Regenerated book chapter 1, narration, modality and hashes.
