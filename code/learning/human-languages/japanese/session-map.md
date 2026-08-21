# Japanese Session Map — Script Before Decoding

This is the authoritative order for the starter course. Every new lesson is an
independent step of at most 225 seconds. A learner may add due review prompts,
but no session fuses two new signs into one indivisible lesson.

The governing rule is simple: a sign becomes load-bearing only after a script
lesson has isolated its shape, sound or reading, and a small writing action. A
historical spelling or advanced example that has not earned that preparation is
given in romanization and postponed.

## New-lesson sessions

| Session | Chapter | New lesson | One small outcome |
|---|---:|---|---|
| **S1** | 1 | [`JA-W01-i`](./lessons/JA-W01-i.md) | trace and write **い** |
| **S2** | 1 | [`JA-W01-ha`](./lessons/JA-W01-ha.md) | trace and write **は** |
| **S3** | 1 | [`JA-W01-hai-read`](./lessons/JA-W01-hai-read.md) | join **は + い** |
| **S4** | 1 | [`JA-W01-e`](./lessons/JA-W01-e.md) | trace and write **え** |
| **S5** | 1 | [`JA-C01-hai`](./lessons/JA-C01-hai.md) | hear, say, read and write **はい** |
| **S6** | 1 | [`JA-C01-iie`](./lessons/JA-C01-iie.md) | contrast **はい / いいえ** |
| **S7** | 2 | [`JA-W01-ko`](./lessons/JA-W01-ko.md) | trace and write **こ** |
| **S8** | 2 | [`JA-W01-n`](./lessons/JA-W01-n.md) | trace and write **ん** |
| **S9** | 2 | [`JA-W01-ni`](./lessons/JA-W01-ni.md) | trace and write **に** |
| **S10** | 2 | [`JA-W01-chi`](./lessons/JA-W01-chi.md) | trace and write **ち** |
| **S11** | 2 | [`JA-W01-wa`](./lessons/JA-W01-wa.md) | trace and write **わ** |
| **S12** | 2 | [`JA-W01-konnichiwa-read`](./lessons/JA-W01-konnichiwa-read.md) | assemble the greeting |
| **S13** | 2 | [`JA-C01-konnichiwa`](./lessons/JA-C01-konnichiwa.md) | use the daytime greeting |
| **S14** | 3 | [`JA-W03-a`](./lessons/JA-W03-a.md) | trace and write **あ** |
| **S15** | 3 | [`JA-W03-ri`](./lessons/JA-W03-ri.md) | trace and write **り** |
| **S16** | 3 | [`JA-W03-ka`](./lessons/JA-W03-ka.md) | trace and write **か** |
| **S17** | 3 | [`JA-W03-dakuten`](./lessons/JA-W03-dakuten.md) | add the dakuten |
| **S18** | 3 | [`JA-W03-to`](./lessons/JA-W03-to.md) | trace and write **と** |
| **S19** | 3 | [`JA-W03-u`](./lessons/JA-W03-u.md) | trace and write **う** |
| **S20** | 3 | [`JA-W03-arigatou-read`](./lessons/JA-W03-arigatou-read.md) | assemble **ありがとう** |
| **S21** | 3 | [`JA-C01-arigatou`](./lessons/JA-C01-arigatou.md) | use plain thanks |
| **S22** | 4 | [`JA-W03-sa`](./lessons/JA-W03-sa.md) | trace and write **さ / ざ** |
| **S23** | 4 | [`JA-W03-ma`](./lessons/JA-W03-ma.md) | trace and write **ま** |
| **S24** | 4 | [`JA-W03-su`](./lessons/JA-W03-su.md) | trace and write **す** |
| **S25** | 4 | [`JA-C01-gozaimasu`](./lessons/JA-C01-gozaimasu.md) | choose polite thanks |
| **S26** | 4 | [`JA-C03-practice`](./lessons/JA-C03-practice.md) | read the complete polite form |
| **S27** | 5 | [`JA-W05-nichi-kanji`](./lessons/JA-W05-nichi-kanji.md) | write four-stroke **日** |
| **S28** | 5 | [`JA-W05-hon-kanji`](./lessons/JA-W05-hon-kanji.md) | write five-stroke **本** |
| **S29** | 5 | [`JA-W05-gen-component`](./lessons/JA-W05-gen-component.md) | practise speech component **言** |
| **S30** | 5 | [`JA-W05-five-component`](./lessons/JA-W05-five-component.md) | practise sound clue **五** |
| **S31** | 5 | [`JA-W05-mouth-component`](./lessons/JA-W05-mouth-component.md) | practise box component **口** |
| **S32** | 5 | [`JA-W05-go-kanji`](./lessons/JA-W05-go-kanji.md) | assemble **語** from three blocks |
| **S33** | 5 | [`JA-C01-nihongo`](./lessons/JA-C01-nihongo.md) | read and write **日本語** |
| **S34** | 6 | [`JA-W06-ko-katakana`](./lessons/JA-W06-ko-katakana.md) | write katakana **コ** |
| **S35** | 6 | [`JA-W06-long-mark`](./lessons/JA-W06-long-mark.md) | add one mora with **ー** |
| **S36** | 6 | [`JA-W06-hi-katakana`](./lessons/JA-W06-hi-katakana.md) | write katakana **ヒ** |
| **S37** | 6 | [`JA-C01-koohii`](./lessons/JA-C01-koohii.md) | read **コーヒー** as four morae |
| **S38** | 7 | [`JA-C01-practice`](./lessons/JA-C01-practice.md) | run the complete doorway exchange |

## Review rule

The `reviews_of` field in each lesson is the machine-checked review queue. The
fixed no-tracking fallback uses the session-count windows from
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md): retrieve a
new atom at N+1, N+3, N+7 and N+15 when the authored runway is long enough, then
move it into the mixed pool. A missed item returns sooner; it never causes the
new-sign lesson to grow beyond five minutes.

The six chapter payoffs at S6, S13, S21, S26, S33 and S37 are cumulative review
points. S38 checks listening, speaking, reading and writing separately rather
than letting recognition stand in for production.
