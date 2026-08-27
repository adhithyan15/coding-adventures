// Authored cyrillic ductus records. This is the stable source-ownership boundary.

import type { LetterDuctus, Point, Stroke, StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import cyrillic from "../../../../../learning/human-languages/data/scripts/cyrillic.json";

const cyrillicAlphabetSource = (glyph: string): StrokeSource => {
  const letter = cyrillic.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Cyrillic ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  // The native-teacher demonstration keeps lowercase а in one pen-down run:
  // its rounded body closes at the right and flows into the finishing stem.
  // Noto Sans Cyrillic uses a double-storey printed outline, so the source's
  // opening loop is fitted through the font's extra shoulder without adding a
  // lift that the handwriting demonstration does not contain.
    ["cyrillic:а", {
    script: "cyrillic",
    glyph: "а",
    strokes: [
      {
        segments: [
          {
            label: "sweep over the shoulder and around the round body",
            path: [
              { x: 110, y: 455 }, { x: 155, y: 495 }, { x: 215, y: 515 },
              { x: 285, y: 515 }, { x: 345, y: 490 }, { x: 395, y: 445 },
              { x: 420, y: 395 }, { x: 420, y: 345 }, { x: 390, y: 305 },
              { x: 325, y: 300 }, { x: 250, y: 300 }, { x: 175, y: 300 },
              { x: 145, y: 285 }, { x: 95, y: 245 }, { x: 75, y: 190 },
              { x: 85, y: 125 }, { x: 120, y: 70 }, { x: 175, y: 35 },
              { x: 240, y: 25 }, { x: 305, y: 45 }, { x: 355, y: 85 },
              { x: 395, y: 140 }, { x: 420, y: 205 }, { x: 420, y: 265 },
              { x: 420, y: 305 },
            ],
          },
          {
            label: "continue down the right-hand finishing stem",
            path: [
              { x: 420, y: 305 }, { x: 430, y: 250 }, { x: 435, y: 190 },
              { x: 435, y: 125 }, { x: 435, y: 65 }, { x: 440, y: 20 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("а"),
  }],
  // RussianIrina closes the rounded lowercase б body and immediately climbs
  // into its top flag. The source's direct diagonal crossing is a handwritten
  // form; Noto Sans Cyrillic joins the ascender at the upper-left shoulder, so
  // this one-run fit carries the pen left along the printed shoulder first.
    ["cyrillic:б", {
    script: "cyrillic",
    glyph: "б",
    strokes: [
      {
        segments: [
          {
            label: "circle counterclockwise around the rounded lower body",
            path: [
              { x: 430, y: 480 }, { x: 350, y: 495 }, { x: 270, y: 495 },
              { x: 190, y: 470 }, { x: 125, y: 420 }, { x: 90, y: 340 },
              { x: 85, y: 250 }, { x: 105, y: 165 }, { x: 155, y: 95 },
              { x: 225, y: 45 }, { x: 305, y: 25 }, { x: 380, y: 45 },
              { x: 445, y: 90 }, { x: 490, y: 155 }, { x: 510, y: 235 },
              { x: 505, y: 315 }, { x: 480, y: 400 }, { x: 430, y: 480 },
            ],
          },
          {
            label: "continue through the rising shoulder and sweep the top flag right",
            path: [
              { x: 430, y: 480 }, { x: 350, y: 490 }, { x: 270, y: 490 },
              { x: 195, y: 485 }, { x: 195, y: 465 }, { x: 195, y: 450 },
              { x: 170, y: 445 }, { x: 140, y: 445 }, { x: 140, y: 480 },
              { x: 140, y: 520 }, { x: 150, y: 545 }, { x: 165, y: 585 },
              { x: 185, y: 620 }, { x: 215, y: 660 }, { x: 260, y: 690 },
              { x: 320, y: 710 }, { x: 390, y: 720 }, { x: 460, y: 730 },
              { x: 500, y: 735 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("б"),
  }],
  // RussianIrina starts lowercase в at the baseline, climbs through its tall
  // handwritten ascender loop, descends to the baseline, and continues around
  // the lower bowl without lifting. Noto Sans Cyrillic prints two compact bowls
  // on a straight stem, so the same one-run order is fitted through the upper
  // bowl, down the stem, and counterclockwise around the lower bowl.
    ["cyrillic:в", {
    script: "cyrillic",
    glyph: "в",
    strokes: [
      {
        segments: [
          {
            label: "climb through the upper loop and descend to the baseline",
            path: [
              { x: 130, y: 20 }, { x: 130, y: 100 }, { x: 130, y: 200 },
              { x: 130, y: 300 }, { x: 130, y: 400 }, { x: 130, y: 500 },
              { x: 220, y: 500 }, { x: 310, y: 500 }, { x: 380, y: 480 },
              { x: 430, y: 445 }, { x: 455, y: 400 }, { x: 450, y: 355 },
              { x: 420, y: 320 }, { x: 365, y: 300 }, { x: 295, y: 290 },
              { x: 220, y: 290 }, { x: 150, y: 290 }, { x: 130, y: 260 },
              { x: 130, y: 180 }, { x: 130, y: 100 }, { x: 130, y: 20 },
            ],
          },
          {
            label: "continue counterclockwise around the rounded lower bowl",
            path: [
              { x: 130, y: 20 }, { x: 220, y: 35 }, { x: 310, y: 35 },
              { x: 385, y: 50 }, { x: 440, y: 80 }, { x: 470, y: 120 },
              { x: 475, y: 165 }, { x: 455, y: 205 }, { x: 415, y: 235 },
              { x: 360, y: 260 }, { x: 295, y: 270 }, { x: 220, y: 270 },
              { x: 150, y: 270 }, { x: 130, y: 260 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("в"),
  }],
  // RussianIrina writes lowercase г as one rounded two-hump cursive run. The
  // bundled Noto glyph is the block-like isolated form, so its zero-lift order
  // is preserved by climbing the upright, sweeping and retracing the top bar,
  // then descending the upright. Connected cursive restores the exit hump.
    ["cyrillic:г", {
    script: "cyrillic",
    glyph: "г",
    strokes: [
      {
        segments: [
          {
            label: "climb the upright and sweep the top bar right",
            path: [
              { x: 130, y: 20 }, { x: 130, y: 120 }, { x: 130, y: 240 },
              { x: 130, y: 360 }, { x: 130, y: 500 }, { x: 220, y: 500 },
              { x: 310, y: 500 }, { x: 390, y: 500 },
            ],
          },
          {
            label: "reverse along the top and descend to the baseline",
            path: [
              { x: 390, y: 500 }, { x: 310, y: 500 }, { x: 220, y: 500 },
              { x: 130, y: 500 }, { x: 130, y: 360 }, { x: 130, y: 240 },
              { x: 130, y: 120 }, { x: 130, y: 20 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("г"),
  }],
  // RussianIrina writes lowercase д as one cursive body-to-descender run. The
  // bundled Noto glyph is the block-like isolated form, so its zero-lift order
  // is preserved by circling the body before retracing both feet through their
  // joined base shelf. Connected cursive restores the below-baseline loop.
    ["cyrillic:д", {
    script: "cyrillic",
    glyph: "д",
    strokes: [
      {
        segments: [
          {
            label: "circle counterclockwise around the closed body",
            path: [
              { x: 470, y: 462 }, { x: 390, y: 500 }, { x: 300, y: 500 },
              { x: 205, y: 500 }, { x: 190, y: 420 }, { x: 185, y: 330 },
              { x: 175, y: 240 }, { x: 150, y: 150 }, { x: 110, y: 74 },
              { x: 190, y: 35 }, { x: 290, y: 35 }, { x: 390, y: 35 },
              { x: 470, y: 74 }, { x: 470, y: 170 }, { x: 470, y: 270 },
              { x: 470, y: 370 }, { x: 470, y: 462 },
            ],
          },
          {
            label: "descend, retrace both feet, and finish along the base shelf",
            path: [
              { x: 470, y: 462 }, { x: 470, y: 330 }, { x: 470, y: 200 },
              { x: 470, y: 74 }, { x: 550, y: 35 }, { x: 550, y: -50 },
              { x: 550, y: -110 }, { x: 550, y: -50 }, { x: 550, y: 35 },
              { x: 450, y: 35 }, { x: 350, y: 35 }, { x: 250, y: 35 },
              { x: 150, y: 35 }, { x: 55, y: 35 }, { x: 55, y: -50 },
              { x: 55, y: -110 }, { x: 55, y: -50 }, { x: 55, y: 35 },
              { x: 150, y: 35 }, { x: 250, y: 35 }, { x: 350, y: 35 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("д"),
  }],
  // RussianIrina writes lowercase е as one upper-loop-to-lower-bowl cursive
  // run. Noto Sans Cyrillic prints a compact e with a long middle bar, so the
  // sourced zero-lift order is fitted by sweeping and reversing through that
  // bar before continuing counterclockwise around the lower bowl.
    ["cyrillic:е", {
    script: "cyrillic",
    glyph: "е",
    strokes: [
      {
        segments: [
          {
            label: "curve around the upper bowl and sweep through the middle",
            path: [
              { x: 430, y: 380 }, { x: 390, y: 455 }, { x: 320, y: 505 },
              { x: 245, y: 505 }, { x: 175, y: 470 }, { x: 115, y: 410 },
              { x: 85, y: 340 }, { x: 85, y: 285 }, { x: 150, y: 285 },
              { x: 240, y: 285 }, { x: 330, y: 285 }, { x: 440, y: 285 },
            ],
          },
          {
            label: "reverse through the middle and circle the lower bowl",
            path: [
              { x: 440, y: 285 }, { x: 330, y: 285 }, { x: 240, y: 285 },
              { x: 150, y: 285 }, { x: 85, y: 260 }, { x: 80, y: 185 },
              { x: 105, y: 115 }, { x: 160, y: 55 }, { x: 230, y: 25 },
              { x: 305, y: 25 }, { x: 375, y: 40 }, { x: 440, y: 70 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("е"),
  }],
  // RussianIrina writes lowercase ё by completing the same looped body as е,
  // then lifting for the left dot and once more for the right dot. The Noto
  // Sans Cyrillic fit reuses the printed e route and places both circular dots
  // as separate runs in the demonstrated left-to-right order.
    ["cyrillic:ё", {
    script: "cyrillic",
    glyph: "ё",
    strokes: [
      {
        segments: [
          {
            label: "curve around the upper bowl and sweep through the middle",
            path: [
              { x: 430, y: 380 }, { x: 390, y: 455 }, { x: 320, y: 505 },
              { x: 245, y: 505 }, { x: 175, y: 470 }, { x: 115, y: 410 },
              { x: 85, y: 340 }, { x: 85, y: 285 }, { x: 150, y: 285 },
              { x: 240, y: 285 }, { x: 330, y: 285 }, { x: 440, y: 285 },
            ],
          },
          {
            label: "reverse through the middle and circle the lower bowl",
            path: [
              { x: 440, y: 285 }, { x: 330, y: 285 }, { x: 240, y: 285 },
              { x: 150, y: 285 }, { x: 85, y: 260 }, { x: 80, y: 185 },
              { x: 105, y: 115 }, { x: 160, y: 55 }, { x: 230, y: 25 },
              { x: 305, y: 25 }, { x: 375, y: 40 }, { x: 440, y: 70 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift and place the left dot",
            path: [
              { x: 197, y: 674 }, { x: 203, y: 674 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift again and place the right dot",
            path: [
              { x: 379, y: 674 }, { x: 385, y: 674 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ё"),
  }],
  // RussianIrina writes lowercase ж as one continuous rounded left-to-centre-
  // to-right run. Noto Sans Cyrillic prints a straight upright with four arms,
  // so the sourced zero-lift order is fitted by retracing each side junction
  // and the central upright before continuing into the opposite wing.
    ["cyrillic:ж", {
    script: "cyrillic",
    glyph: "ж",
    strokes: [
      {
        segments: [
          {
            label: "trace the left wings and rise through the centre",
            path: [
              { x: 60, y: 30 }, { x: 110, y: 100 }, { x: 190, y: 220 },
              { x: 265, y: 275 }, { x: 210, y: 350 }, { x: 140, y: 455 },
              { x: 75, y: 510 }, { x: 140, y: 455 }, { x: 210, y: 350 },
              { x: 265, y: 275 }, { x: 340, y: 275 }, { x: 380, y: 275 },
              { x: 380, y: 380 }, { x: 380, y: 510 }, { x: 380, y: 380 },
              { x: 380, y: 275 }, { x: 380, y: 150 }, { x: 380, y: 30 },
              { x: 380, y: 150 }, { x: 380, y: 275 },
            ],
          },
          {
            label: "retrace the centre and trace the right wings",
            path: [
              { x: 380, y: 275 }, { x: 495, y: 275 }, { x: 560, y: 370 },
              { x: 630, y: 470 },
              { x: 690, y: 510 }, { x: 630, y: 470 }, { x: 560, y: 370 },
              { x: 495, y: 275 }, { x: 560, y: 180 }, { x: 630, y: 80 },
              { x: 700, y: 30 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ж"),
  }],
  // RussianIrina writes lowercase з as one continuous smaller-upper-lobe to
  // larger-lower-lobe run with a cursive exit. Noto Sans Cyrillic omits the
  // exit, so the sourced zero-lift order is fitted by circling both printed
  // lobes through their middle junction and retracing to the lower-right tip.
    ["cyrillic:з", {
    script: "cyrillic",
    glyph: "з",
    strokes: [
      {
        segments: [
          {
            label: "circle the smaller upper lobe and descend through the middle",
            path: [
              { x: 80, y: 485 }, { x: 155, y: 510 }, { x: 225, y: 510 },
              { x: 300, y: 500 }, { x: 365, y: 460 }, { x: 390, y: 410 },
              { x: 385, y: 360 }, { x: 345, y: 320 }, { x: 285, y: 285 },
              { x: 200, y: 280 },
            ],
          },
          {
            label: "circle the larger lower lobe and finish at the lower right",
            path: [
              { x: 200, y: 280 }, { x: 285, y: 275 }, { x: 360, y: 245 },
              { x: 405, y: 200 }, { x: 410, y: 145 }, { x: 385, y: 90 },
              { x: 325, y: 45 }, { x: 245, y: 25 }, { x: 160, y: 25 },
              { x: 85, y: 45 }, { x: 160, y: 25 }, { x: 245, y: 25 },
              { x: 325, y: 45 }, { x: 405, y: 75 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("з"),
  }],
  // RussianIrina writes lowercase и as one continuous left-stem, rising-
  // diagonal, right-stem run with cursive entry and exit joins. The bundled
  // printed glyph omits those joins, so the sourced zero-lift order is fitted
  // directly through its two stems and backwards-N diagonal.
    ["cyrillic:и", {
    script: "cyrillic",
    glyph: "и",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 125, y: 510 }, { x: 125, y: 390 }, { x: 125, y: 270 },
              { x: 125, y: 150 }, { x: 125, y: 25 }, { x: 160, y: 25 },
              { x: 190, y: 40 },
            ],
          },
          {
            label: "rise diagonally to the upper right",
            path: [
              { x: 190, y: 40 }, { x: 225, y: 100 }, { x: 270, y: 180 },
              { x: 315, y: 255 }, { x: 360, y: 335 }, { x: 405, y: 410 },
              { x: 450, y: 485 }, { x: 475, y: 510 },
            ],
          },
          {
            label: "descend the right stem and finish at the baseline",
            path: [
              { x: 475, y: 510 }, { x: 475, y: 390 }, { x: 475, y: 270 },
              { x: 475, y: 150 }, { x: 475, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("и"),
  }],
  // RussianIrina writes the lowercase й body with the same continuous motion
  // as и, then lifts once and adds its breve from left to right. The fitted
  // path preserves that body-before-breve order across the bundled printed
  // backwards-N body and its separate curved mark.
    ["cyrillic:й", {
    script: "cyrillic",
    glyph: "й",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 125, y: 510 }, { x: 125, y: 390 }, { x: 125, y: 270 },
              { x: 125, y: 150 }, { x: 125, y: 25 }, { x: 160, y: 25 },
              { x: 190, y: 40 },
            ],
          },
          {
            label: "rise diagonally to the upper right",
            path: [
              { x: 190, y: 40 }, { x: 225, y: 100 }, { x: 270, y: 180 },
              { x: 315, y: 255 }, { x: 360, y: 335 }, { x: 405, y: 410 },
              { x: 450, y: 485 }, { x: 475, y: 510 },
            ],
          },
          {
            label: "descend the right stem and finish at the baseline",
            path: [
              { x: 475, y: 510 }, { x: 475, y: 390 }, { x: 475, y: 270 },
              { x: 475, y: 150 }, { x: 475, y: 25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the breve from left to right",
            path: [
              { x: 195, y: 715 }, { x: 220, y: 675 }, { x: 265, y: 640 },
              { x: 310, y: 635 }, { x: 355, y: 640 }, { x: 400, y: 675 },
              { x: 430, y: 715 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("й"),
  }],
  // RussianIrina writes lowercase к in one continuous school-hand motion:
  // descend the left stem, rise into the upper arm and return to the middle,
  // then continue through the lower arm. The fitted path preserves that order
  // while tracing the bundled printed vertical and its two angular diagonals.
    ["cyrillic:к", {
    script: "cyrillic",
    glyph: "к",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 510 },
              { x: 129, y: 390 },
              { x: 129, y: 270 },
              { x: 129, y: 150 },
              { x: 129, y: 25 },
            ],
          },
          {
            label: "rise through the upper arm and return to the middle junction",
            path: [
              { x: 129, y: 25 },
              { x: 129, y: 120 },
              { x: 129, y: 220 },
              { x: 190, y: 274 },
              { x: 250, y: 310 },
              { x: 370, y: 400 },
              { x: 435, y: 490 },
              { x: 465, y: 510 },
              { x: 420, y: 470 },
              { x: 360, y: 400 },
              { x: 300, y: 320 },
              { x: 250, y: 274 },
              { x: 190, y: 274 },
            ],
          },
          {
            label: "continue down-right through the lower arm to the baseline",
            path: [
              { x: 190, y: 274 },
              { x: 250, y: 250 },
              { x: 300, y: 210 },
              { x: 350, y: 150 },
              { x: 410, y: 70 },
              { x: 475, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("к"),
  }],
  // RussianIrina writes lowercase л in one continuous school-hand motion:
  // curl around the baseline hook, rise to the apex, then descend the right
  // leg. The fitted path preserves that order while tracing the bundled
  // printed glyph's curved left leg, horizontal shoulder, and upright stem.
    ["cyrillic:л", {
    script: "cyrillic",
    glyph: "л",
    strokes: [
      {
        segments: [
          {
            label: "curve from the baseline hook up the left leg",
            path: [
              { x: 25, y: 30 },
              { x: 55, y: 25 },
              { x: 85, y: 35 },
              { x: 110, y: 75 },
              { x: 125, y: 140 },
              { x: 135, y: 240 },
              { x: 145, y: 360 },
              { x: 160, y: 460 },
              { x: 175, y: 500 },
            ],
          },
          {
            label: "sweep right along the top shoulder",
            path: [
              { x: 175, y: 500 },
              { x: 260, y: 500 },
              { x: 360, y: 500 },
              { x: 458, y: 500 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 458, y: 500 },
              { x: 458, y: 380 },
              { x: 458, y: 260 },
              { x: 458, y: 140 },
              { x: 458, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("л"),
  }],
  // RussianIrina writes lowercase м in one continuous school-hand motion:
  // rise from the entry hook to the first apex, descend and rise through the
  // second arch, then descend the right leg. The fitted path preserves that
  // order while tracing the bundled printed stems and deep central V.
    ["cyrillic:м", {
    script: "cyrillic",
    glyph: "м",
    strokes: [
      {
        segments: [
          {
            label: "rise from the baseline through the left stem",
            path: [
              { x: 126, y: 25 },
              { x: 126, y: 140 },
              { x: 126, y: 260 },
              { x: 126, y: 380 },
              { x: 126, y: 500 },
              { x: 185, y: 500 },
            ],
          },
          {
            label: "descend diagonally to the central valley",
            path: [
              { x: 185, y: 500 },
              { x: 230, y: 400 },
              { x: 275, y: 290 },
              { x: 325, y: 170 },
              { x: 380, y: 50 },
            ],
          },
          {
            label: "rise diagonally to the second apex",
            path: [
              { x: 380, y: 50 },
              { x: 435, y: 170 },
              { x: 490, y: 290 },
              { x: 535, y: 400 },
              { x: 585, y: 500 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 585, y: 500 },
              { x: 642, y: 500 },
              { x: 642, y: 380 },
              { x: 642, y: 260 },
              { x: 642, y: 140 },
              { x: 642, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("м"),
  }],
  // RussianIrina writes lowercase н in one continuous school-hand motion:
  // descend the left stem, turn upward through the rounded middle bridge,
  // rise to the right shoulder, then descend the right stem. The fitted path
  // preserves that order across the bundled printed stems and middle bar.
    ["cyrillic:н", {
    script: "cyrillic",
    glyph: "н",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 510 },
              { x: 129, y: 390 },
              { x: 129, y: 274 },
              { x: 129, y: 150 },
              { x: 129, y: 25 },
            ],
          },
          {
            label: "retrace to the middle bridge and rise to the upper right",
            path: [
              { x: 129, y: 25 },
              { x: 129, y: 140 },
              { x: 129, y: 274 },
              { x: 220, y: 274 },
              { x: 310, y: 274 },
              { x: 400, y: 274 },
              { x: 485, y: 274 },
              { x: 485, y: 390 },
              { x: 485, y: 510 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 485, y: 510 },
              { x: 485, y: 390 },
              { x: 485, y: 274 },
              { x: 485, y: 150 },
              { x: 485, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("н"),
  }],
  // RussianIrina writes lowercase о as one continuous counterclockwise oval:
  // begin on the upper-right shoulder, pass over the top and down the left,
  // sweep through the bottom, then rise on the right and close. The fitted
  // path preserves that order in the bundled printed oval.
    ["cyrillic:о", {
    script: "cyrillic",
    glyph: "о",
    strokes: [
      {
        segments: [
          {
            label: "curve left over the top and descend the left side",
            path: [
              { x: 430, y: 450 },
              { x: 380, y: 490 },
              { x: 300, y: 510 },
              { x: 220, y: 500 },
              { x: 145, y: 450 },
              { x: 105, y: 380 },
              { x: 95, y: 270 },
              { x: 105, y: 170 },
              { x: 150, y: 90 },
              { x: 220, y: 40 },
              { x: 300, y: 25 },
            ],
          },
          {
            label: "sweep through the bottom and rise to close the oval",
            path: [
              { x: 300, y: 25 },
              { x: 380, y: 40 },
              { x: 450, y: 90 },
              { x: 490, y: 170 },
              { x: 500, y: 268 },
              { x: 490, y: 360 },
              { x: 450, y: 440 },
              { x: 380, y: 490 },
              { x: 300, y: 510 },
              { x: 380, y: 490 },
              { x: 430, y: 450 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("о"),
  }],
  // RussianIrina writes lowercase п in one continuous school-hand motion:
  // descend the left stem, turn upward into the rounded top shoulder, then
  // descend the right stem. The fitted path preserves that order across the
  // bundled printed stems and horizontal top bar.
    ["cyrillic:п", {
    script: "cyrillic",
    glyph: "п",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 500 },
              { x: 129, y: 380 },
              { x: 129, y: 260 },
              { x: 129, y: 140 },
              { x: 129, y: 25 },
            ],
          },
          {
            label: "retrace to the top shoulder and sweep right",
            path: [
              { x: 129, y: 25 },
              { x: 129, y: 140 },
              { x: 129, y: 260 },
              { x: 129, y: 380 },
              { x: 129, y: 500 },
              { x: 220, y: 500 },
              { x: 310, y: 500 },
              { x: 400, y: 500 },
              { x: 477, y: 500 },
            ],
          },
          {
            label: "descend the right stem to the baseline",
            path: [
              { x: 477, y: 500 },
              { x: 477, y: 380 },
              { x: 477, y: 260 },
              { x: 477, y: 140 },
              { x: 477, y: 25 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("п"),
  }],
  // RussianIrina writes lowercase р in one continuous school-hand motion:
  // descend below the baseline, retrace upward, then curve through the rounded
  // shoulder and baseline exit. The fitted path preserves that stem-before-bowl
  // order while closing the bowl around the bundled printed outline.
    ["cyrillic:р", {
    script: "cyrillic",
    glyph: "р",
    strokes: [
      {
        segments: [
          {
            label: "descend the stem below the baseline",
            path: [
              { x: 129, y: 510 },
              { x: 129, y: 350 },
              { x: 129, y: 190 },
              { x: 129, y: 30 },
              { x: 129, y: -100 },
              { x: 129, y: -200 },
            ],
          },
          {
            label: "retrace to the upper shoulder and curve right",
            path: [
              { x: 129, y: -200 },
              { x: 129, y: -80 },
              { x: 129, y: 40 },
              { x: 129, y: 180 },
              { x: 129, y: 320 },
              { x: 129, y: 450 },
              { x: 157, y: 450 },
              { x: 173, y: 463 },
              { x: 190, y: 475 },
              { x: 220, y: 490 },
              { x: 280, y: 510 },
              { x: 370, y: 500 },
              { x: 450, y: 450 },
              { x: 500, y: 370 },
              { x: 515, y: 270 },
            ],
          },
          {
            label: "sweep around the bowl and return to the stem",
            path: [
              { x: 515, y: 270 },
              { x: 505, y: 170 },
              { x: 455, y: 90 },
              { x: 380, y: 40 },
              { x: 300, y: 25 },
              { x: 230, y: 45 },
              { x: 185, y: 100 },
              { x: 177, y: 175 },
              { x: 165, y: 220 },
              { x: 150, y: 269 },
              { x: 129, y: 269 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("р"),
  }],
  // RussianIrina writes lowercase с in one continuous counterclockwise motion:
  // curve from the upper-right tip across the top, descend the left side, then
  // sweep through the bottom into the lower-right exit. The fitted path keeps
  // that open-curve order across the bundled wider printed outline.
    ["cyrillic:с", {
    script: "cyrillic",
    glyph: "с",
    strokes: [
      {
        segments: [
          {
            label: "curve left over the top and descend the left side",
            path: [
              { x: 438, y: 480 },
              { x: 380, y: 510 },
              { x: 306, y: 509 },
              { x: 230, y: 500 },
              { x: 160, y: 450 },
              { x: 110, y: 380 },
              { x: 94, y: 263 },
            ],
          },
          {
            label: "sweep through the bottom and rise to the lower-right tip",
            path: [
              { x: 94, y: 263 },
              { x: 100, y: 160 },
              { x: 145, y: 80 },
              { x: 220, y: 35 },
              { x: 307, y: 27 },
              { x: 380, y: 35 },
              { x: 439, y: 51 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("с"),
  }],
  // RussianIrina writes lowercase т as one joined, rounded m-like school-hand
  // run: descend the first stem, pass through two arches, then descend and exit.
  // The fitted path preserves the initial descent and zero lifts while routing
  // that continuous motion through the bundled printed central stem and top bar.
    ["cyrillic:т", {
    script: "cyrillic",
    glyph: "т",
    strokes: [
      {
        segments: [
          {
            label: "descend the central stem to the baseline",
            path: [
              { x: 231, y: 499 },
              { x: 231, y: 380 },
              { x: 231, y: 260 },
              { x: 231, y: 140 },
              { x: 231, y: 25 },
            ],
          },
          {
            label: "retrace to the top junction and sweep left",
            path: [
              { x: 231, y: 25 },
              { x: 231, y: 140 },
              { x: 231, y: 260 },
              { x: 231, y: 380 },
              { x: 231, y: 499 },
              { x: 150, y: 499 },
              { x: 52, y: 499 },
            ],
          },
          {
            label: "retrace through the junction and sweep to the right tip",
            path: [
              { x: 52, y: 499 },
              { x: 150, y: 499 },
              { x: 231, y: 499 },
              { x: 320, y: 499 },
              { x: 413, y: 499 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("т"),
  }],
  // RussianIrina writes lowercase у as one joined y-like school-hand run:
  // descend the left arm, rise through the right arm, then retrace into a
  // looped descender and exit. The fitted path preserves that zero-lift order
  // while following the printed arms and its unlooped left-curving terminal.
    ["cyrillic:у", {
    script: "cyrillic",
    glyph: "у",
    strokes: [
      {
        segments: [
          {
            label: "descend the left arm to the middle junction",
            path: [
              { x: 47, y: 500 },
              { x: 80, y: 430 },
              { x: 115, y: 340 },
              { x: 155, y: 240 },
              { x: 215, y: 100 },
            ],
          },
          {
            label: "turn and rise through the right arm",
            path: [
              { x: 215, y: 100 },
              { x: 260, y: 95 },
              { x: 315, y: 220 },
              { x: 365, y: 350 },
              { x: 460, y: 500 },
            ],
          },
          {
            label: "retrace to the junction and descend below the baseline",
            path: [
              { x: 460, y: 500 },
              { x: 410, y: 400 },
              { x: 360, y: 270 },
              { x: 310, y: 145 },
              { x: 260, y: 60 },
              { x: 235, y: -40 },
              { x: 220, y: -85 },
            ],
          },
          {
            label: "curve left through the descender terminal",
            path: [
              { x: 220, y: -85 },
              { x: 205, y: -125 },
              { x: 175, y: -165 },
              { x: 135, y: -195 },
              { x: 85, y: -200 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("у"),
  }],
  // RussianIrina writes lowercase ф in two runs: the long central stem first,
  // then, after one lift, a linked left-loop-to-right-loop body. The fitted
  // path preserves that order while expanding the loops into the printed bowls.
    ["cyrillic:ф", {
    script: "cyrillic",
    glyph: "ф",
    strokes: [
      {
        segments: [
          {
            label: "descend the long central stem below the baseline",
            path: [
              { x: 368, y: 720 },
              { x: 368, y: 500 },
              { x: 368, y: 265 },
              { x: 368, y: 0 },
              { x: 368, y: -200 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift and curve over and around the left bowl",
            path: [
              { x: 368, y: 265 },
              { x: 368, y: 470 },
              { x: 325, y: 500 },
              { x: 240, y: 505 },
              { x: 160, y: 440 },
              { x: 95, y: 350 },
              { x: 95, y: 265 },
            ],
          },
          {
            label: "sweep through the lower-left curve to the centre",
            path: [
              { x: 95, y: 265 },
              { x: 95, y: 180 },
              { x: 160, y: 95 },
              { x: 240, y: 35 },
              { x: 325, y: 25 },
              { x: 368, y: 25 },
            ],
          },
          {
            label: "continue through the lower-right curve",
            path: [
              { x: 368, y: 25 },
              { x: 411, y: 25 },
              { x: 500, y: 35 },
              { x: 580, y: 100 },
              { x: 640, y: 180 },
              { x: 640, y: 265 },
            ],
          },
          {
            label: "rise over the right bowl to the upper junction",
            path: [
              { x: 640, y: 265 },
              { x: 640, y: 350 },
              { x: 580, y: 430 },
              { x: 500, y: 500 },
              { x: 411, y: 500 },
              { x: 368, y: 470 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ф"),
  }],
  // RussianIrina writes lowercase х as two facing top-to-bottom curves: the
  // left run first, then the right run after one lift. The fitted path keeps
  // that run order while straightening the curves into the printed X arms.
    ["cyrillic:х", {
    script: "cyrillic",
    glyph: "х",
    strokes: [
      {
        segments: [
          {
            label: "descend from the upper-left tip to the central crossing",
            path: [
              { x: 68, y: 536 },
              { x: 160, y: 408 },
              { x: 256, y: 274 },
            ],
          },
          {
            label: "sweep down-left from the crossing to the lower-left tip",
            path: [
              { x: 256, y: 274 },
              { x: 158, y: 138 },
              { x: 58, y: 0 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift and descend from the upper-right tip to the crossing",
            path: [
              { x: 442, y: 536 },
              { x: 350, y: 408 },
              { x: 256, y: 274 },
            ],
          },
          {
            label: "sweep down-right from the crossing to the lower-right tip",
            path: [
              { x: 256, y: 274 },
              { x: 354, y: 138 },
              { x: 452, y: 0 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("х"),
  }],
  // RussianIrina writes lowercase ц in one run: left stem down, joined rise
  // and descent through the right stem, then the tail. The fitted path squares
  // those joins and keeps the printed descender connected by retracing.
    ["cyrillic:ц", {
    script: "cyrillic",
    glyph: "ц",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 130, y: 536 },
              { x: 130, y: 280 },
              { x: 130, y: 37 },
            ],
          },
          {
            label: "sweep along the base and rise through the right stem",
            path: [
              { x: 130, y: 37 },
              { x: 300, y: 37 },
              { x: 477, y: 37 },
              { x: 477, y: 280 },
              { x: 477, y: 536 },
            ],
          },
          {
            label: "retrace the right stem and cross the tail shoulder",
            path: [
              { x: 477, y: 536 },
              { x: 477, y: 280 },
              { x: 477, y: 37 },
              { x: 560, y: 37 },
            ],
          },
          {
            label: "descend the short tail below the baseline",
            path: [
              { x: 560, y: 37 },
              { x: 560, y: -50 },
              { x: 560, y: -140 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ц"),
  }],
  // RussianIrina writes lowercase ч in one run: short left stem down, a
  // rounded joined rise, then the full right stem down into an exit. The
  // fitted path opens that bridge into the printed shallow bowl.
    ["cyrillic:ч", {
    script: "cyrillic",
    glyph: "ч",
    strokes: [
      {
        segments: [
          {
            label: "descend the short left stem to the middle join",
            path: [
              { x: 104, y: 536 },
              { x: 104, y: 450 },
              { x: 104, y: 363 },
            ],
          },
          {
            label: "sweep through the bowl and rise along the right stem",
            path: [
              { x: 104, y: 363 },
              { x: 104, y: 255 },
              { x: 276, y: 218 },
              { x: 460, y: 250 },
              { x: 460, y: 390 },
              { x: 460, y: 536 },
            ],
          },
          {
            label: "descend the full right stem to the baseline",
            path: [
              { x: 460, y: 536 },
              { x: 460, y: 270 },
              { x: 460, y: 0 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ч"),
  }],
  // RussianIrina writes lowercase ш in one run: descend each stem from left
  // to right and rise through the two rounded joins. The fitted path squares
  // those joins into the printed glyph's horizontal baseline bars.
    ["cyrillic:ш", {
    script: "cyrillic",
    glyph: "ш",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 126, y: 536 },
              { x: 126, y: 270 },
              { x: 126, y: 37 },
            ],
          },
          {
            label: "cross the first base join and rise through the middle stem",
            path: [
              { x: 126, y: 37 },
              { x: 286, y: 37 },
              { x: 447, y: 37 },
              { x: 447, y: 270 },
              { x: 447, y: 536 },
            ],
          },
          {
            label: "retrace the middle stem to the baseline",
            path: [
              { x: 447, y: 536 },
              { x: 447, y: 270 },
              { x: 447, y: 37 },
            ],
          },
          {
            label: "cross the second base join and rise through the right stem",
            path: [
              { x: 447, y: 37 },
              { x: 607, y: 37 },
              { x: 768, y: 37 },
              { x: 768, y: 270 },
              { x: 768, y: 536 },
            ],
          },
          {
            label: "retrace the right stem to the baseline",
            path: [
              { x: 768, y: 536 },
              { x: 768, y: 270 },
              { x: 768, y: 37 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ш"),
  }],
  // RussianIrina writes lowercase щ like ш and continues from the right stem
  // directly into its looped tail. The fitted path squares the joins and keeps
  // the printed descender connected through the tail shoulder.
    ["cyrillic:щ", {
    script: "cyrillic",
    glyph: "щ",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 536 },
              { x: 129, y: 270 },
              { x: 129, y: 37 },
            ],
          },
          {
            label: "cross the first base join and rise through the middle stem",
            path: [
              { x: 129, y: 37 },
              { x: 287, y: 37 },
              { x: 445, y: 37 },
              { x: 445, y: 270 },
              { x: 445, y: 536 },
            ],
          },
          {
            label: "retrace the middle stem to the baseline",
            path: [
              { x: 445, y: 536 },
              { x: 445, y: 270 },
              { x: 445, y: 37 },
            ],
          },
          {
            label: "cross the second base join and rise through the right stem",
            path: [
              { x: 445, y: 37 },
              { x: 603, y: 37 },
              { x: 760, y: 37 },
              { x: 760, y: 270 },
              { x: 760, y: 536 },
            ],
          },
          {
            label: "retrace the right stem and cross the tail shoulder",
            path: [
              { x: 760, y: 536 },
              { x: 760, y: 270 },
              { x: 760, y: 37 },
              { x: 845, y: 37 },
            ],
          },
          {
            label: "descend the short tail below the baseline",
            path: [
              { x: 845, y: 37 },
              { x: 845, y: -50 },
              { x: 845, y: -140 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("щ"),
  }],
  // RussianIrina writes lowercase ъ in one run: a narrow entry loop and top
  // shoulder flow into the descending stem, which turns directly through the
  // counterclockwise lower bowl. The fitted path squares the entry into the
  // printed top flag while preserving that flag-to-stem-to-bowl order.
    ["cyrillic:ъ", {
    script: "cyrillic",
    glyph: "ъ",
    strokes: [
      {
        segments: [
          {
            label: "sweep right along the broad top flag",
            path: [
              { x: 15, y: 499 },
              { x: 80, y: 499 },
              { x: 145, y: 499 },
              { x: 207, y: 499 },
            ],
          },
          {
            label: "descend the main stem to the baseline",
            path: [
              { x: 207, y: 499 },
              { x: 207, y: 300 },
              { x: 207, y: 150 },
              { x: 207, y: 36 },
            ],
          },
          {
            label: "sweep right along the lower bowl",
            path: [
              { x: 207, y: 36 },
              { x: 280, y: 36 },
              { x: 357, y: 36 },
              { x: 440, y: 45 },
              { x: 510, y: 85 },
            ],
          },
          {
            label: "curve upward around the bowl's right side",
            path: [
              { x: 510, y: 85 },
              { x: 533, y: 130 },
              { x: 533, y: 173 },
              { x: 515, y: 220 },
              { x: 470, y: 265 },
              { x: 410, y: 294 },
              { x: 357, y: 301 },
            ],
          },
          {
            label: "return left through the upper bowl to close against the stem",
            path: [
              { x: 357, y: 301 },
              { x: 305, y: 301 },
              { x: 251, y: 301 },
              { x: 207, y: 301 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ъ"),
  }],
  // RussianIrina writes lowercase ы in two runs: the descending left stem
  // turns directly through a counterclockwise lower bowl, then a lifted right
  // stem descends into a curled exit. The fitted path keeps that body-first
  // order while straightening both stems and closing the printed bowl.
    ["cyrillic:ы", {
    script: "cyrillic",
    glyph: "ы",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 537 },
              { x: 129, y: 360 },
              { x: 129, y: 180 },
              { x: 129, y: 37 },
            ],
          },
          {
            label: "sweep right along the lower bowl",
            path: [
              { x: 129, y: 37 },
              { x: 200, y: 37 },
              { x: 276, y: 37 },
              { x: 350, y: 45 },
              { x: 425, y: 88 },
            ],
          },
          {
            label: "curve upward around the bowl's right side",
            path: [
              { x: 425, y: 88 },
              { x: 451, y: 130 },
              { x: 451, y: 176 },
              { x: 435, y: 220 },
              { x: 395, y: 263 },
              { x: 335, y: 297 },
              { x: 276, y: 304 },
            ],
          },
          {
            label: "return left through the upper bowl to close against the stem",
            path: [
              { x: 276, y: 304 },
              { x: 225, y: 304 },
              { x: 173, y: 304 },
              { x: 129, y: 304 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the separate right stem",
            path: [
              { x: 630, y: 537 },
              { x: 630, y: 360 },
              { x: 630, y: 180 },
              { x: 630, y: 37 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ы"),
  }],
  // RussianIrina writes lowercase ь in one run: the descending stem turns
  // directly through a counterclockwise lower bowl and closes against itself.
  // The fitted path keeps that stem-first order while straightening the
  // upright and widening the printed bowl.
    ["cyrillic:ь", {
    script: "cyrillic",
    glyph: "ь",
    strokes: [
      {
        segments: [
          {
            label: "descend the stem to the baseline",
            path: [
              { x: 129, y: 536 },
              { x: 129, y: 360 },
              { x: 129, y: 180 },
              { x: 129, y: 36 },
            ],
          },
          {
            label: "sweep right along the lower bowl",
            path: [
              { x: 129, y: 36 },
              { x: 200, y: 36 },
              { x: 279, y: 36 },
              { x: 355, y: 44 },
              { x: 430, y: 87 },
            ],
          },
          {
            label: "curve upward around the bowl's right side",
            path: [
              { x: 430, y: 87 },
              { x: 456, y: 128 },
              { x: 456, y: 173 },
              { x: 440, y: 217 },
              { x: 400, y: 258 },
              { x: 340, y: 291 },
              { x: 279, y: 301 },
            ],
          },
          {
            label: "return left through the upper bowl to close against the stem",
            path: [
              { x: 279, y: 301 },
              { x: 225, y: 301 },
              { x: 173, y: 301 },
              { x: 129, y: 301 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ь"),
  }],
  // RussianIrina writes lowercase э in two runs: the outer backwards-C curve
  // travels from upper left around the right side to lower left, then a lifted
  // tongue travels right-to-left. The fitted path widens the curve and
  // straightens the printed middle bar without changing that order.
    ["cyrillic:э", {
    script: "cyrillic",
    glyph: "э",
    strokes: [
      {
        segments: [
          {
            label: "sweep right across the upper curve",
            path: [
              { x: 82, y: 472 },
              { x: 150, y: 500 },
              { x: 230, y: 505 },
              { x: 315, y: 480 },
              { x: 378, y: 420 },
            ],
          },
          {
            label: "continue down around the outer right side",
            path: [
              { x: 378, y: 420 },
              { x: 420, y: 350 },
              { x: 425, y: 270 },
              { x: 415, y: 185 },
              { x: 378, y: 110 },
            ],
          },
          {
            label: "sweep left through the lower curve",
            path: [
              { x: 378, y: 110 },
              { x: 315, y: 45 },
              { x: 230, y: 25 },
              { x: 150, y: 35 },
              { x: 82, y: 72 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle tongue right-to-left",
            path: [
              { x: 356, y: 276 },
              { x: 290, y: 276 },
              { x: 225, y: 276 },
              { x: 160, y: 276 },
              { x: 95, y: 276 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("э"),
  }],
  // RussianIrina writes lowercase ю in one run: the descending left stem
  // turns through a rising connector and continues clockwise around the oval.
  // The fitted path retraces to the printed middle bar while preserving that
  // zero-lift stem-to-connector-to-oval order.
    ["cyrillic:ю", {
    script: "cyrillic",
    glyph: "ю",
    strokes: [
      {
        segments: [
          {
            label: "descend the left stem to the baseline",
            path: [
              { x: 129, y: 536 },
              { x: 129, y: 360 },
              { x: 129, y: 180 },
              { x: 129, y: 0 },
            ],
          },
          {
            label: "retrace upward and sweep right along the middle bar",
            path: [
              { x: 129, y: 0 },
              { x: 129, y: 140 },
              { x: 129, y: 276 },
              { x: 210, y: 276 },
              { x: 322, y: 276 },
            ],
          },
          {
            label: "curve upward around the oval and across its top",
            path: [
              { x: 322, y: 276 },
              { x: 330, y: 390 },
              { x: 395, y: 475 },
              { x: 514, y: 510 },
              { x: 625, y: 475 },
              { x: 695, y: 390 },
            ],
          },
          {
            label: "continue down around the oval's right side",
            path: [
              { x: 695, y: 390 },
              { x: 715, y: 320 },
              { x: 715, y: 245 },
              { x: 695, y: 165 },
              { x: 650, y: 90 },
            ],
          },
          {
            label: "sweep left through the bottom and rise to close",
            path: [
              { x: 650, y: 90 },
              { x: 585, y: 30 },
              { x: 514, y: 27 },
              { x: 405, y: 55 },
              { x: 340, y: 145 },
              { x: 322, y: 276 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("ю"),
  }],
  // RussianIrina writes lowercase я in one run: rise from the baseline,
  // circle the upper loop counterclockwise, then descend the diagonal leg.
  // The printed fit uses its right upright as that rise and preserves the
  // source's zero-lift rise-to-loop-to-leg order.
    ["cyrillic:я", {
    script: "cyrillic",
    glyph: "я",
    strokes: [
      {
        segments: [
          {
            label: "climb the right stem from the baseline to the top",
            path: [
              { x: 449, y: 0 },
              { x: 449, y: 140 },
              { x: 449, y: 300 },
              { x: 449, y: 440 },
              { x: 449, y: 499 },
            ],
          },
          {
            label: "curve counterclockwise around the upper bowl",
            path: [
              { x: 449, y: 499 },
              { x: 360, y: 499 },
              { x: 265, y: 499 },
              { x: 175, y: 475 },
              { x: 115, y: 425 },
              { x: 105, y: 370 },
              { x: 120, y: 325 },
              { x: 180, y: 290 },
              { x: 280, y: 243 },
              { x: 405, y: 243 },
            ],
          },
          {
            label: "sweep left through the bowl's lower join",
            path: [
              { x: 405, y: 243 },
              { x: 340, y: 243 },
              { x: 277, y: 225 },
              { x: 187, y: 218 },
            ],
          },
          {
            label: "descend the diagonal leg to the lower-left tip",
            path: [
              { x: 187, y: 218 },
              { x: 155, y: 170 },
              { x: 120, y: 120 },
              { x: 85, y: 70 },
              { x: 35, y: 0 },
            ],
          },
        ],
      },
    ],
    source: cyrillicAlphabetSource("я"),
  }],
];
