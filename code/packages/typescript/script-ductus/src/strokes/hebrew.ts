// Authored hebrew ductus records. This is the stable source-ownership boundary.

import type { LetterDuctus, Point, Stroke, StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import hebrew from "../../../../../learning/human-languages/data/scripts/hebrew.json";

const hebrewAlphabetSource = (glyph: string): StrokeSource => {
  const letter = hebrew.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Hebrew ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  // HebrewPod101's second handwritten Alef demonstration draws one descending
  // diagonal, lifts, then draws the opposing diagonal across it. This learner
  // path keeps those two pen-down runs while routing the crossing through the
  // branches of the vendored Noto Sans Hebrew block Alef.
    ["hebrew:א", {
    script: "hebrew",
    glyph: "א",
    strokes: [
      {
        segments: [
          {
            label: "draw the main diagonal down and right",
            path: [
              { x: 120, y: 560 },
              { x: 180, y: 480 },
              { x: 250, y: 400 },
              { x: 320, y: 310 },
              { x: 390, y: 220 },
              { x: 470, y: 100 },
              { x: 540, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend from the upper-right arm to the crossing",
            path: [
              { x: 540, y: 560 },
              { x: 535, y: 500 },
              { x: 525, y: 430 },
              { x: 500, y: 370 },
              { x: 470, y: 330 },
              { x: 425, y: 290 },
              { x: 385, y: 285 },
            ],
          },
          {
            label: "continue through the crossing and down the lower-left leg",
            path: [
              { x: 385, y: 285 },
              { x: 350, y: 315 },
              { x: 320, y: 340 },
              { x: 280, y: 370 },
              { x: 252, y: 370 },
              { x: 220, y: 350 },
              { x: 175, y: 300 },
              { x: 135, y: 220 },
              { x: 105, y: 120 },
              { x: 85, y: 30 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("א"),
  }],
  // The same lesson's block-style Bet joins the top bar directly to the right
  // descent, then lifts once before drawing the baseline left-to-right. Its
  // later dagesh is an optional mark and is not part of base U+05D1 here.
    ["hebrew:ב", {
    script: "hebrew",
    glyph: "ב",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 90, y: 555 },
              { x: 170, y: 555 },
              { x: 260, y: 555 },
              { x: 330, y: 540 },
              { x: 390, y: 500 },
              { x: 415, y: 430 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 415, y: 430 },
              { x: 415, y: 330 },
              { x: 415, y: 220 },
              { x: 415, y: 100 },
              { x: 415, y: 40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the baseline from left to right",
            path: [
              { x: 50, y: 40 },
              { x: 150, y: 40 },
              { x: 250, y: 40 },
              { x: 350, y: 40 },
              { x: 450, y: 40 },
              { x: 520, y: 40 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ב"),
  }],
  // The dedicated Gimel lesson's printed-form demonstration joins its short
  // top bar to the right stem and short lower-right leg. It then lifts once,
  // restarts at the lower junction, and draws the longer leg down-left. That
  // angular order follows Noto Sans Hebrew while the source note preserves the
  // lesson's visibly different rounded cursive alternative.
    ["hebrew:ג", {
    script: "hebrew",
    glyph: "ג",
    strokes: [
      {
        segments: [
          {
            label: "draw the short top bar from left to right",
            path: [
              { x: 105, y: 555 },
              { x: 145, y: 555 },
              { x: 185, y: 550 },
              { x: 220, y: 535 },
              { x: 245, y: 510 },
            ],
          },
          {
            label: "continue down the right stem without lifting",
            path: [
              { x: 245, y: 510 },
              { x: 260, y: 455 },
              { x: 263, y: 380 },
              { x: 263, y: 300 },
              { x: 263, y: 220 },
              { x: 265, y: 150 },
            ],
          },
          {
            label: "continue into the short lower-right leg",
            path: [
              { x: 265, y: 150 },
              { x: 275, y: 110 },
              { x: 286, y: 70 },
              { x: 300, y: 25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, restart at the lower junction, and draw the longer leg down-left",
            path: [
              { x: 235, y: 155 },
              { x: 215, y: 130 },
              { x: 185, y: 100 },
              { x: 150, y: 75 },
              { x: 110, y: 55 },
              { x: 70, y: 42 },
              { x: 38, y: 40 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ג"),
  }],
  // The source's cursive Dalet is explicitly one curve: a broad left-to-right
  // arch curls through a small loop and continues into its tail. The learner
  // path preserves that zero-lift run while fitting it to Noto Sans Hebrew's
  // angular block top bar, sharp right heel, and downstroke.
    ["hebrew:ד", {
    script: "hebrew",
    glyph: "ד",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 150, y: 555 },
              { x: 240, y: 555 },
              { x: 330, y: 555 },
              { x: 420, y: 555 },
              { x: 480, y: 555 },
            ],
          },
          {
            label: "continue around the sharp right corner and down without lifting",
            path: [
              { x: 480, y: 555 },
              { x: 430, y: 540 },
              { x: 385, y: 510 },
              { x: 370, y: 460 },
              { x: 370, y: 370 },
              { x: 370, y: 270 },
              { x: 370, y: 170 },
              { x: 370, y: 70 },
              { x: 370, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ד"),
  }],
  // The dedicated Hei lesson writes the printed body as a left-to-right top
  // bar that turns down the right side, then lifts once for the detached left
  // leg. This angular order follows Noto Sans Hebrew while the source note
  // preserves the lesson's rounded handwritten alternative.
    ["hebrew:ה", {
    script: "hebrew",
    glyph: "ה",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 150, y: 555 },
              { x: 240, y: 555 },
              { x: 330, y: 555 },
              { x: 410, y: 555 },
              { x: 480, y: 555 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 480, y: 555 },
              { x: 500, y: 530 },
              { x: 510, y: 480 },
              { x: 510, y: 380 },
              { x: 510, y: 270 },
              { x: 510, y: 160 },
              { x: 510, y: 50 },
              { x: 510, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the detached left leg from top to bottom",
            path: [
              { x: 115, y: 320 },
              { x: 115, y: 260 },
              { x: 115, y: 180 },
              { x: 115, y: 100 },
              { x: 115, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ה"),
  }],
  // The dedicated Vav lesson draws the printed head left-to-right and turns
  // directly into the top-to-bottom stem. This two-movement learner path is
  // one continuous zero-lift stroke on the Noto Sans Hebrew outline.
    ["hebrew:ו", {
    script: "hebrew",
    glyph: "ו",
    strokes: [
      {
        segments: [
          {
            label: "draw the small head from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 120, y: 555 },
              { x: 175, y: 555 },
            ],
          },
          {
            label: "continue straight down without lifting",
            path: [
              { x: 175, y: 555 },
              { x: 175, y: 480 },
              { x: 175, y: 380 },
              { x: 175, y: 270 },
              { x: 175, y: 160 },
              { x: 175, y: 60 },
              { x: 175, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ו"),
  }],
  // The lesson's rounded handwritten Zayin begins with a short rightward rise
  // and continues around the body without lifting. This path preserves that
  // order while following Noto Sans Hebrew's broader head and curved stem.
    ["hebrew:ז", {
    script: "hebrew",
    glyph: "ז",
    strokes: [
      {
        segments: [
          {
            label: "draw the short head from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 160, y: 555 },
              { x: 260, y: 555 },
            ],
          },
          {
            label: "continue down through the curved stem without lifting",
            path: [
              { x: 260, y: 555 },
              { x: 220, y: 520 },
              { x: 180, y: 475 },
              { x: 150, y: 425 },
              { x: 132, y: 360 },
              { x: 130, y: 285 },
              { x: 138, y: 205 },
              { x: 148, y: 125 },
              { x: 160, y: 55 },
              { x: 166, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ז"),
  }],
  // The printed Heit demonstration joins its left-to-right top bar to the
  // right descent, then lifts once for the left leg. The source also preserves
  // the same order with rounded corners in handwriting.
    ["hebrew:ח", {
    script: "hebrew",
    glyph: "ח",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 75, y: 555 },
              { x: 170, y: 555 },
              { x: 280, y: 555 },
              { x: 390, y: 555 },
              { x: 480, y: 555 },
              { x: 540, y: 540 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 540, y: 540 },
              { x: 542, y: 480 },
              { x: 542, y: 380 },
              { x: 542, y: 270 },
              { x: 542, y: 160 },
              { x: 542, y: 55 },
              { x: 542, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the joined left leg from top to bottom",
            path: [
              { x: 142, y: 555 },
              { x: 142, y: 480 },
              { x: 142, y: 380 },
              { x: 142, y: 270 },
              { x: 142, y: 160 },
              { x: 142, y: 55 },
              { x: 142, y: 20 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ח"),
  }],
  // Printed Tet uses an L-shaped left-and-base stroke, then restarts at the
  // lower right and climbs before turning inward. The source's rounded
  // handwriting preserves that unusual bottom-up finish as one continuous run.
    ["hebrew:ט", {
    script: "hebrew",
    glyph: "ט",
    strokes: [
      {
        segments: [
          {
            label: "draw the left side from top to bottom",
            path: [
              { x: 103, y: 560 },
              { x: 103, y: 480 },
              { x: 103, y: 380 },
              { x: 103, y: 270 },
              { x: 125, y: 170 },
            ],
          },
          {
            label: "continue around the bottom from left to right without lifting",
            path: [
              { x: 125, y: 170 },
              { x: 160, y: 90 },
              { x: 235, y: 35 },
              { x: 315, y: 25 },
              { x: 400, y: 45 },
              { x: 470, y: 105 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, restart at the lower-right, and climb the right side",
            path: [
              { x: 470, y: 105 },
              { x: 515, y: 170 },
              { x: 537, y: 250 },
              { x: 537, y: 330 },
              { x: 530, y: 420 },
              { x: 500, y: 495 },
              { x: 455, y: 545 },
            ],
          },
          {
            label: "turn down-left into the inward hook without lifting",
            path: [
              { x: 455, y: 545 },
              { x: 410, y: 560 },
              { x: 365, y: 557 },
              { x: 330, y: 540 },
              { x: 315, y: 530 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ט"),
  }],
  // Printed Yod is the same tiny comma-like idea as handwriting with a sharper
  // angle: the head runs left-to-right and turns directly down the short stem.
    ["hebrew:י", {
    script: "hebrew",
    glyph: "י",
    strokes: [
      {
        segments: [
          {
            label: "draw the small head from left to right",
            path: [
              { x: 60, y: 555 },
              { x: 120, y: 555 },
              { x: 180, y: 555 },
            ],
          },
          {
            label: "continue down through the short angled stem without lifting",
            path: [
              { x: 180, y: 555 },
              { x: 180, y: 480 },
              { x: 180, y: 390 },
              { x: 180, y: 300 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("י"),
  }],
  // Printed Kaf sharpens the handwritten half-circle into one continuous
  // top-right-bottom run: across the top, around the right side, then left.
    ["hebrew:כ", {
    script: "hebrew",
    glyph: "כ",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 135, y: 555 },
              { x: 209, y: 555 },
            ],
          },
          {
            label: "continue down the rounded right side without lifting",
            path: [
              { x: 209, y: 555 },
              { x: 300, y: 530 },
              { x: 380, y: 470 },
              { x: 420, y: 385 },
              { x: 423, y: 294 },
              { x: 420, y: 205 },
              { x: 380, y: 120 },
              { x: 300, y: 58 },
              { x: 209, y: 38 },
            ],
          },
          {
            label: "turn left along the base without lifting",
            path: [
              { x: 209, y: 38 },
              { x: 135, y: 38 },
              { x: 60, y: 38 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("כ"),
  }],
  // Printed Lamed is one angular run: down the tall left stroke, right across
  // the middle, then diagonally down-left. Handwriting rounds this into a loop.
    ["hebrew:ל", {
    script: "hebrew",
    glyph: "ל",
    strokes: [
      {
        segments: [
          {
            label: "draw the tall left stroke from top to bottom",
            path: [
              { x: 80, y: 730 },
              { x: 80, y: 660 },
              { x: 80, y: 590 },
              { x: 80, y: 555 },
            ],
          },
          {
            label: "continue right along the middle bar without lifting",
            path: [
              { x: 80, y: 555 },
              { x: 180, y: 555 },
              { x: 300, y: 555 },
              { x: 420, y: 555 },
            ],
          },
          {
            label: "turn diagonally down-left through the lower stroke without lifting",
            path: [
              { x: 420, y: 555 },
              { x: 400, y: 480 },
              { x: 370, y: 390 },
              { x: 340, y: 300 },
              { x: 310, y: 210 },
              { x: 280, y: 120 },
              { x: 250, y: 38 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ל"),
  }],
  // Printed Mem starts with its detached angled left part. After one lift, the
  // angular right body climbs, descends, and returns left along the base.
    ["hebrew:מ", {
    script: "hebrew",
    glyph: "מ",
    strokes: [
      {
        segments: [
          {
            label: "draw the detached left part from its lower tip up to the corner",
            path: [
              { x: 92, y: 45 },
              { x: 115, y: 205 },
              { x: 145, y: 365 },
              { x: 140, y: 555 },
            ],
          },
          {
            label: "turn down-right through its short inner leg without lifting",
            path: [
              { x: 140, y: 555 },
              { x: 150, y: 520 },
              { x: 160, y: 485 },
              { x: 170, y: 450 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then climb diagonally right through the upper shoulder",
            path: [
              { x: 190, y: 440 },
              { x: 235, y: 515 },
              { x: 300, y: 560 },
              { x: 390, y: 560 },
            ],
          },
          {
            label: "turn down the right side without lifting",
            path: [
              { x: 390, y: 560 },
              { x: 470, y: 525 },
              { x: 530, y: 430 },
              { x: 565, y: 315 },
              { x: 550, y: 180 },
              { x: 500, y: 76 },
            ],
          },
          {
            label: "turn left along the base without lifting, stopping before the left part",
            path: [
              { x: 500, y: 76 },
              { x: 430, y: 48 },
              { x: 355, y: 38 },
              { x: 280, y: 38 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("מ"),
  }],
  // Printed Nun joins its small head, right descent, and leftward base in one
  // run. The source's immediately adjacent cursive form rounds the same hook.
    ["hebrew:נ", {
    script: "hebrew",
    glyph: "נ",
    strokes: [
      {
        segments: [
          {
            label: "draw the short top head from left to right",
            path: [
              { x: 105, y: 555 },
              { x: 155, y: 555 },
              { x: 210, y: 540 },
              { x: 255, y: 500 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 255, y: 500 },
              { x: 260, y: 400 },
              { x: 260, y: 280 },
              { x: 260, y: 160 },
              { x: 240, y: 80 },
            ],
          },
          {
            label: "turn left along the base without lifting",
            path: [
              { x: 240, y: 80 },
              { x: 190, y: 55 },
              { x: 120, y: 40 },
              { x: 60, y: 40 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("נ"),
  }],
  // Printed Samekh closes one continuous clockwise loop. The source's
  // immediately adjacent cursive form rounds the same zero-lift movement.
    ["hebrew:ס", {
    script: "hebrew",
    glyph: "ס",
    strokes: [
      {
        segments: [
          {
            label: "draw the flat top from left to right",
            path: [
              { x: 70, y: 555 },
              { x: 170, y: 555 },
              { x: 275, y: 555 },
              { x: 365, y: 550 },
            ],
          },
          {
            label: "round down the right side without lifting",
            path: [
              { x: 365, y: 550 },
              { x: 455, y: 520 },
              { x: 525, y: 430 },
              { x: 550, y: 325 },
              { x: 535, y: 200 },
              { x: 465, y: 90 },
              { x: 365, y: 35 },
            ],
          },
          {
            label: "sweep left along the base without lifting",
            path: [
              { x: 365, y: 35 },
              { x: 285, y: 30 },
              { x: 205, y: 55 },
              { x: 145, y: 115 },
            ],
          },
          {
            label: "climb the left side and close the loop without lifting",
            path: [
              { x: 145, y: 115 },
              { x: 120, y: 210 },
              { x: 120, y: 315 },
              { x: 125, y: 410 },
              { x: 150, y: 490 },
              { x: 120, y: 535 },
              { x: 70, y: 555 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ס"),
  }],
  // Printed Ayin descends its right branch into the base, sweeps left, then
  // turns back to climb the left branch in one uninterrupted run.
    ["hebrew:ע", {
    script: "hebrew",
    glyph: "ע",
    strokes: [
      {
        segments: [
          {
            label: "descend the right branch and curve left into the base",
            path: [
              { x: 500, y: 560 },
              { x: 495, y: 455 },
              { x: 475, y: 335 },
              { x: 440, y: 225 },
              { x: 390, y: 145 },
              { x: 330, y: 85 },
              { x: 250, y: 45 },
            ],
          },
          {
            label: "sweep left along the base without lifting",
            path: [
              { x: 250, y: 45 },
              { x: 190, y: 25 },
              { x: 125, y: 15 },
              { x: 70, y: 10 },
            ],
          },
          {
            label: "turn back and climb the left branch without lifting",
            path: [
              { x: 70, y: 10 },
              { x: 145, y: 35 },
              { x: 205, y: 80 },
              { x: 210, y: 165 },
              { x: 180, y: 285 },
              { x: 150, y: 410 },
              { x: 115, y: 560 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ע"),
  }],
  // Printed Pe draws the outer top, right side, and returning base in one run,
  // then lifts once for its short inner curl. The adjacent cursive form instead
  // coils inward as one rounded spiral.
    ["hebrew:פ", {
    script: "hebrew",
    glyph: "פ",
    strokes: [
      {
        segments: [
          {
            label: "draw the outer top from left to right",
            path: [
              { x: 150, y: 560 },
              { x: 220, y: 570 },
              { x: 286, y: 565 },
              { x: 365, y: 535 },
              { x: 430, y: 475 },
            ],
          },
          {
            label: "turn down the right side without lifting",
            path: [
              { x: 430, y: 475 },
              { x: 475, y: 410 },
              { x: 505, y: 330 },
              { x: 505, y: 260 },
              { x: 480, y: 185 },
              { x: 435, y: 120 },
              { x: 370, y: 75 },
            ],
          },
          {
            label: "return left along the base without lifting",
            path: [
              { x: 370, y: 75 },
              { x: 300, y: 45 },
              { x: 220, y: 35 },
              { x: 140, y: 38 },
              { x: 70, y: 38 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the short inner curl from left to right",
            path: [
              { x: 95, y: 400 },
              { x: 98, y: 350 },
              { x: 120, y: 305 },
              { x: 160, y: 270 },
              { x: 205, y: 250 },
              { x: 252, y: 247 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("פ"),
  }],
  // Printed Tsadi draws its long left diagonal into the returning base, then
  // lifts once for the short upper-right arm. Its cursive counterpart compresses
  // those branches into one compact rounded run.
    ["hebrew:צ", {
    script: "hebrew",
    glyph: "צ",
    strokes: [
      {
        segments: [
          {
            label: "descend the long diagonal from the upper left",
            path: [
              { x: 100, y: 560 },
              { x: 145, y: 505 },
              { x: 195, y: 430 },
              { x: 245, y: 350 },
              { x: 295, y: 270 },
              { x: 345, y: 185 },
              { x: 395, y: 100 },
              { x: 440, y: 40 },
            ],
          },
          {
            label: "turn left along the base without lifting",
            path: [
              { x: 440, y: 40 },
              { x: 350, y: 38 },
              { x: 250, y: 38 },
              { x: 150, y: 38 },
              { x: 55, y: 38 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then curve the upper-right arm down-left into the junction",
            path: [
              { x: 460, y: 560 },
              { x: 458, y: 505 },
              { x: 448, y: 450 },
              { x: 430, y: 390 },
              { x: 405, y: 335 },
              { x: 375, y: 285 },
              { x: 345, y: 260 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("צ"),
  }],
  // Printed Qof keeps the top and slanted right body in one run, then lifts
  // once for the separate descending stem. Its cursive counterpart rounds the
  // same idea into one continuous hooked descent.
    ["hebrew:ק", {
    script: "hebrew",
    glyph: "ק",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 85, y: 555 },
              { x: 180, y: 555 },
              { x: 280, y: 555 },
              { x: 380, y: 555 },
              { x: 470, y: 555 },
              { x: 560, y: 555 },
            ],
          },
          {
            label: "turn down-left through the right body without lifting",
            path: [
              { x: 560, y: 555 },
              { x: 545, y: 520 },
              { x: 520, y: 460 },
              { x: 500, y: 400 },
              { x: 480, y: 335 },
              { x: 455, y: 260 },
              { x: 430, y: 180 },
              { x: 405, y: 100 },
              { x: 375, y: 10 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the separate inner-left stem below the line",
            path: [
              { x: 140, y: 360 },
              { x: 140, y: 275 },
              { x: 140, y: 180 },
              { x: 140, y: 80 },
              { x: 140, y: -20 },
              { x: 140, y: -105 },
              { x: 140, y: -180 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ק"),
  }],
  // Printed Resh carries its short top bar directly around the rounded corner
  // and down the right side. The cursive form keeps the same zero-lift hook.
    ["hebrew:ר", {
    script: "hebrew",
    glyph: "ר",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 55, y: 555 },
              { x: 105, y: 555 },
              { x: 155, y: 555 },
              { x: 205, y: 555 },
              { x: 250, y: 555 },
            ],
          },
          {
            label: "round the top-right corner and continue down without lifting",
            path: [
              { x: 250, y: 555 },
              { x: 305, y: 550 },
              { x: 350, y: 530 },
              { x: 385, y: 495 },
              { x: 400, y: 445 },
              { x: 400, y: 350 },
              { x: 400, y: 250 },
              { x: 400, y: 140 },
              { x: 400, y: 10 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ר"),
  }],
  // Printed Shin draws its outer right-base-left bowl in one run, then lifts
  // once for the middle branch. The adjacent purple cursive form compresses
  // those parts into a single rounded loop with a short rightward exit.
    ["hebrew:ש", {
    script: "hebrew",
    glyph: "ש",
    strokes: [
      {
        segments: [
          {
            label: "descend the right branch and round left along the base",
            path: [
              { x: 620, y: 570 },
              { x: 620, y: 500 },
              { x: 620, y: 420 },
              { x: 620, y: 340 },
              { x: 610, y: 250 },
              { x: 580, y: 170 },
              { x: 530, y: 100 },
              { x: 470, y: 60 },
              { x: 400, y: 35 },
              { x: 330, y: 32 },
              { x: 260, y: 45 },
              { x: 200, y: 80 },
              { x: 160, y: 135 },
            ],
          },
          {
            label: "continue up the left branch without lifting",
            path: [
              { x: 160, y: 135 },
              { x: 135, y: 200 },
              { x: 110, y: 280 },
              { x: 110, y: 380 },
              { x: 110, y: 480 },
              { x: 110, y: 570 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the middle branch into the base",
            path: [
              { x: 365, y: 570 },
              { x: 365, y: 500 },
              { x: 365, y: 430 },
              { x: 355, y: 365 },
              { x: 330, y: 320 },
              { x: 295, y: 285 },
              { x: 250, y: 260 },
              { x: 205, y: 250 },
              { x: 165, y: 250 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ש"),
  }],
  // Printed Tav joins its top bar to the right side, then lifts once for the
  // separate left leg and foot. The purple cursive form instead retraces its
  // left stem and arches into the right side in one continuous run.
    ["hebrew:ת", {
    script: "hebrew",
    glyph: "ת",
    strokes: [
      {
        segments: [
          {
            label: "draw the top bar from left to right",
            path: [
              { x: 65, y: 555 },
              { x: 130, y: 555 },
              { x: 210, y: 555 },
              { x: 300, y: 555 },
              { x: 390, y: 555 },
              { x: 430, y: 550 },
            ],
          },
          {
            label: "continue down the right side without lifting",
            path: [
              { x: 430, y: 550 },
              { x: 490, y: 535 },
              { x: 535, y: 500 },
              { x: 560, y: 450 },
              { x: 565, y: 380 },
              { x: 565, y: 280 },
              { x: 565, y: 170 },
              { x: 565, y: 70 },
              { x: 565, y: 20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the separate left leg",
            path: [
              { x: 195, y: 520 },
              { x: 195, y: 450 },
              { x: 195, y: 360 },
              { x: 195, y: 270 },
              { x: 195, y: 180 },
              { x: 185, y: 120 },
            ],
          },
          {
            label: "curve left into the small foot without lifting",
            path: [
              { x: 185, y: 120 },
              { x: 165, y: 80 },
              { x: 135, y: 50 },
              { x: 100, y: 38 },
              { x: 70, y: 42 },
              { x: 50, y: 55 },
            ],
          },
        ],
      },
    ],
    source: hebrewAlphabetSource("ת"),
  }],
];
