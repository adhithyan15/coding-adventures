# Modality and the drivable course (HL08)

[`HL08`](../../../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md) asks a
concrete question: **how much of this can I learn in the car?** `modality.ts` answers
it for every lesson and every chapter.

```ts
import { summarizeModality, deriveLessonModality, loadEverything } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const modality = summarizeModality(lessons);
modality.drivablePercent;              // 84 — the share learnable by ear alone

modality.drivablePercent;              // 65 — the share a hands-free view can deliver
modality.tracks[0].chapters[0];        // { drivablePrefix: 5, firstNonVoiceLesson: "…", … }
deriveLessonModality(lessons[0]).reasons; // ["wide-table"] — why it needs eyes
```
