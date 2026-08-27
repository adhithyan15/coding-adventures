// Authored chinese ductus records. This is the stable source-ownership boundary.

import type { Point, Stroke, StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import chinese from "../../../../../learning/human-languages/data/scripts/chinese.json";

const chineseCharacterSource = (glyph: string): StrokeSource => {
  const letter = chinese.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Chinese ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const simpleStroke = (label: string, path: Point[]): Stroke => ({
  segments: [{ label, path }],
});

export const entries: DuctusEntry[] = [
  // Hanzi Writer Data's ordered medians draw 人 with the left-falling stroke
  // first, then restart at the central junction for the right-falling stroke.
  // The source's Arphic-derived proportions are fitted to the vendored Noto
  // Sans SC outline while preserving both directions and the intervening lift.
    ["chinese:人", {
    script: "chinese",
    glyph: "人",
    strokes: [
      {
        segments: [
          {
            label: "draw the left-falling piě stroke from the upper centre",
            path: [
              { x: 500, y: 810 },
              { x: 500, y: 740 },
              { x: 490, y: 650 },
              { x: 470, y: 555 },
              { x: 445, y: 465 },
              { x: 410, y: 375 },
              { x: 365, y: 285 },
              { x: 310, y: 200 },
              { x: 245, y: 120 },
              { x: 175, y: 55 },
              { x: 105, y: 5 },
              { x: 65, y: -25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the right-falling nà stroke from the junction",
            path: [
              { x: 500, y: 690 },
              { x: 515, y: 620 },
              { x: 535, y: 535 },
              { x: 565, y: 445 },
              { x: 605, y: 355 },
              { x: 655, y: 265 },
              { x: 715, y: 180 },
              { x: 785, y: 105 },
              { x: 860, y: 45 },
              { x: 925, y: 0 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("人"),
  }],
  // The compressed person radical keeps the source dataset's two-run order:
  // a long left-falling stroke, then a separately started vertical. Its Noto
  // Sans SC fit follows the glyph's narrow left-side proportions rather than
  // mechanically squeezing the full 人 path.
    ["chinese:亻", {
    script: "chinese",
    glyph: "亻",
    strokes: [
      {
        segments: [
          {
            label: "draw the left-falling piě stroke from upper right to lower left",
            path: [
              { x: 440, y: 820 },
              { x: 430, y: 790 },
              { x: 415, y: 755 },
              { x: 395, y: 720 },
              { x: 375, y: 680 },
              { x: 350, y: 640 },
              { x: 325, y: 600 },
              { x: 295, y: 560 },
              { x: 265, y: 520 },
              { x: 230, y: 475 },
              { x: 195, y: 435 },
              { x: 160, y: 395 },
              { x: 125, y: 360 },
              { x: 95, y: 330 },
              { x: 75, y: 305 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the vertical shù stroke from the junction to the baseline",
            path: [
              { x: 310, y: 590 },
              { x: 310, y: 550 },
              { x: 310, y: 500 },
              { x: 310, y: 440 },
              { x: 310, y: 370 },
              { x: 310, y: 295 },
              { x: 310, y: 220 },
              { x: 310, y: 140 },
              { x: 310, y: 60 },
              { x: 310, y: -50 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("亻"),
  }],
  // 口 establishes the first Chinese joined corner in the authored inventory:
  // descend the left side, join the top and right side in one héngzhé run, then
  // close the bottom last. The flat Noto fit preserves those three source runs.
    ["chinese:口", {
    script: "chinese",
    glyph: "口",
    strokes: [
      {
        segments: [
          {
            label: "draw the left vertical shù stroke from top to bottom",
            path: [
              { x: 166, y: 700 },
              { x: 166, y: 620 },
              { x: 166, y: 530 },
              { x: 166, y: 440 },
              { x: 166, y: 350 },
              { x: 166, y: 260 },
              { x: 166, y: 170 },
              { x: 166, y: 80 },
              { x: 166, y: -35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the top bar from left to right",
            path: [
              { x: 166, y: 700 },
              { x: 260, y: 700 },
              { x: 360, y: 700 },
              { x: 470, y: 700 },
              { x: 580, y: 700 },
              { x: 690, y: 700 },
              { x: 785, y: 700 },
              { x: 835, y: 700 },
            ],
          },
          {
            label: "turn the corner without lifting and descend the right side",
            path: [
              { x: 835, y: 700 },
              { x: 835, y: 610 },
              { x: 835, y: 520 },
              { x: 835, y: 430 },
              { x: 835, y: 340 },
              { x: 835, y: 250 },
              { x: 835, y: 160 },
              { x: 835, y: 70 },
              { x: 835, y: -30 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then close the bottom from left to right",
            path: [
              { x: 166, y: 70 },
              { x: 260, y: 70 },
              { x: 360, y: 70 },
              { x: 470, y: 70 },
              { x: 580, y: 70 },
              { x: 690, y: 70 },
              { x: 785, y: 70 },
              { x: 835, y: 70 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("口"),
  }],
  // 女 begins with one bent piědiǎn run: descend down-left, turn at the lower
  // junction, and sweep down-right without lifting. A separately started
  // left-falling piě comes next, then the middle héng crosses left-to-right.
  // The four movements follow the three pinned medians on the Noto Sans SC fit.
    ["chinese:女", {
    script: "chinese",
    glyph: "女",
    strokes: [
      {
        segments: [
          {
            label: "draw the first piědiǎn stroke down and left",
            path: [
              { x: 460, y: 840 },
              { x: 440, y: 790 },
              { x: 415, y: 720 },
              { x: 390, y: 650 },
              { x: 365, y: 580 },
              { x: 340, y: 510 },
              { x: 310, y: 440 },
              { x: 285, y: 375 },
              { x: 255, y: 320 },
              { x: 220, y: 275 },
            ],
          },
          {
            label: "turn without lifting and sweep down to the lower right",
            path: [
              { x: 220, y: 275 },
              { x: 300, y: 265 },
              { x: 400, y: 220 },
              { x: 500, y: 175 },
              { x: 600, y: 125 },
              { x: 700, y: 75 },
              { x: 800, y: 20 },
              { x: 890, y: -35 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the left-falling piě stroke from upper right to lower left",
            path: [
              { x: 717, y: 550 },
              { x: 700, y: 490 },
              { x: 680, y: 430 },
              { x: 650, y: 360 },
              { x: 615, y: 295 },
              { x: 570, y: 235 },
              { x: 520, y: 180 },
              { x: 460, y: 125 },
              { x: 390, y: 75 },
              { x: 310, y: 30 },
              { x: 220, y: -10 },
              { x: 130, y: -45 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng from left to right",
            path: [
              { x: 70, y: 561 },
              { x: 180, y: 561 },
              { x: 300, y: 561 },
              { x: 420, y: 561 },
              { x: 540, y: 561 },
              { x: 660, y: 561 },
              { x: 780, y: 561 },
              { x: 890, y: 561 },
              { x: 940, y: 561 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("女"),
  }],
  // 子 has two joined turns across its first two strokes: the top horizontal
  // turns down-left, then a separately started vertical hooks left at the base.
  // A second lift precedes the final middle horizontal from left to right.
    ["chinese:子", {
    script: "chinese",
    glyph: "子",
    strokes: [
      {
        segments: [
          {
            label: "draw the top horizontal héng from left to right",
            path: [
              { x: 160, y: 735 },
              { x: 250, y: 735 },
              { x: 350, y: 735 },
              { x: 450, y: 735 },
              { x: 550, y: 735 },
              { x: 650, y: 735 },
              { x: 740, y: 735 },
              { x: 790, y: 735 },
            ],
          },
          {
            label: "turn without lifting and sweep down-left",
            path: [
              { x: 790, y: 735 },
              { x: 750, y: 680 },
              { x: 700, y: 640 },
              { x: 650, y: 600 },
              { x: 600, y: 565 },
              { x: 550, y: 535 },
              { x: 490, y: 515 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the central vertical",
            path: [
              { x: 504, y: 530 },
              { x: 504, y: 460 },
              { x: 504, y: 380 },
              { x: 504, y: 300 },
              { x: 504, y: 220 },
              { x: 504, y: 140 },
              { x: 504, y: 70 },
              { x: 500, y: 20 },
              { x: 500, y: -35 },
            ],
          },
          {
            label: "hook left at the base without lifting",
            path: [
              { x: 500, y: -35 },
              { x: 450, y: -40 },
              { x: 390, y: -40 },
              { x: 330, y: -35 },
              { x: 285, y: -20 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng from left to right",
            path: [
              { x: 60, y: 357 },
              { x: 170, y: 357 },
              { x: 290, y: 357 },
              { x: 410, y: 357 },
              { x: 530, y: 357 },
              { x: 650, y: 357 },
              { x: 770, y: 357 },
              { x: 890, y: 357 },
              { x: 945, y: 357 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("子"),
  }],
  // 日 starts with the left side, then joins the top bar to the right side in
  // one héngzhé stroke. The inside bar precedes a separately closing bottom.
    ["chinese:日", {
    script: "chinese",
    glyph: "日",
    strokes: [
      {
        segments: [
          {
            label: "descend the left vertical shù from top to bottom",
            path: [
              { x: 214, y: 735 },
              { x: 214, y: 630 },
              { x: 214, y: 520 },
              { x: 214, y: 410 },
              { x: 214, y: 300 },
              { x: 214, y: 190 },
              { x: 214, y: 80 },
              { x: 214, y: 0 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the top horizontal héng from left to right",
            path: [
              { x: 214, y: 735 },
              { x: 310, y: 735 },
              { x: 410, y: 735 },
              { x: 510, y: 735 },
              { x: 610, y: 735 },
              { x: 710, y: 735 },
              { x: 792, y: 735 },
            ],
          },
          {
            label: "turn without lifting and descend the right side",
            path: [
              { x: 792, y: 735 },
              { x: 792, y: 630 },
              { x: 792, y: 520 },
              { x: 792, y: 410 },
              { x: 792, y: 300 },
              { x: 792, y: 190 },
              { x: 792, y: 80 },
              { x: 792, y: 0 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng from left to right",
            path: [
              { x: 214, y: 389 },
              { x: 310, y: 389 },
              { x: 410, y: 389 },
              { x: 510, y: 389 },
              { x: 610, y: 389 },
              { x: 710, y: 389 },
              { x: 792, y: 389 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then close the bottom horizontal héng from left to right",
            path: [
              { x: 214, y: 33 },
              { x: 310, y: 33 },
              { x: 410, y: 33 },
              { x: 510, y: 33 },
              { x: 610, y: 33 },
              { x: 710, y: 33 },
              { x: 792, y: 33 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("日"),
  }],
  // 讠 starts with a down-right dot. After one lift, the short horizontal,
  // vertical descent, and rising finish stay joined inside one second stroke.
    ["chinese:讠", {
    script: "chinese",
    glyph: "讠",
    strokes: [
      {
        segments: [
          {
            label: "draw the top dot down and right",
            path: [
              { x: 150, y: 780 },
              { x: 180, y: 755 },
              { x: 215, y: 720 },
              { x: 250, y: 685 },
              { x: 290, y: 645 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the short horizontal from left to right",
            path: [
              { x: 60, y: 492 },
              { x: 110, y: 492 },
              { x: 160, y: 492 },
              { x: 210, y: 492 },
              { x: 255, y: 492 },
              { x: 293, y: 492 },
            ],
          },
          {
            label: "turn without lifting and descend the vertical",
            path: [
              { x: 293, y: 492 },
              { x: 293, y: 410 },
              { x: 293, y: 320 },
              { x: 293, y: 230 },
              { x: 293, y: 140 },
              { x: 293, y: 60 },
              { x: 293, y: 20 },
            ],
          },
          {
            label: "turn without lifting and rise to the upper right",
            path: [
              { x: 293, y: 20 },
              { x: 330, y: 35 },
              { x: 370, y: 60 },
              { x: 410, y: 85 },
              { x: 445, y: 110 },
              { x: 475, y: 140 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("讠"),
  }],
  // 氵 stacks two separately drawn down-right dots above a third stroke that
  // begins at the bottom and rises to the upper right. The three pinned
  // medians remain separate while fitting the narrow Noto Sans SC radical.
    ["chinese:氵", {
    script: "chinese",
    glyph: "氵",
    strokes: [
      {
        segments: [
          {
            label: "draw the upper dot down and right",
            path: [
              { x: 155, y: 785 },
              { x: 195, y: 770 },
              { x: 235, y: 745 },
              { x: 275, y: 720 },
              { x: 315, y: 695 },
              { x: 350, y: 675 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle dot down and right",
            path: [
              { x: 72, y: 515 },
              { x: 110, y: 505 },
              { x: 150, y: 485 },
              { x: 190, y: 465 },
              { x: 230, y: 445 },
              { x: 270, y: 420 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then begin the bottom stroke with a slight rise left",
            path: [
              { x: 158, y: -58 },
              { x: 150, y: -32 },
              { x: 155, y: 0 },
            ],
          },
          {
            label: "continue without lifting in a long rise to the upper right",
            path: [
              { x: 155, y: 0 },
              { x: 185, y: 45 },
              { x: 220, y: 95 },
              { x: 255, y: 145 },
              { x: 290, y: 195 },
              { x: 325, y: 245 },
              { x: 360, y: 295 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("氵"),
  }],
  // 宀 places its top dot first, then a separate down-left stroke on the left.
  // After the second lift, the roof crosses left-to-right and hooks down-left
  // without breaking. The Noto fit keeps that source order and joined hook.
    ["chinese:宀", {
    script: "chinese",
    glyph: "宀",
    strokes: [
      {
        segments: [
          {
            label: "draw the top dot down and right",
            path: [
              { x: 440, y: 805 },
              { x: 455, y: 790 },
              { x: 470, y: 770 },
              { x: 485, y: 750 },
              { x: 500, y: 730 },
              { x: 515, y: 715 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the left-side stroke down and left",
            path: [
              { x: 150, y: 660 },
              { x: 145, y: 625 },
              { x: 138, y: 585 },
              { x: 130, y: 545 },
              { x: 122, y: 505 },
              { x: 112, y: 475 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the horizontal roof from left to right",
            path: [
              { x: 150, y: 646 },
              { x: 250, y: 646 },
              { x: 360, y: 646 },
              { x: 470, y: 646 },
              { x: 580, y: 646 },
              { x: 690, y: 646 },
              { x: 790, y: 646 },
              { x: 875, y: 646 },
            ],
          },
          {
            label: "hook down and left without lifting",
            path: [
              { x: 875, y: 646 },
              { x: 880, y: 620 },
              { x: 875, y: 585 },
              { x: 865, y: 545 },
              { x: 850, y: 505 },
              { x: 833, y: 475 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("宀"),
  }],
  // 你 writes 亻 first, then the five strokes of 尔: a falling stroke, a
  // joined horizontal hook, a joined vertical hook, and two separate dots.
  // The seven Noto-fitted runs preserve that component order and six lifts.
    ["chinese:你", {
    script: "chinese",
    glyph: "你",
    strokes: [
      {
        segments: [
          {
            label: "draw the left-falling stroke of the person radical",
            path: [
              { x: 300, y: 810 },
              { x: 285, y: 760 },
              { x: 265, y: 705 },
              { x: 240, y: 650 },
              { x: 210, y: 595 },
              { x: 175, y: 540 },
              { x: 140, y: 495 },
              { x: 105, y: 455 },
              { x: 70, y: 425 },
              { x: 45, y: 410 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the vertical stroke of the person radical",
            path: [
              { x: 196, y: 605 },
              { x: 196, y: 520 },
              { x: 196, y: 430 },
              { x: 196, y: 340 },
              { x: 196, y: 250 },
              { x: 196, y: 160 },
              { x: 196, y: 70 },
              { x: 196, y: -50 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the upper-right left-falling stroke",
            path: [
              { x: 500, y: 810 },
              { x: 490, y: 765 },
              { x: 475, y: 715 },
              { x: 455, y: 660 },
              { x: 430, y: 605 },
              { x: 405, y: 550 },
              { x: 375, y: 500 },
              { x: 345, y: 455 },
              { x: 325, y: 435 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the upper horizontal from left to right",
            path: [
              { x: 450, y: 612 },
              { x: 530, y: 612 },
              { x: 620, y: 612 },
              { x: 710, y: 612 },
              { x: 800, y: 612 },
              { x: 890, y: 612 },
            ],
          },
          {
            label: "hook down and left without lifting",
            path: [
              { x: 890, y: 612 },
              { x: 900, y: 580 },
              { x: 900, y: 540 },
              { x: 895, y: 500 },
              { x: 885, y: 465 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend the central vertical",
            path: [
              { x: 649, y: 590 },
              { x: 649, y: 500 },
              { x: 649, y: 400 },
              { x: 649, y: 300 },
              { x: 649, y: 200 },
              { x: 649, y: 100 },
              { x: 649, y: 15 },
              { x: 645, y: -30 },
            ],
          },
          {
            label: "hook left at the base without lifting",
            path: [
              { x: 645, y: -30 },
              { x: 615, y: -40 },
              { x: 580, y: -42 },
              { x: 545, y: -40 },
              { x: 515, y: -30 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the lower-left dot down and left",
            path: [
              { x: 485, y: 380 },
              { x: 470, y: 330 },
              { x: 450, y: 275 },
              { x: 425, y: 220 },
              { x: 400, y: 165 },
              { x: 370, y: 110 },
              { x: 345, y: 80 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the lower-right dot down and right",
            path: [
              { x: 790, y: 380 },
              { x: 815, y: 335 },
              { x: 840, y: 285 },
              { x: 865, y: 235 },
              { x: 885, y: 185 },
              { x: 900, y: 135 },
              { x: 915, y: 90 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("你"),
  }],
  // 好 writes all three strokes of 女 before the three strokes of 子. The
  // first strokes of both components turn without lifting, and 子's vertical
  // keeps its base hook joined. Six Noto-fitted runs preserve five lifts.
    ["chinese:好", {
    script: "chinese",
    glyph: "好",
    strokes: [
      {
        segments: [
          {
            label: "draw 女's first bent stroke down and left",
            path: [
              { x: 218, y: 820 },
              { x: 205, y: 750 },
              { x: 190, y: 675 },
              { x: 175, y: 600 },
              { x: 155, y: 520 },
              { x: 135, y: 440 },
              { x: 120, y: 365 },
              { x: 100, y: 320 },
              { x: 82, y: 300 },
            ],
          },
          {
            label: "turn without lifting and sweep right",
            path: [
              { x: 82, y: 300 },
              { x: 145, y: 270 },
              { x: 205, y: 225 },
              { x: 265, y: 175 },
              { x: 325, y: 120 },
              { x: 375, y: 70 },
              { x: 410, y: 40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 女's left-falling stroke",
            path: [
              { x: 390, y: 620 },
              { x: 380, y: 550 },
              { x: 365, y: 475 },
              { x: 345, y: 395 },
              { x: 320, y: 310 },
              { x: 290, y: 225 },
              { x: 255, y: 150 },
              { x: 215, y: 80 },
              { x: 165, y: 20 },
              { x: 95, y: -45 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 女's horizontal stroke left to right",
            path: [
              { x: 45, y: 600 },
              { x: 100, y: 600 },
              { x: 160, y: 600 },
              { x: 220, y: 600 },
              { x: 280, y: 600 },
              { x: 335, y: 600 },
              { x: 370, y: 600 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 子's top horizontal left to right",
            path: [
              { x: 485, y: 730 },
              { x: 555, y: 730 },
              { x: 630, y: 730 },
              { x: 705, y: 730 },
              { x: 780, y: 730 },
              { x: 850, y: 730 },
            ],
          },
          {
            label: "turn without lifting and sweep down-left",
            path: [
              { x: 850, y: 730 },
              { x: 840, y: 700 },
              { x: 820, y: 665 },
              { x: 790, y: 625 },
              { x: 755, y: 585 },
              { x: 715, y: 545 },
              { x: 680, y: 520 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then descend 子's vertical stroke",
            path: [
              { x: 700, y: 520 },
              { x: 700, y: 440 },
              { x: 700, y: 350 },
              { x: 700, y: 260 },
              { x: 700, y: 170 },
              { x: 700, y: 80 },
              { x: 700, y: 15 },
              { x: 695, y: -25 },
            ],
          },
          {
            label: "hook left at the base without lifting",
            path: [
              { x: 695, y: -25 },
              { x: 665, y: -35 },
              { x: 625, y: -40 },
              { x: 585, y: -38 },
              { x: 545, y: -25 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw 子's middle horizontal left to right",
            path: [
              { x: 440, y: 380 },
              { x: 520, y: 380 },
              { x: 610, y: 380 },
              { x: 700, y: 380 },
              { x: 790, y: 380 },
              { x: 880, y: 380 },
              { x: 950, y: 380 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("好"),
  }],
  // 我 has seven sourced strokes. Only the vertical and its base hook remain
  // joined; the long curved slash also hooks upward without lifting, producing
  // nine visible movements and six pen lifts.
    ["chinese:我", {
    script: "chinese",
    glyph: "我",
    strokes: [
      { segments: [{ label: "draw the short upper-left falling stroke", path: [
        { x: 450, y: 800 }, { x: 390, y: 785 }, { x: 325, y: 770 },
        { x: 255, y: 755 }, { x: 185, y: 740 }, { x: 105, y: 720 },
      ] }] },
      { segments: [{ label: "lift, then draw the upper horizontal left to right", path: [
        { x: 65, y: 510 }, { x: 180, y: 510 }, { x: 300, y: 510 },
        { x: 420, y: 510 }, { x: 540, y: 510 }, { x: 660, y: 510 },
        { x: 780, y: 510 }, { x: 900, y: 510 }, { x: 940, y: 510 },
      ] }] },
      { segments: [
        { label: "lift, then descend the vertical stroke", path: [
          { x: 307, y: 720 }, { x: 307, y: 620 }, { x: 307, y: 520 },
          { x: 307, y: 420 }, { x: 307, y: 320 }, { x: 307, y: 220 },
          { x: 307, y: 120 }, { x: 307, y: 20 }, { x: 302, y: -25 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 302, y: -25 }, { x: 275, y: -35 }, { x: 235, y: -40 },
          { x: 195, y: -38 }, { x: 155, y: -25 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw the lower rising stroke", path: [
        { x: 55, y: 215 }, { x: 120, y: 230 }, { x: 190, y: 245 },
        { x: 265, y: 262 }, { x: 340, y: 280 }, { x: 415, y: 298 },
        { x: 490, y: 315 },
      ] }] },
      { segments: [
        { label: "lift, then draw the long curved slash down and right", path: [
          { x: 600, y: 810 }, { x: 600, y: 700 }, { x: 605, y: 590 },
          { x: 615, y: 480 }, { x: 635, y: 365 }, { x: 660, y: 255 },
          { x: 700, y: 150 }, { x: 750, y: 65 }, { x: 805, y: 5 },
          { x: 850, y: -35 }, { x: 875, y: -45 },
        ] },
        { label: "hook upward on the right without lifting", path: [
          { x: 875, y: -45 }, { x: 895, y: 5 }, { x: 905, y: 55 },
          { x: 915, y: 105 }, { x: 925, y: 145 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw the separate rising slash up and left", path: [
        { x: 850, y: 390 }, { x: 815, y: 325 }, { x: 770, y: 260 },
        { x: 720, y: 200 }, { x: 660, y: 140 }, { x: 595, y: 85 },
        { x: 525, y: 35 }, { x: 455, y: -5 },
      ] }] },
      { segments: [{ label: "lift, then place the upper-right dot down and right", path: [
        { x: 755, y: 785 }, { x: 785, y: 755 }, { x: 815, y: 720 },
        { x: 845, y: 685 }, { x: 875, y: 650 }, { x: 895, y: 625 },
      ] }] },
    ],
    source: chineseCharacterSource("我"),
  }],
  // 是 closes 日 in four strokes before drawing the five-stroke lower body.
  // Only 日's top-right corner remains joined: nine strokes, eight lifts, and
  // ten visible movements on the Noto Sans SC fit.
    ["chinese:是", {
    script: "chinese",
    glyph: "是",
    strokes: [
      { segments: [{ label: "draw 日's left vertical", path: [
        { x: 200, y: 770 }, { x: 200, y: 710 }, { x: 200, y: 650 },
        { x: 200, y: 590 }, { x: 200, y: 530 }, { x: 200, y: 490 },
      ] }] },
      { segments: [
        { label: "lift, then draw 日's top horizontal", path: [
          { x: 200, y: 760 }, { x: 300, y: 760 }, { x: 400, y: 760 },
          { x: 500, y: 760 }, { x: 600, y: 760 }, { x: 700, y: 760 },
          { x: 795, y: 760 },
        ] },
        { label: "turn down the right side without lifting", path: [
          { x: 795, y: 760 }, { x: 795, y: 700 }, { x: 795, y: 640 },
          { x: 795, y: 580 }, { x: 795, y: 520 }, { x: 795, y: 490 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 日's inner horizontal", path: [
          { x: 235, y: 634 }, { x: 330, y: 634 }, { x: 430, y: 634 },
          { x: 530, y: 634 }, { x: 630, y: 634 }, { x: 730, y: 634 },
          { x: 760, y: 634 },
      ] }] },
      { segments: [{ label: "lift, then close 日 with the bottom horizontal", path: [
        { x: 235, y: 500 }, { x: 330, y: 500 }, { x: 430, y: 500 },
        { x: 530, y: 500 }, { x: 630, y: 500 }, { x: 730, y: 500 },
        { x: 760, y: 500 },
      ] }] },
      { segments: [{ label: "lift, then draw the wide middle horizontal", path: [
        { x: 65, y: 365 }, { x: 180, y: 365 }, { x: 300, y: 365 },
        { x: 420, y: 365 }, { x: 540, y: 365 }, { x: 660, y: 365 },
        { x: 780, y: 365 }, { x: 900, y: 365 }, { x: 940, y: 365 },
      ] }] },
      { segments: [{ label: "lift, then descend the central vertical", path: [
        { x: 508, y: 350 }, { x: 508, y: 300 }, { x: 508, y: 245 },
        { x: 508, y: 190 }, { x: 508, y: 130 }, { x: 508, y: 70 },
        { x: 508, y: 10 },
      ] }] },
      { segments: [{ label: "lift, then draw the short lower-right horizontal", path: [
        { x: 510, y: 185 }, { x: 580, y: 185 }, { x: 650, y: 185 },
        { x: 720, y: 185 }, { x: 790, y: 185 }, { x: 850, y: 185 },
      ] }] },
      { segments: [{ label: "lift, then draw the lower-left falling stroke", path: [
        { x: 265, y: 280 }, { x: 250, y: 230 }, { x: 230, y: 180 },
        { x: 205, y: 130 }, { x: 175, y: 85 }, { x: 140, y: 45 },
        { x: 100, y: 10 }, { x: 60, y: -25 },
      ] }] },
      { segments: [{ label: "lift, then draw the long finishing stroke down and right", path: [
        { x: 245, y: 180 }, { x: 280, y: 140 }, { x: 320, y: 105 },
        { x: 370, y: 65 }, { x: 430, y: 25 }, { x: 500, y: -5 },
        { x: 580, y: -25 }, { x: 670, y: -25 }, { x: 760, y: -25 },
        { x: 850, y: -25 }, { x: 920, y: -20 },
      ] }] },
    ],
    source: chineseCharacterSource("是"),
  }],
  // 不 places four independent strokes: top horizontal, long falling stroke,
  // central vertical, then the right-falling dot. Four strokes mean three
  // lifts and four visible movements on the Noto Sans SC fit.
    ["chinese:不", {
    script: "chinese",
    glyph: "不",
    strokes: [
      { segments: [{ label: "draw the top horizontal left-to-right", path: [
        { x: 85, y: 730 }, { x: 200, y: 730 }, { x: 320, y: 730 },
        { x: 440, y: 730 }, { x: 560, y: 730 }, { x: 680, y: 730 },
        { x: 800, y: 730 }, { x: 915, y: 730 },
      ] }] },
      { segments: [{ label: "lift, then draw the long stroke down-left", path: [
        { x: 545, y: 710 }, { x: 525, y: 650 }, { x: 490, y: 590 },
        { x: 440, y: 525 }, { x: 380, y: 460 }, { x: 315, y: 395 },
        { x: 245, y: 330 }, { x: 175, y: 275 }, { x: 105, y: 225 },
      ] }] },
      { segments: [{ label: "lift, then descend the central vertical", path: [
        { x: 500, y: 550 }, { x: 500, y: 475 }, { x: 500, y: 400 },
        { x: 500, y: 325 }, { x: 500, y: 250 }, { x: 500, y: 175 },
        { x: 500, y: 100 }, { x: 500, y: 20 }, { x: 500, y: -55 },
      ] }] },
      { segments: [{ label: "lift, then draw the separate right-falling dot", path: [
        { x: 610, y: 470 }, { x: 660, y: 430 }, { x: 715, y: 390 },
        { x: 770, y: 350 }, { x: 825, y: 305 }, { x: 875, y: 260 },
        { x: 920, y: 220 },
      ] }] },
    ],
    source: chineseCharacterSource("不"),
  }],
  // 名 completes 夕 before 口. The second 夕 stroke joins its horizontal to the
  // long down-left fall, and 口 joins its top to the right side: six strokes,
  // five lifts, and eight visible movements on the Noto Sans SC fit.
    ["chinese:名", {
    script: "chinese",
    glyph: "名",
    strokes: [
      { segments: [{ label: "draw 夕's upper left-falling stroke", path: [
        { x: 445, y: 820 }, { x: 420, y: 775 }, { x: 385, y: 730 },
        { x: 340, y: 685 }, { x: 285, y: 640 }, { x: 225, y: 600 },
        { x: 165, y: 560 }, { x: 105, y: 525 },
      ] }] },
      { segments: [
        { label: "lift, then draw 夕's horizontal", path: [
          { x: 350, y: 705 }, { x: 440, y: 705 }, { x: 530, y: 705 },
          { x: 620, y: 705 }, { x: 710, y: 705 }, { x: 775, y: 705 },
        ] },
        { label: "continue down-left without lifting", path: [
          { x: 775, y: 705 }, { x: 745, y: 650 }, { x: 700, y: 590 },
          { x: 640, y: 530 }, { x: 570, y: 470 }, { x: 490, y: 415 },
          { x: 400, y: 360 }, { x: 305, y: 315 }, { x: 205, y: 275 },
          { x: 110, y: 240 },
        ] },
      ] },
      { segments: [{ label: "lift, then place 夕's inner down-right dot", path: [
        { x: 300, y: 540 }, { x: 330, y: 515 }, { x: 365, y: 490 },
        { x: 400, y: 460 }, { x: 435, y: 430 }, { x: 470, y: 400 },
      ] }] },
      { segments: [{ label: "lift, then descend 口's left side", path: [
        { x: 290, y: 305 }, { x: 290, y: 245 }, { x: 290, y: 185 },
        { x: 290, y: 125 }, { x: 290, y: 65 }, { x: 290, y: 5 },
        { x: 290, y: -50 },
      ] }] },
      { segments: [
        { label: "lift, then draw 口's top horizontal", path: [
          { x: 300, y: 305 }, { x: 385, y: 305 }, { x: 470, y: 305 },
          { x: 555, y: 305 }, { x: 640, y: 305 }, { x: 725, y: 305 },
          { x: 810, y: 305 },
        ] },
        { label: "turn down the right side without lifting", path: [
          { x: 810, y: 305 }, { x: 810, y: 245 }, { x: 810, y: 185 },
          { x: 810, y: 125 }, { x: 810, y: 65 }, { x: 810, y: 5 },
          { x: 810, y: -50 },
        ] },
      ] },
      { segments: [{ label: "lift, then close 口 with the bottom horizontal", path: [
        { x: 300, y: 5 }, { x: 385, y: 5 }, { x: 470, y: 5 },
        { x: 555, y: 5 }, { x: 640, y: 5 }, { x: 725, y: 5 },
        { x: 800, y: 5 },
      ] }] },
    ],
    source: chineseCharacterSource("名"),
  }],
  // 字 writes 宀 before 子. The roof ends in one joined hook; 子 then keeps its
  // top turn and vertical base hook joined: six strokes, five lifts, and nine
  // visible movements on the Noto Sans SC fit.
    ["chinese:字", {
    script: "chinese",
    glyph: "字",
    strokes: [
      { segments: [{ label: "draw 宀's top dot down-right", path: [
        { x: 455, y: 825 }, { x: 475, y: 800 }, { x: 495, y: 775 },
        { x: 520, y: 750 }, { x: 545, y: 725 },
      ] }] },
      { segments: [{ label: "lift, then draw 宀's left-side stroke down-left", path: [
        { x: 125, y: 690 }, { x: 120, y: 650 }, { x: 115, y: 610 },
        { x: 105, y: 570 }, { x: 95, y: 535 },
      ] }] },
      { segments: [
        { label: "lift, then draw 宀's horizontal roof", path: [
          { x: 140, y: 700 }, { x: 250, y: 700 }, { x: 360, y: 700 },
          { x: 470, y: 700 }, { x: 580, y: 700 }, { x: 690, y: 700 },
          { x: 800, y: 700 }, { x: 880, y: 700 },
        ] },
        { label: "hook down-left without lifting", path: [
          { x: 880, y: 700 }, { x: 875, y: 660 }, { x: 865, y: 620 },
          { x: 855, y: 580 }, { x: 850, y: 545 },
        ] },
      ] },
      { segments: [
        { label: "lift, then draw 子's top horizontal", path: [
          { x: 260, y: 515 }, { x: 345, y: 515 }, { x: 430, y: 515 },
          { x: 515, y: 515 }, { x: 600, y: 515 }, { x: 685, y: 515 },
          { x: 735, y: 515 },
        ] },
        { label: "turn down-left without lifting", path: [
          { x: 735, y: 515 }, { x: 700, y: 480 }, { x: 660, y: 445 },
          { x: 615, y: 410 }, { x: 570, y: 380 }, { x: 525, y: 350 },
          { x: 490, y: 330 },
        ] },
      ] },
      { segments: [
        { label: "lift, then descend 子's vertical", path: [
          { x: 500, y: 350 }, { x: 500, y: 290 }, { x: 500, y: 230 },
          { x: 500, y: 170 }, { x: 500, y: 110 }, { x: 500, y: 50 },
          { x: 500, y: 5 },
        ] },
        { label: "hook left without lifting", path: [
          { x: 500, y: 5 }, { x: 480, y: -20 }, { x: 450, y: -35 },
          { x: 410, y: -40 }, { x: 365, y: -40 }, { x: 325, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 子's middle horizontal", path: [
        { x: 85, y: 265 }, { x: 200, y: 265 }, { x: 315, y: 265 },
        { x: 430, y: 265 }, { x: 545, y: 265 }, { x: 660, y: 265 },
        { x: 775, y: 265 }, { x: 900, y: 265 },
      ] }] },
    ],
    source: chineseCharacterSource("字"),
  }],
  // 谢 writes 讠, then 身, then 寸. Its twelve cited strokes preserve the two
  // turns in 讠's second run, 身's two-turn enclosure, and 寸's base hook:
  // twelve strokes, eleven lifts, and seventeen visible movements.
    ["chinese:谢", {
    script: "chinese",
    glyph: "谢",
    strokes: [
      { segments: [{ label: "draw 讠's top dot down-right", path: [
        { x: 90, y: 780 }, { x: 120, y: 755 }, { x: 150, y: 725 },
        { x: 185, y: 690 }, { x: 225, y: 650 },
      ] }] },
      { segments: [
        { label: "lift, then draw 讠's short horizontal", path: [
          { x: 50, y: 490 }, { x: 85, y: 490 }, { x: 120, y: 490 },
          { x: 155, y: 490 }, { x: 195, y: 490 },
        ] },
        { label: "turn down without lifting", path: [
          { x: 195, y: 490 }, { x: 195, y: 400 }, { x: 195, y: 300 },
          { x: 195, y: 200 }, { x: 195, y: 100 }, { x: 195, y: 10 },
        ] },
        { label: "turn and finish rising up-right without lifting", path: [
          { x: 195, y: 10 }, { x: 225, y: 25 }, { x: 255, y: 50 },
          { x: 285, y: 80 }, { x: 315, y: 115 }, { x: 335, y: 145 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 身's upper falling stroke", path: [
        { x: 505, y: 825 }, { x: 500, y: 790 }, { x: 485, y: 755 },
        { x: 465, y: 720 }, { x: 440, y: 685 },
      ] }] },
      { segments: [{ label: "lift, then descend 身's left side", path: [
        { x: 375, y: 680 }, { x: 375, y: 600 }, { x: 375, y: 520 },
        { x: 375, y: 440 }, { x: 375, y: 360 }, { x: 375, y: 285 },
      ] }] },
      { segments: [
        { label: "lift, then draw 身's top horizontal", path: [
          { x: 405, y: 695 }, { x: 450, y: 695 }, { x: 495, y: 695 },
          { x: 540, y: 695 }, { x: 580, y: 695 },
        ] },
        { label: "turn and descend 身's right side without lifting", path: [
          { x: 580, y: 695 }, { x: 580, y: 575 }, { x: 580, y: 455 },
          { x: 580, y: 335 }, { x: 580, y: 215 }, { x: 580, y: 95 },
          { x: 580, y: 10 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 580, y: 10 }, { x: 565, y: -10 }, { x: 540, y: -25 },
          { x: 510, y: -35 }, { x: 475, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 身's upper inner horizontal", path: [
        { x: 390, y: 565 }, { x: 430, y: 565 }, { x: 470, y: 565 },
        { x: 510, y: 565 }, { x: 550, y: 565 }, { x: 580, y: 565 },
      ] }] },
      { segments: [{ label: "lift, then draw 身's lower inner horizontal", path: [
        { x: 390, y: 430 }, { x: 430, y: 430 }, { x: 470, y: 430 },
        { x: 510, y: 430 }, { x: 550, y: 430 }, { x: 580, y: 430 },
      ] }] },
      { segments: [{ label: "lift, then draw 身's wide lower horizontal", path: [
        { x: 290, y: 285 }, { x: 350, y: 285 }, { x: 410, y: 285 },
        { x: 470, y: 285 }, { x: 530, y: 285 }, { x: 585, y: 285 },
      ] }] },
      { segments: [{ label: "lift, then draw 身's lower falling stroke down-left", path: [
        { x: 535, y: 270 }, { x: 510, y: 220 }, { x: 480, y: 170 },
        { x: 440, y: 120 }, { x: 395, y: 75 }, { x: 345, y: 30 },
        { x: 295, y: -10 },
      ] }] },
      { segments: [{ label: "lift, then draw 寸's horizontal", path: [
        { x: 650, y: 585 }, { x: 700, y: 585 }, { x: 750, y: 585 },
        { x: 800, y: 585 }, { x: 850, y: 585 }, { x: 900, y: 585 },
        { x: 950, y: 585 },
      ] }] },
      { segments: [
        { label: "lift, then descend 寸's vertical", path: [
          { x: 855, y: 825 }, { x: 855, y: 700 }, { x: 855, y: 575 },
          { x: 855, y: 450 }, { x: 855, y: 325 }, { x: 855, y: 200 },
          { x: 855, y: 75 }, { x: 855, y: 5 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 855, y: 5 }, { x: 840, y: -15 }, { x: 815, y: -30 },
          { x: 785, y: -40 }, { x: 750, y: -40 }, { x: 720, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then place 寸's dot down-right", path: [
        { x: 680, y: 430 }, { x: 700, y: 390 }, { x: 720, y: 350 },
        { x: 740, y: 310 }, { x: 760, y: 270 },
      ] }] },
    ],
    source: chineseCharacterSource("谢"),
  }],
  // 请 writes 讠 before 青. The speech radical keeps both turns inside its
  // second run; 青 closes with a joined top, right side, and leftward base hook:
  // ten strokes, nine lifts, and fourteen visible movements.
    ["chinese:请", {
    script: "chinese",
    glyph: "请",
    strokes: [
      { segments: [{ label: "draw 讠's top dot down-right", path: [
        { x: 135, y: 780 }, { x: 165, y: 750 }, { x: 195, y: 715 },
        { x: 225, y: 680 }, { x: 255, y: 650 },
      ] }] },
      { segments: [
        { label: "lift, then draw 讠's short horizontal", path: [
          { x: 45, y: 490 }, { x: 80, y: 490 }, { x: 120, y: 490 },
          { x: 160, y: 490 }, { x: 200, y: 490 }, { x: 235, y: 490 },
        ] },
        { label: "turn down without lifting", path: [
          { x: 235, y: 490 }, { x: 235, y: 400 }, { x: 235, y: 300 },
          { x: 235, y: 200 }, { x: 235, y: 100 }, { x: 235, y: 10 },
        ] },
        { label: "turn and finish rising up-right without lifting", path: [
          { x: 235, y: 10 }, { x: 265, y: 25 }, { x: 295, y: 50 },
          { x: 330, y: 80 }, { x: 360, y: 110 }, { x: 390, y: 145 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 青's top horizontal", path: [
        { x: 385, y: 735 }, { x: 475, y: 735 }, { x: 565, y: 735 },
        { x: 655, y: 735 }, { x: 745, y: 735 }, { x: 835, y: 735 },
        { x: 925, y: 735 },
      ] }] },
      { segments: [{ label: "lift, then draw 青's second horizontal", path: [
        { x: 410, y: 610 }, { x: 490, y: 610 }, { x: 570, y: 610 },
        { x: 650, y: 610 }, { x: 730, y: 610 }, { x: 810, y: 610 },
        { x: 895, y: 610 },
      ] }] },
      { segments: [{ label: "lift, then descend 青's upper vertical", path: [
        { x: 650, y: 835 }, { x: 650, y: 765 }, { x: 650, y: 695 },
        { x: 650, y: 625 }, { x: 650, y: 555 }, { x: 650, y: 485 },
      ] }] },
      { segments: [{ label: "lift, then draw 青's wide middle horizontal", path: [
        { x: 355, y: 485 }, { x: 455, y: 485 }, { x: 555, y: 485 },
        { x: 655, y: 485 }, { x: 755, y: 485 }, { x: 855, y: 485 },
        { x: 955, y: 485 },
      ] }] },
      { segments: [{ label: "lift, then descend 青's lower left side", path: [
        { x: 460, y: 370 }, { x: 460, y: 295 }, { x: 460, y: 220 },
        { x: 460, y: 145 }, { x: 460, y: 70 }, { x: 460, y: -5 },
        { x: 460, y: -70 },
      ] }] },
      { segments: [
        { label: "lift, then draw 青's lower top horizontal", path: [
          { x: 490, y: 370 }, { x: 550, y: 370 }, { x: 610, y: 370 },
          { x: 670, y: 370 }, { x: 730, y: 370 }, { x: 790, y: 370 },
          { x: 845, y: 370 },
        ] },
        { label: "turn and descend the right side without lifting", path: [
          { x: 845, y: 370 }, { x: 845, y: 295 }, { x: 845, y: 220 },
          { x: 845, y: 145 }, { x: 845, y: 70 }, { x: 845, y: 5 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 845, y: 5 }, { x: 830, y: -15 }, { x: 805, y: -30 },
          { x: 775, y: -40 }, { x: 740, y: -40 }, { x: 705, y: -35 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 青's upper inner horizontal", path: [
        { x: 480, y: 235 }, { x: 540, y: 235 }, { x: 600, y: 235 },
        { x: 660, y: 235 }, { x: 720, y: 235 }, { x: 780, y: 235 },
        { x: 825, y: 235 },
      ] }] },
      { segments: [{ label: "lift, then draw 青's lower inner horizontal", path: [
        { x: 480, y: 100 }, { x: 540, y: 100 }, { x: 600, y: 100 },
        { x: 660, y: 100 }, { x: 720, y: 100 }, { x: 780, y: 100 },
        { x: 825, y: 100 },
      ] }] },
    ],
    source: chineseCharacterSource("请"),
  }],
  // 再 opens with the upper horizontal, then builds the central frame before
  // closing with the long bottom bar: six strokes, five lifts, eight movements.
    ["chinese:再", {
    script: "chinese",
    glyph: "再",
    strokes: [
      { segments: [{ label: "draw the top horizontal left-to-right", path: [
        { x: 80, y: 745 }, { x: 220, y: 745 }, { x: 360, y: 745 },
        { x: 500, y: 745 }, { x: 640, y: 745 }, { x: 780, y: 745 },
        { x: 920, y: 745 },
      ] }] },
      { segments: [{ label: "lift, then descend the left side", path: [
        { x: 195, y: 575 }, { x: 195, y: 475 }, { x: 195, y: 375 },
        { x: 195, y: 275 }, { x: 195, y: 175 }, { x: 195, y: 75 },
        { x: 195, y: -70 },
      ] }] },
      { segments: [
        { label: "lift, then draw the frame's top horizontal", path: [
          { x: 225, y: 575 }, { x: 315, y: 575 }, { x: 405, y: 575 },
          { x: 495, y: 575 }, { x: 585, y: 575 }, { x: 675, y: 575 },
          { x: 800, y: 575 },
        ] },
        { label: "turn and descend the right side without lifting", path: [
          { x: 800, y: 575 }, { x: 800, y: 475 }, { x: 800, y: 375 },
          { x: 800, y: 275 }, { x: 800, y: 175 }, { x: 800, y: 75 },
          { x: 800, y: 10 },
        ] },
        { label: "hook left at the base without lifting", path: [
          { x: 800, y: 10 }, { x: 785, y: -10 }, { x: 760, y: -25 },
          { x: 730, y: -35 }, { x: 695, y: -35 }, { x: 655, y: -30 },
        ] },
      ] },
      { segments: [{ label: "lift, then descend the central vertical", path: [
        { x: 495, y: 755 }, { x: 495, y: 665 }, { x: 495, y: 575 },
        { x: 495, y: 485 }, { x: 495, y: 395 }, { x: 495, y: 305 },
        { x: 495, y: 205 },
      ] }] },
      { segments: [{ label: "lift, then draw the inner horizontal", path: [
        { x: 210, y: 390 }, { x: 310, y: 390 }, { x: 410, y: 390 },
        { x: 510, y: 390 }, { x: 610, y: 390 }, { x: 710, y: 390 },
        { x: 790, y: 390 },
      ] }] },
      { segments: [{ label: "lift, then close with the long bottom horizontal", path: [
        { x: 45, y: 195 }, { x: 195, y: 195 }, { x: 345, y: 195 },
        { x: 495, y: 195 }, { x: 645, y: 195 }, { x: 795, y: 195 },
        { x: 955, y: 195 },
      ] }] },
    ],
    source: chineseCharacterSource("再"),
  }],
  // 见 completes its open upper frame before drawing the two lower runs:
  // four strokes, three lifts, seven movements.
    ["chinese:见", {
    script: "chinese",
    glyph: "见",
    strokes: [
      { segments: [{ label: "descend the frame's left side", path: [
        { x: 215, y: 755 }, { x: 215, y: 670 }, { x: 215, y: 585 },
        { x: 215, y: 500 }, { x: 215, y: 415 }, { x: 215, y: 330 },
        { x: 215, y: 235 },
      ] }] },
      { segments: [
        { label: "lift, then draw the frame's top horizontal", path: [
          { x: 220, y: 745 }, { x: 310, y: 745 }, { x: 400, y: 745 },
          { x: 490, y: 745 }, { x: 580, y: 745 }, { x: 670, y: 745 },
          { x: 780, y: 745 },
        ] },
        { label: "turn and descend the right side without lifting", path: [
          { x: 780, y: 745 }, { x: 780, y: 660 }, { x: 780, y: 575 },
          { x: 780, y: 490 }, { x: 780, y: 405 }, { x: 780, y: 320 },
          { x: 780, y: 235 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw the inner left-falling leg", path: [
        { x: 490, y: 600 }, { x: 490, y: 520 }, { x: 485, y: 430 },
        { x: 475, y: 340 }, { x: 450, y: 250 }, { x: 420, y: 170 },
        { x: 380, y: 100 }, { x: 325, y: 45 }, { x: 260, y: 0 },
        { x: 180, y: -40 }, { x: 90, y: -65 },
      ] }] },
      { segments: [
        { label: "lift, then descend the second leg", path: [
          { x: 555, y: 285 }, { x: 555, y: 235 }, { x: 555, y: 185 },
          { x: 555, y: 135 }, { x: 555, y: 85 }, { x: 555, y: 55 },
        ] },
        { label: "bend right along the base without lifting", path: [
          { x: 555, y: 55 }, { x: 570, y: 20 }, { x: 610, y: -10 },
          { x: 665, y: -20 }, { x: 725, y: -20 }, { x: 785, y: -15 },
          { x: 835, y: 5 }, { x: 885, y: 40 },
        ] },
        { label: "finish with an upward hook without lifting", path: [
          { x: 885, y: 40 }, { x: 895, y: 70 }, { x: 905, y: 100 },
          { x: 915, y: 125 }, { x: 925, y: 145 }, { x: 930, y: 150 },
        ] },
      ] },
    ],
    source: chineseCharacterSource("见"),
  }],
  // 什 completes both strokes of 亻 before writing 十: four separate strokes,
  // three lifts, and four movements.
    ["chinese:什", {
    script: "chinese",
    glyph: "什",
    strokes: [
      { segments: [{ label: "draw 亻's left-falling stroke from the upper centre down-left", path: [
        { x: 280, y: 810 }, { x: 265, y: 760 }, { x: 245, y: 700 },
        { x: 220, y: 640 }, { x: 190, y: 580 }, { x: 155, y: 525 },
        { x: 120, y: 480 }, { x: 85, y: 450 }, { x: 50, y: 430 },
      ] }] },
      { segments: [{ label: "lift, then descend 亻's vertical stroke to the baseline", path: [
        { x: 225, y: 590 }, { x: 225, y: 480 }, { x: 225, y: 370 },
        { x: 225, y: 260 }, { x: 225, y: 150 }, { x: 225, y: 40 },
        { x: 225, y: -65 },
      ] }] },
      { segments: [{ label: "lift, then draw 十's horizontal stroke left-to-right", path: [
        { x: 340, y: 457 }, { x: 440, y: 457 }, { x: 540, y: 457 },
        { x: 640, y: 457 }, { x: 740, y: 457 }, { x: 840, y: 457 },
        { x: 940, y: 457 },
      ] }] },
      { segments: [{ label: "lift, then descend 十's vertical stroke through the horizontal", path: [
        { x: 646, y: 810 }, { x: 646, y: 680 }, { x: 646, y: 550 },
        { x: 646, y: 420 }, { x: 646, y: 290 }, { x: 646, y: 160 },
        { x: 646, y: 30 }, { x: 646, y: -65 },
      ] }] },
    ],
    source: chineseCharacterSource("什"),
  }],
  // 么 places its upper falling stroke, joins the second fall to its rightward
  // base sweep, then adds the final dot: three strokes, two lifts, four movements.
    ["chinese:么", {
    script: "chinese",
    glyph: "么",
    strokes: [
      { segments: [{ label: "draw the upper left-falling stroke down-left", path: [
        { x: 475, y: 805 }, { x: 455, y: 755 }, { x: 420, y: 700 },
        { x: 375, y: 640 }, { x: 325, y: 580 }, { x: 270, y: 520 },
        { x: 215, y: 470 }, { x: 165, y: 430 }, { x: 120, y: 400 },
        { x: 75, y: 410 },
      ] }] },
      { segments: [
        { label: "lift, then draw the second left-falling stroke down-left", path: [
          { x: 650, y: 580 }, { x: 620, y: 520 }, { x: 575, y: 450 },
          { x: 520, y: 375 }, { x: 455, y: 300 }, { x: 390, y: 225 },
          { x: 325, y: 155 }, { x: 260, y: 95 }, { x: 205, y: 50 },
          { x: 175, y: 30 },
        ] },
        { label: "turn and sweep right along the base without lifting", path: [
          { x: 175, y: 30 }, { x: 270, y: 35 }, { x: 380, y: 45 },
          { x: 490, y: 55 }, { x: 600, y: 70 }, { x: 705, y: 85 },
          { x: 805, y: 105 },
        ] },
      ] },
      { segments: [{ label: "lift, then place the final dot down-right", path: [
        { x: 670, y: 295 }, { x: 715, y: 245 }, { x: 760, y: 185 },
        { x: 805, y: 125 }, { x: 845, y: 65 }, { x: 885, y: 5 },
        { x: 905, y: -30 },
      ] }] },
    ],
    source: chineseCharacterSource("么"),
  }],
  // 早 completes 日 before writing 十 below it. The top and right sides of 日
  // stay joined: six strokes, five lifts, and seven learner movements.
    ["chinese:早", {
    script: "chinese",
    glyph: "早",
    strokes: [
      { segments: [{ label: "descend 日's left side from the upper left", path: [
        { x: 189, y: 759 }, { x: 189, y: 690 }, { x: 189, y: 620 },
        { x: 189, y: 550 }, { x: 189, y: 480 }, { x: 189, y: 412 },
      ] }] },
      { segments: [
        { label: "lift, then draw 日's top horizontal left-to-right", path: [
          { x: 189, y: 759 }, { x: 290, y: 759 }, { x: 395, y: 759 },
          { x: 500, y: 759 }, { x: 605, y: 759 }, { x: 710, y: 759 },
          { x: 806, y: 759 },
        ] },
        { label: "turn without lifting and descend 日's right side", path: [
          { x: 806, y: 759 }, { x: 806, y: 690 }, { x: 806, y: 620 },
          { x: 806, y: 550 }, { x: 806, y: 480 }, { x: 806, y: 412 },
        ] },
      ] },
      { segments: [{ label: "lift, then draw 日's middle horizontal left-to-right", path: [
        { x: 189, y: 587 }, { x: 290, y: 587 }, { x: 395, y: 587 },
        { x: 500, y: 587 }, { x: 605, y: 587 }, { x: 710, y: 587 },
        { x: 806, y: 587 },
      ] }] },
      { segments: [{ label: "lift, then close 日 with its bottom horizontal left-to-right", path: [
        { x: 189, y: 412 }, { x: 290, y: 412 }, { x: 395, y: 412 },
        { x: 500, y: 412 }, { x: 605, y: 412 }, { x: 710, y: 412 },
        { x: 806, y: 412 },
      ] }] },
      { segments: [{ label: "lift, then draw 十's horizontal left-to-right", path: [
        { x: 60, y: 193 }, { x: 190, y: 193 }, { x: 345, y: 193 },
        { x: 500, y: 193 }, { x: 655, y: 193 }, { x: 810, y: 193 },
        { x: 944, y: 193 },
      ] }] },
      { segments: [{ label: "lift, then descend 十's vertical through the horizontal", path: [
        { x: 496, y: 389 }, { x: 496, y: 310 }, { x: 496, y: 230 },
        { x: 496, y: 150 }, { x: 496, y: 70 }, { x: 496, y: -65 },
      ] }] },
    ],
    source: chineseCharacterSource("早"),
  }],
  // 上 descends its vertical first, then places the short middle horizontal
  // before the long base: three separate strokes, two lifts, three movements.
    ["chinese:上", {
    script: "chinese",
    glyph: "上",
    strokes: [
      { segments: [{ label: "descend the central vertical from top to bottom", path: [
        { x: 466, y: 810 }, { x: 466, y: 680 }, { x: 466, y: 550 },
        { x: 466, y: 420 }, { x: 466, y: 290 }, { x: 466, y: 160 },
        { x: 466, y: 20 },
      ] }] },
      { segments: [{ label: "lift, then draw the short middle horizontal left-to-right", path: [
        { x: 470, y: 478 }, { x: 550, y: 478 }, { x: 630, y: 478 },
        { x: 710, y: 478 }, { x: 790, y: 478 }, { x: 868, y: 478 },
      ] }] },
      { segments: [{ label: "lift, then draw the long base horizontal left-to-right", path: [
        { x: 65, y: 5 }, { x: 210, y: 5 }, { x: 355, y: 5 },
        { x: 500, y: 5 }, { x: 645, y: 5 }, { x: 790, y: 5 },
        { x: 936, y: 5 },
      ] }] },
    ],
    source: chineseCharacterSource("上"),
  }],
  // These six entries close the font-checked ductus for the family and school
  // vocabulary tranche. Their paths preserve Hanzi Writer Data's pinned PRC
  // stroke order while fitting the bundled Noto Sans SC outlines.
    ["chinese:儿", {
    script: "chinese",
    glyph: "儿",
    strokes: [
      simpleStroke("draw the short left-falling piě stroke from the upper centre down-left", [
        { x: 296, y: 752 }, { x: 296, y: 376 }, { x: 288, y: 368 },
        { x: 288, y: 312 }, { x: 280, y: 304 }, { x: 272, y: 232 },
        { x: 264, y: 224 }, { x: 248, y: 160 }, { x: 208, y: 88 },
        { x: 160, y: 32 },
      ]),
      simpleStroke("lift, then draw the vertical shù stroke down from the upper right, bend it right along the baseline, and hook upward without lifting", [
        { x: 664, y: 736 }, { x: 664, y: 64 }, { x: 672, y: 56 },
        { x: 672, y: 0 }, { x: 680, y: -16 }, { x: 704, y: -32 },
        { x: 872, y: -32 }, { x: 888, y: -24 }, { x: 912, y: 8 },
        { x: 912, y: 40 }, { x: 920, y: 48 }, { x: 920, y: 160 },
        { x: 904, y: 176 }, { x: 904, y: 192 },
      ]),
    ],
    source: chineseCharacterSource("儿"),
  }],
    ["chinese:家", {
    script: "chinese",
    glyph: "家",
    strokes: [
      simpleStroke("draw the top dot down and right", [
        { x: 432, y: 824 }, { x: 456, y: 808 }, { x: 480, y: 808 },
        { x: 488, y: 792 }, { x: 488, y: 768 }, { x: 496, y: 760 },
      ]),
      simpleStroke("lift, then draw the left-side roof dot down and left", [
        { x: 224, y: 712 }, { x: 128, y: 712 }, { x: 120, y: 704 },
        { x: 120, y: 576 },
      ]),
      simpleStroke("lift, then draw the horizontal roof left-to-right and hook down-left without lifting", [
        { x: 264, y: 712 }, { x: 488, y: 712 }, { x: 496, y: 720 },
        { x: 512, y: 720 }, { x: 520, y: 712 }, { x: 872, y: 712 },
        { x: 880, y: 704 }, { x: 880, y: 616 },
      ]),
      simpleStroke("lift, then draw the upper horizontal of the lower body from left to right", [
        { x: 248, y: 552 }, { x: 456, y: 552 }, { x: 464, y: 544 },
        { x: 488, y: 544 }, { x: 496, y: 552 }, { x: 640, y: 552 },
      ]),
      simpleStroke("lift, then draw its left-falling stroke", [
        { x: 456, y: 504 }, { x: 448, y: 496 }, { x: 448, y: 480 },
        { x: 424, y: 456 }, { x: 376, y: 448 }, { x: 304, y: 408 },
        { x: 288, y: 408 }, { x: 264, y: 392 }, { x: 248, y: 392 },
        { x: 240, y: 384 }, { x: 224, y: 384 }, { x: 192, y: 368 },
        { x: 168, y: 368 }, { x: 160, y: 360 }, { x: 112, y: 360 },
      ]),
      simpleStroke("lift, then descend the centre and curve to a hook at the base without lifting", [
        { x: 416, y: 456 }, { x: 432, y: 464 }, { x: 448, y: 448 },
        { x: 448, y: 440 }, { x: 472, y: 416 }, { x: 488, y: 384 },
        { x: 488, y: 368 }, { x: 504, y: 352 }, { x: 512, y: 352 },
        { x: 528, y: 328 }, { x: 536, y: 296 }, { x: 552, y: 280 },
        { x: 552, y: 224 }, { x: 560, y: 216 }, { x: 560, y: 184 },
        { x: 568, y: 176 }, { x: 568, y: 96 }, { x: 560, y: 88 },
        { x: 552, y: 32 }, { x: 512, y: -24 }, { x: 480, y: -40 },
        { x: 456, y: -48 }, { x: 392, y: -48 }, { x: 360, y: -24 },
      ]),
      simpleStroke("lift, then add the short left-falling stroke beside the centre", [
        { x: 456, y: 432 }, { x: 472, y: 416 }, { x: 488, y: 384 },
        { x: 488, y: 368 }, { x: 496, y: 360 }, { x: 480, y: 344 },
        { x: 448, y: 336 }, { x: 392, y: 296 }, { x: 264, y: 232 },
        { x: 248, y: 232 }, { x: 232, y: 216 }, { x: 216, y: 216 },
        { x: 208, y: 208 }, { x: 192, y: 208 }, { x: 160, y: 192 },
        { x: 136, y: 192 },
      ]),
      simpleStroke("lift, then add the longer left-falling stroke below it", [
        { x: 512, y: 344 }, { x: 528, y: 328 }, { x: 528, y: 312 },
        { x: 552, y: 272 }, { x: 552, y: 224 }, { x: 536, y: 208 },
        { x: 520, y: 208 }, { x: 488, y: 192 }, { x: 472, y: 176 },
        { x: 464, y: 176 }, { x: 424, y: 144 }, { x: 400, y: 136 },
        { x: 384, y: 120 }, { x: 288, y: 72 }, { x: 272, y: 72 },
        { x: 216, y: 40 }, { x: 200, y: 40 }, { x: 192, y: 32 },
        { x: 176, y: 32 }, { x: 144, y: 16 }, { x: 96, y: 16 },
      ]),
      simpleStroke("lift, then add the lower left-falling stroke", [
        { x: 832, y: 432 }, { x: 792, y: 432 }, { x: 776, y: 416 },
        { x: 768, y: 416 }, { x: 736, y: 384 }, { x: 680, y: 352 },
        { x: 664, y: 320 }, { x: 648, y: 304 }, { x: 624, y: 304 },
        { x: 616, y: 296 }, { x: 552, y: 280 }, { x: 536, y: 296 },
        { x: 528, y: 328 },
      ]),
      simpleStroke("lift, then finish with the long right-falling nà stroke", [
        { x: 528, y: 328 }, { x: 528, y: 312 }, { x: 552, y: 280 },
        { x: 568, y: 288 }, { x: 592, y: 288 }, { x: 624, y: 304 },
        { x: 648, y: 304 }, { x: 656, y: 312 }, { x: 672, y: 296 },
        { x: 672, y: 280 }, { x: 720, y: 184 }, { x: 760, y: 136 },
        { x: 760, y: 128 }, { x: 832, y: 56 }, { x: 840, y: 56 },
        { x: 888, y: 16 }, { x: 928, y: 16 }, { x: 936, y: 24 },
      ]),
    ],
    source: chineseCharacterSource("家"),
  }],
    ["chinese:大", {
    script: "chinese",
    glyph: "大",
    strokes: [
      simpleStroke("draw the horizontal héng stroke from left to right", [
        { x: 104, y: 512 }, { x: 472, y: 512 }, { x: 488, y: 496 },
        { x: 504, y: 496 }, { x: 520, y: 512 }, { x: 896, y: 512 },
      ]),
      simpleStroke("lift, then start above the bar and draw the left-falling piě stroke through it", [
        { x: 496, y: 792 }, { x: 496, y: 624 }, { x: 488, y: 616 },
        { x: 488, y: 536 }, { x: 472, y: 512 }, { x: 496, y: 488 },
        { x: 496, y: 472 }, { x: 464, y: 440 }, { x: 456, y: 424 },
        { x: 448, y: 376 }, { x: 440, y: 368 }, { x: 432, y: 328 },
        { x: 416, y: 304 }, { x: 416, y: 288 }, { x: 352, y: 176 },
        { x: 320, y: 144 }, { x: 320, y: 136 }, { x: 232, y: 48 },
        { x: 224, y: 48 }, { x: 200, y: 24 }, { x: 168, y: 8 },
      ]),
      simpleStroke("lift, return near the crossing, and draw the long right-falling nà stroke", [
        { x: 464, y: 432 }, { x: 464, y: 440 }, { x: 496, y: 472 },
        { x: 536, y: 432 }, { x: 552, y: 400 }, { x: 552, y: 384 },
        { x: 576, y: 344 }, { x: 576, y: 328 }, { x: 616, y: 248 },
        { x: 632, y: 232 }, { x: 640, y: 208 }, { x: 696, y: 144 },
        { x: 696, y: 136 }, { x: 800, y: 32 }, { x: 808, y: 32 },
        { x: 832, y: 8 }, { x: 856, y: 0 }, { x: 880, y: -24 },
        { x: 896, y: -16 }, { x: 920, y: -16 }, { x: 928, y: -8 },
      ]),
    ],
    source: chineseCharacterSource("大"),
  }],
    ["chinese:小", {
    script: "chinese",
    glyph: "小",
    strokes: [
      simpleStroke("draw the centre vertical shù stroke downward and hook left without lifting", [
        { x: 504, y: 776 }, { x: 504, y: 48 }, { x: 496, y: 40 },
        { x: 496, y: -8 }, { x: 488, y: -24 }, { x: 464, y: -40 },
        { x: 384, y: -40 },
      ]),
      simpleStroke("lift, then draw the short left-falling piě stroke", [
        { x: 224, y: 496 }, { x: 216, y: 488 }, { x: 216, y: 472 },
        { x: 200, y: 440 }, { x: 200, y: 416 }, { x: 192, y: 408 },
        { x: 184, y: 368 }, { x: 96, y: 192 },
      ]),
      simpleStroke("lift, then draw the right dot downward", [
        { x: 776, y: 520 }, { x: 792, y: 488 }, { x: 808, y: 472 },
        { x: 824, y: 440 }, { x: 824, y: 424 }, { x: 864, y: 352 },
        { x: 864, y: 336 }, { x: 880, y: 312 }, { x: 888, y: 272 },
        { x: 904, y: 248 }, { x: 920, y: 184 },
      ]),
    ],
    source: chineseCharacterSource("小"),
  }],
    ["chinese:中", {
    script: "chinese",
    glyph: "中",
    strokes: [
      simpleStroke("draw the left vertical shù stroke from top to bottom", [
        { x: 152, y: 616 }, { x: 152, y: 520 }, { x: 152, y: 424 },
        { x: 152, y: 328 }, { x: 152, y: 232 },
      ]),
      simpleStroke("lift, then draw the top horizontal left-to-right and turn down the right side without lifting", [
        { x: 132, y: 624 }, { x: 320, y: 624 }, { x: 504, y: 624 },
        { x: 688, y: 624 }, { x: 864, y: 624 }, { x: 864, y: 520 },
        { x: 864, y: 416 }, { x: 864, y: 312 }, { x: 864, y: 220 },
      ]),
      simpleStroke("lift, then close the box with the bottom horizontal héng stroke left-to-right", [
        { x: 152, y: 284 }, { x: 320, y: 284 }, { x: 488, y: 284 },
        { x: 656, y: 284 }, { x: 824, y: 284 },
      ]),
      simpleStroke("lift, then draw the central vertical shù stroke from top through the box to the base", [
        { x: 496, y: 824 }, { x: 496, y: 640 }, { x: 496, y: 456 },
        { x: 496, y: 272 }, { x: 496, y: 88 }, { x: 496, y: -48 },
      ]),
    ],
    source: chineseCharacterSource("中"),
  }],
    ["chinese:同", {
    script: "chinese",
    glyph: "同",
    strokes: [
      simpleStroke("draw the outer left vertical shù stroke from top to bottom", [
        { x: 152, y: 744 }, { x: 152, y: 584 }, { x: 152, y: 424 },
        { x: 152, y: 264 }, { x: 152, y: 104 }, { x: 152, y: -32 },
      ]),
      simpleStroke("lift, then draw the outer top horizontal and turn down the right side without lifting", [
        { x: 124, y: 752 }, { x: 312, y: 752 }, { x: 500, y: 752 },
        { x: 688, y: 752 }, { x: 877, y: 752 }, { x: 877, y: 584 },
        { x: 877, y: 416 }, { x: 877, y: 248 }, { x: 877, y: 48 },
        { x: 870, y: -16 }, { x: 840, y: -38 }, { x: 780, y: -45 },
        { x: 720, y: -42 },
      ]),
      simpleStroke("lift, then draw the short inner horizontal héng stroke left-to-right", [
        { x: 280, y: 580 }, { x: 392, y: 580 }, { x: 504, y: 580 },
        { x: 616, y: 580 }, { x: 720, y: 580 },
      ]),
      simpleStroke("lift, then draw 口's left vertical", [
        { x: 336, y: 408 }, { x: 336, y: 312 }, { x: 336, y: 216 },
        { x: 336, y: 120 },
      ]),
      simpleStroke("lift, then draw 口's top horizontal and turn down its right side without lifting", [
        { x: 352, y: 408 }, { x: 432, y: 408 }, { x: 512, y: 408 },
        { x: 592, y: 408 }, { x: 672, y: 408 }, { x: 672, y: 312 },
        { x: 672, y: 216 }, { x: 672, y: 136 },
      ]),
      simpleStroke("lift, then close 口 with its bottom horizontal left-to-right", [
        { x: 336, y: 152 }, { x: 420, y: 152 }, { x: 504, y: 152 },
        { x: 588, y: 152 }, { x: 672, y: 152 },
      ]),
    ],
    source: chineseCharacterSource("同"),
  }],
    ["chinese:学", {
    script: "chinese",
    glyph: "学",
    strokes: [
      simpleStroke("draw the centre top dot downward", [
        { x: 232, y: 768 }, { x: 232, y: 760 }, { x: 272, y: 704 },
        { x: 272, y: 648 }, { x: 280, y: 640 }, { x: 384, y: 640 },
      ]),
      simpleStroke("lift, then draw the left top dot downward", [
        { x: 448, y: 816 }, { x: 464, y: 816 }, { x: 496, y: 776 },
        { x: 520, y: 728 }, { x: 520, y: 688 },
      ]),
      simpleStroke("lift, then draw the right top stroke down-left", [
        { x: 768, y: 744 }, { x: 768, y: 736 }, { x: 744, y: 712 },
        { x: 736, y: 656 }, { x: 720, y: 640 }, { x: 592, y: 640 },
      ]),
      simpleStroke("lift, then draw the left dot of the cover", [
        { x: 236, y: 466 }, { x: 264, y: 464 },
      ]),
      simpleStroke("lift, then draw the horizontal cover and hook downward without lifting", [
        { x: 112, y: 568 }, { x: 112, y: 632 }, { x: 120, y: 640 },
        { x: 872, y: 640 }, { x: 888, y: 624 }, { x: 888, y: 512 },
      ]),
      simpleStroke("lift, then draw 子's top horizontal and turn down-left without lifting", [
        { x: 336, y: 464 }, { x: 656, y: 464 }, { x: 664, y: 456 },
        { x: 664, y: 440 }, { x: 656, y: 432 }, { x: 656, y: 416 },
        { x: 640, y: 400 }, { x: 504, y: 328 },
      ]),
      simpleStroke("lift, then draw 子's vertical and hook left without lifting", [
        { x: 504, y: 328 }, { x: 496, y: 312 }, { x: 496, y: 248 },
        { x: 504, y: 240 }, { x: 496, y: 232 }, { x: 496, y: 0 },
        { x: 488, y: -8 }, { x: 488, y: -24 }, { x: 456, y: -40 },
        { x: 352, y: -40 },
      ]),
      simpleStroke("lift, then draw 子's bottom horizontal from left to right", [
        { x: 96, y: 240 }, { x: 488, y: 240 }, { x: 496, y: 248 },
        { x: 504, y: 240 }, { x: 896, y: 240 },
      ]),
    ],
    source: chineseCharacterSource("学"),
  }],
    ["chinese:生", {
    script: "chinese",
    glyph: "生",
    strokes: [
      simpleStroke("draw the short upper left-falling piě stroke", [
        { x: 288, y: 792 }, { x: 256, y: 760 }, { x: 240, y: 696 }, { x: 224, y: 672 },
        { x: 224, y: 632 }, { x: 216, y: 624 }, { x: 208, y: 600 },
        { x: 192, y: 568 }, { x: 176, y: 536 }, { x: 152, y: 504 },
        { x: 134, y: 480 },
      ]),
      simpleStroke("lift, then draw the upper horizontal héng stroke", [
        { x: 320, y: 608 }, { x: 488, y: 608 }, { x: 496, y: 600 },
        { x: 504, y: 608 }, { x: 856, y: 608 },
      ]),
      simpleStroke("lift, then draw the shorter middle horizontal héng stroke", [
        { x: 168, y: 312 }, { x: 488, y: 312 }, { x: 496, y: 304 },
        { x: 504, y: 312 }, { x: 824, y: 312 },
      ]),
      simpleStroke("lift, then draw the vertical shù stroke through the two bars", [
        { x: 496, y: 792 }, { x: 496, y: 616 }, { x: 504, y: 608 },
        { x: 496, y: 600 }, { x: 496, y: 320 }, { x: 504, y: 312 },
        { x: 496, y: 304 }, { x: 496, y: 96 },
      ]),
      simpleStroke("lift, then draw the long bottom horizontal héng stroke", [
        { x: 144, y: -16 }, { x: 488, y: -16 }, { x: 496, y: -8 },
        { x: 504, y: -16 }, { x: 904, y: -16 },
      ]),
    ],
    source: chineseCharacterSource("生"),
  }],
  // Hanzi Writer Data draws 一 with a single left-to-right héng. There is nothing
  // to lift between, so the ductus is one stroke of one segment -- the shortest
  // entry in this table, and the reason the lesson can say the stroke count IS
  // the number.
    ["chinese:一", {
    script: "chinese",
    glyph: "一",
    strokes: [
      {
        segments: [
          {
            label: "draw the horizontal héng stroke straight across the middle, left to right",
            path: [
              { x: 52, y: 390 },
              { x: 127, y: 390 },
              { x: 202, y: 390 },
              { x: 277, y: 390 },
              { x: 352, y: 390 },
              { x: 427, y: 390 },
              { x: 502, y: 390 },
              { x: 577, y: 390 },
              { x: 652, y: 390 },
              { x: 727, y: 390 },
              { x: 802, y: 390 },
              { x: 877, y: 390 },
              { x: 952, y: 390 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("一"),
  }],
  // Two héng strokes, top before bottom, the lower one markedly wider. The source's
  // ordered medians give the same two runs; the widths are read off the vendored
  // Noto Sans SC outline rather than the Arphic-derived source graphics.
    ["chinese:二", {
    script: "chinese",
    glyph: "二",
    strokes: [
      {
        segments: [
          {
            label: "draw the upper, shorter horizontal héng stroke from left to right",
            path: [
              { x: 152, y: 656 },
              { x: 210, y: 656 },
              { x: 268, y: 656 },
              { x: 326, y: 656 },
              { x: 384, y: 656 },
              { x: 442, y: 656 },
              { x: 500, y: 656 },
              { x: 558, y: 656 },
              { x: 616, y: 656 },
              { x: 674, y: 656 },
              { x: 732, y: 656 },
              { x: 790, y: 656 },
              { x: 848, y: 656 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the lower, longer horizontal héng stroke from left to right",
            path: [
              { x: 66, y: 62 },
              { x: 139, y: 61 },
              { x: 211, y: 62 },
              { x: 284, y: 61 },
              { x: 356, y: 62 },
              { x: 429, y: 61 },
              { x: 501, y: 62 },
              { x: 574, y: 61 },
              { x: 646, y: 62 },
              { x: 719, y: 61 },
              { x: 791, y: 62 },
              { x: 864, y: 61 },
              { x: 936, y: 62 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("二"),
  }],
  // Three héng strokes, ordered top, middle, bottom. The middle is the shortest and
  // the base the widest -- the proportions that stop 三 reading as a tally, and the
  // reason this is the last numeral whose strokes can be counted for its value.
    ["chinese:三", {
    script: "chinese",
    glyph: "三",
    strokes: [
      {
        segments: [
          {
            label: "draw the top horizontal héng stroke from left to right",
            path: [
              { x: 172, y: 704 },
              { x: 224, y: 705 },
              { x: 276, y: 704 },
              { x: 329, y: 705 },
              { x: 381, y: 704 },
              { x: 433, y: 705 },
              { x: 485, y: 704 },
              { x: 537, y: 705 },
              { x: 589, y: 704 },
              { x: 642, y: 705 },
              { x: 694, y: 704 },
              { x: 746, y: 705 },
              { x: 798, y: 704 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the middle horizontal héng stroke, the shortest of the three",
            path: [
              { x: 212, y: 378 },
              { x: 258, y: 378 },
              { x: 303, y: 378 },
              { x: 349, y: 378 },
              { x: 394, y: 378 },
              { x: 440, y: 378 },
              { x: 485, y: 378 },
              { x: 531, y: 378 },
              { x: 576, y: 378 },
              { x: 622, y: 378 },
              { x: 667, y: 378 },
              { x: 713, y: 378 },
              { x: 758, y: 378 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the bottom horizontal héng stroke, the longest of the three",
            path: [
              { x: 74, y: 31 },
              { x: 145, y: 30 },
              { x: 216, y: 31 },
              { x: 287, y: 30 },
              { x: 358, y: 31 },
              { x: 429, y: 30 },
              { x: 500, y: 31 },
              { x: 571, y: 30 },
              { x: 642, y: 31 },
              { x: 713, y: 30 },
              { x: 784, y: 31 },
              { x: 855, y: 30 },
              { x: 926, y: 31 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("三"),
  }],
  // Five strokes. Medians 1-2 build the box -- the left wall, then the top and right
  // side in ONE turning héngzhé traced here as two joined segments, which is why the
  // corner counts as one stroke and not two. Medians 3-4 are the two inner pieces,
  // and median 5 closes the bottom last.
    ["chinese:四", {
    script: "chinese",
    glyph: "四",
    strokes: [
      {
        segments: [
          {
            label: "draw the left vertical shù stroke from top to bottom",
            path: [
              { x: 135, y: 690 },
              { x: 126, y: 629 },
              { x: 125, y: 568 },
              { x: 126, y: 508 },
              { x: 125, y: 447 },
              { x: 126, y: 386 },
              { x: 125, y: 325 },
              { x: 126, y: 264 },
              { x: 125, y: 203 },
              { x: 126, y: 143 },
              { x: 134, y: 82 },
              { x: 125, y: 21 },
              { x: 126, y: -40 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the horizontal-turning héngzhé stroke across the top",
            path: [
              { x: 100, y: 706 },
              { x: 164, y: 717 },
              { x: 229, y: 716 },
              { x: 293, y: 717 },
              { x: 357, y: 707 },
              { x: 421, y: 702 },
              { x: 486, y: 717 },
              { x: 550, y: 716 },
              { x: 614, y: 707 },
              { x: 678, y: 716 },
              { x: 743, y: 717 },
              { x: 807, y: 716 },
              { x: 871, y: 707 },
            ],
          },
          {
            label: "and down the right side without lifting",
            path: [
              { x: 871, y: 707 },
              { x: 870, y: 630 },
              { x: 870, y: 570 },
              { x: 870, y: 510 },
              { x: 870, y: 450 },
              { x: 870, y: 390 },
              { x: 870, y: 330 },
              { x: 870, y: 270 },
              { x: 870, y: 210 },
              { x: 870, y: 150 },
              { x: 862, y: 90 },
              { x: 858, y: 30 },
              { x: 870, y: -30 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the short inner left-falling piě stroke",
            path: [
              { x: 388, y: 670 },
              { x: 386, y: 630 },
              { x: 385, y: 591 },
              { x: 383, y: 551 },
              { x: 379, y: 512 },
              { x: 375, y: 472 },
              { x: 368, y: 433 },
              { x: 358, y: 393 },
              { x: 344, y: 353 },
              { x: 328, y: 314 },
              { x: 308, y: 274 },
              { x: 281, y: 235 },
              { x: 241, y: 195 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the inner stroke down",
            path: [
              { x: 600, y: 670 },
              { x: 600, y: 643 },
              { x: 600, y: 615 },
              { x: 600, y: 588 },
              { x: 600, y: 560 },
              { x: 600, y: 533 },
              { x: 600, y: 505 },
              { x: 600, y: 478 },
              { x: 600, y: 450 },
              { x: 600, y: 423 },
              { x: 600, y: 395 },
              { x: 600, y: 368 },
              { x: 602, y: 340 },
            ],
          },
          {
            label: "and turning up to the right at its foot",
            path: [
              { x: 602, y: 340 },
              { x: 618, y: 312 },
              { x: 635, y: 312 },
              { x: 652, y: 290 },
              { x: 669, y: 289 },
              { x: 686, y: 288 },
              { x: 703, y: 289 },
              { x: 720, y: 288 },
              { x: 737, y: 289 },
              { x: 754, y: 288 },
              { x: 771, y: 289 },
              { x: 788, y: 290 },
              { x: 805, y: 292 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then close the bottom with a horizontal héng stroke from left to right",
            path: [
              { x: 175, y: 65 },
              { x: 229, y: 65 },
              { x: 283, y: 65 },
              { x: 337, y: 65 },
              { x: 391, y: 65 },
              { x: 445, y: 65 },
              { x: 499, y: 65 },
              { x: 553, y: 65 },
              { x: 607, y: 65 },
              { x: 661, y: 65 },
              { x: 715, y: 65 },
              { x: 769, y: 65 },
              { x: 823, y: 65 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("四"),
  }],
  // Four strokes for the number five. The top bar, then a shù descending from it and
  // leaning left, then a héngzhé that crosses right and turns down, then the widest
  // stroke in the character closing it along the bottom. Where the descender crosses
  // the middle bar the traced band is clamped to one stroke's width, or its centre
  // would land between the two.
    ["chinese:五", {
    script: "chinese",
    glyph: "五",
    strokes: [
      {
        segments: [
          {
            label: "draw the top horizontal héng stroke from left to right",
            path: [
              { x: 130, y: 706 },
              { x: 191, y: 706 },
              { x: 253, y: 706 },
              { x: 314, y: 706 },
              { x: 375, y: 706 },
              { x: 436, y: 697 },
              { x: 498, y: 705 },
              { x: 559, y: 705 },
              { x: 620, y: 705 },
              { x: 681, y: 705 },
              { x: 743, y: 705 },
              { x: 804, y: 705 },
              { x: 865, y: 705 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the shù stroke descending from the top bar, leaning left",
            path: [
              { x: 446, y: 665 },
              { x: 440, y: 615 },
              { x: 432, y: 564 },
              { x: 425, y: 514 },
              { x: 416, y: 463 },
              { x: 416, y: 413 },
              { x: 400, y: 363 },
              { x: 391, y: 312 },
              { x: 382, y: 262 },
              { x: 373, y: 211 },
              { x: 365, y: 161 },
              { x: 355, y: 110 },
              { x: 345, y: 60 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the horizontal-turning héngzhé stroke across to the right",
            path: [
              { x: 190, y: 414 },
              { x: 235, y: 414 },
              { x: 281, y: 414 },
              { x: 326, y: 414 },
              { x: 372, y: 405 },
              { x: 417, y: 405 },
              { x: 463, y: 414 },
              { x: 508, y: 414 },
              { x: 553, y: 414 },
              { x: 599, y: 414 },
              { x: 644, y: 414 },
              { x: 690, y: 414 },
              { x: 735, y: 406 },
            ],
          },
          {
            label: "and then down",
            path: [
              { x: 735, y: 406 },
              { x: 724, y: 390 },
              { x: 733, y: 360 },
              { x: 730, y: 330 },
              { x: 727, y: 300 },
              { x: 725, y: 270 },
              { x: 723, y: 240 },
              { x: 720, y: 210 },
              { x: 716, y: 180 },
              { x: 714, y: 150 },
              { x: 710, y: 120 },
              { x: 708, y: 90 },
              { x: 704, y: 60 },
            ],
          },
        ],
      },
      {
        segments: [
          {
            label: "lift, then draw the long bottom horizontal héng stroke, the widest in the character",
            path: [
              { x: 62, y: 12 },
              { x: 135, y: 12 },
              { x: 208, y: 12 },
              { x: 281, y: 12 },
              { x: 354, y: 21 },
              { x: 427, y: 11 },
              { x: 500, y: 11 },
              { x: 573, y: 11 },
              { x: 646, y: 11 },
              { x: 719, y: 20 },
              { x: 792, y: 12 },
              { x: 865, y: 12 },
              { x: 938, y: 12 },
            ],
          },
        ],
      },
    ],
    source: chineseCharacterSource("五"),
  }],
  // These four entries preserve Hanzi Writer Data's pinned medians directly.
  // Their coordinates therefore describe the authoritative stroke centre-lines;
  // the separately rendered Noto Sans SC outline may vary in width and proportion.
    ["chinese:汉", {
    script: "chinese",
    glyph: "汉",
    strokes: [
      simpleStroke("draw the upper water dot down and right", [
        { x: 115, y: 790 }, { x: 190, y: 750 }, { x: 265, y: 700 },
      ]),
      simpleStroke("lift, then draw the middle water dot down and right", [
        { x: 75, y: 520 }, { x: 150, y: 480 }, { x: 225, y: 430 },
      ]),
      simpleStroke("lift, then draw the lower water stroke rising up and right", [
        { x: 100, y: 0 }, { x: 130, y: 60 }, { x: 165, y: 125 },
        { x: 205, y: 200 }, { x: 250, y: 275 },
      ]),
      simpleStroke("lift, draw the upper-right horizontal, then turn and sweep down-left without lifting", [
        { x: 360, y: 735 }, { x: 550, y: 735 }, { x: 750, y: 735 },
        { x: 860, y: 725 }, { x: 845, y: 620 }, { x: 810, y: 500 },
        { x: 760, y: 390 }, { x: 695, y: 285 }, { x: 610, y: 185 },
        { x: 560, y: 110 }, { x: 490, y: 55 }, { x: 400, y: 5 },
        { x: 315, y: -20 },
      ]),
      simpleStroke("lift, then draw the long right-falling stroke", [
        { x: 415, y: 700 }, { x: 470, y: 500 }, { x: 540, y: 350 },
        { x: 620, y: 220 }, { x: 700, y: 130 }, { x: 820, y: 35 },
        { x: 940, y: -20 },
      ]),
    ],
    source: chineseCharacterSource("汉"),
  }],
    ["chinese:语", {
    script: "chinese",
    glyph: "语",
    strokes: [
      simpleStroke("draw the speech-radical dot down and right", [
        { x: 125, y: 780 }, { x: 185, y: 735 }, { x: 250, y: 675 },
      ]),
      simpleStroke("lift, draw the speech-radical turn, and finish rising up-right without lifting", [
        { x: 50, y: 490 }, { x: 100, y: 490 }, { x: 160, y: 490 },
        { x: 220, y: 490 }, { x: 220, y: 430 }, { x: 220, y: 350 },
        { x: 220, y: 250 }, { x: 220, y: 130 }, { x: 250, y: 80 },
        { x: 345, y: 145 },
      ]),
      simpleStroke("lift, then draw 五's top horizontal left to right", [
        { x: 355, y: 775 }, { x: 490, y: 775 }, { x: 625, y: 775 },
        { x: 760, y: 775 }, { x: 920, y: 775 },
      ]),
      simpleStroke("lift, then draw 五's descending second stroke", [
        { x: 600, y: 740 }, { x: 585, y: 650 }, { x: 555, y: 520 },
        { x: 520, y: 400 },
      ]),
      simpleStroke("lift, then draw 五's horizontal-turning third stroke", [
        { x: 390, y: 590 }, { x: 500, y: 590 }, { x: 620, y: 590 },
        { x: 740, y: 590 }, { x: 820, y: 585 }, { x: 810, y: 500 },
        { x: 790, y: 400 },
      ]),
      simpleStroke("lift, then draw 五's long bottom horizontal left to right", [
        { x: 320, y: 395 }, { x: 440, y: 395 }, { x: 560, y: 395 },
        { x: 700, y: 395 }, { x: 830, y: 395 }, { x: 950, y: 395 },
      ]),
      simpleStroke("lift, then draw 口's left vertical", [
        { x: 440, y: 260 }, { x: 440, y: 100 }, { x: 440, y: -50 },
      ]),
      simpleStroke("lift, then draw 口's top and turn down the right side without lifting", [
        { x: 440, y: 260 }, { x: 520, y: 260 }, { x: 650, y: 260 },
        { x: 760, y: 260 }, { x: 850, y: 260 }, { x: 850, y: 100 },
        { x: 850, y: -45 },
      ]),
      simpleStroke("lift, then close 口 along the bottom from left to right", [
        { x: 440, y: -5 }, { x: 540, y: -5 }, { x: 650, y: -5 },
        { x: 750, y: -5 }, { x: 850, y: -5 },
      ]),
    ],
    source: chineseCharacterSource("语"),
  }],
    ["chinese:文", {
    script: "chinese",
    glyph: "文",
    strokes: [
      simpleStroke("draw the top dot down and right", [
        { x: 440, y: 820 }, { x: 470, y: 770 }, { x: 510, y: 700 },
      ]),
      simpleStroke("lift, then draw the horizontal stroke left to right", [
        { x: 50, y: 625 }, { x: 230, y: 625 }, { x: 400, y: 625 },
        { x: 570, y: 625 }, { x: 750, y: 625 }, { x: 950, y: 625 },
      ]),
      simpleStroke("lift, then draw the long left-falling stroke", [
        { x: 250, y: 600 }, { x: 275, y: 520 }, { x: 315, y: 440 },
        { x: 360, y: 370 }, { x: 415, y: 300 }, { x: 475, y: 235 },
        { x: 500, y: 190 }, { x: 430, y: 130 }, { x: 345, y: 80 },
        { x: 250, y: 45 }, { x: 150, y: 15 }, { x: 60, y: -5 },
      ]),
      simpleStroke("lift, return near the centre, and draw the long right-falling stroke", [
        { x: 750, y: 600 }, { x: 700, y: 470 }, { x: 625, y: 340 },
        { x: 520, y: 205 }, { x: 650, y: 105 }, { x: 790, y: 40 },
        { x: 940, y: -5 },
      ]),
    ],
    source: chineseCharacterSource("文"),
  }],
    ["chinese:国", {
    script: "chinese",
    glyph: "国",
    strokes: [
      simpleStroke("draw the outer left vertical from top to bottom", [
        { x: 125, y: 780 }, { x: 125, y: 620 }, { x: 125, y: 460 },
        { x: 125, y: 300 }, { x: 125, y: 140 }, { x: 125, y: -45 },
      ]),
      simpleStroke("lift, then draw the outer top and turn down the right side without lifting", [
        { x: 125, y: 780 }, { x: 250, y: 780 }, { x: 380, y: 780 },
        { x: 510, y: 780 }, { x: 640, y: 780 }, { x: 760, y: 780 },
        { x: 875, y: 780 }, { x: 875, y: 620 }, { x: 875, y: 460 },
        { x: 875, y: 300 }, { x: 875, y: 140 }, { x: 875, y: -45 },
      ]),
      simpleStroke("lift, then draw 玉's top horizontal left to right", [
        { x: 240, y: 610 }, { x: 340, y: 610 }, { x: 445, y: 610 },
        { x: 550, y: 610 }, { x: 650, y: 610 }, { x: 755, y: 610 },
      ]),
      simpleStroke("lift, then draw 玉's middle horizontal left to right", [
        { x: 270, y: 405 }, { x: 420, y: 405 }, { x: 575, y: 405 },
        { x: 730, y: 405 },
      ]),
      simpleStroke("lift, then draw 玉's central vertical from top to bottom", [
        { x: 495, y: 600 }, { x: 495, y: 500 }, { x: 495, y: 400 },
        { x: 495, y: 290 }, { x: 495, y: 180 },
      ]),
      simpleStroke("lift, then draw 玉's bottom horizontal left to right", [
        { x: 230, y: 180 }, { x: 365, y: 180 }, { x: 500, y: 180 },
        { x: 640, y: 180 }, { x: 775, y: 180 },
      ]),
      simpleStroke("lift, then add 玉's short dot down and right", [
        { x: 625, y: 330 }, { x: 665, y: 290 }, { x: 705, y: 250 },
      ]),
      simpleStroke("lift, then close the outer frame along the bottom left to right", [
        { x: 125, y: 10 }, { x: 310, y: 10 }, { x: 500, y: 10 },
        { x: 690, y: 10 }, { x: 875, y: 10 },
      ]),
    ],
    source: chineseCharacterSource("国"),
  }],
    ["chinese:看", {
    script: "chinese",
    glyph: "看",
    strokes: [
      simpleStroke("draw the short top left-falling stroke", [
        { x: 830, y: 805 }, { x: 750, y: 790 }, { x: 650, y: 780 },
        { x: 525, y: 768 }, { x: 400, y: 760 }, { x: 275, y: 755 },
        { x: 135, y: 755 },
      ]),
      simpleStroke("lift, then draw the upper horizontal left to right", [
        { x: 140, y: 632 }, { x: 300, y: 632 }, { x: 500, y: 632 },
        { x: 700, y: 632 }, { x: 875, y: 632 },
      ]),
      simpleStroke("lift, then draw the next horizontal left to right", [
        { x: 70, y: 496 }, { x: 250, y: 496 }, { x: 500, y: 496 },
        { x: 750, y: 496 }, { x: 930, y: 496 },
      ]),
      simpleStroke("lift, then draw the long left-falling stroke toward the lower component", [
        { x: 455, y: 750 }, { x: 425, y: 660 }, { x: 390, y: 570 },
        { x: 350, y: 485 }, { x: 300, y: 400 }, { x: 240, y: 325 },
        { x: 175, y: 260 }, { x: 110, y: 215 }, { x: 50, y: 180 },
      ]),
      simpleStroke("lift, then draw 目's left vertical from top to bottom", [
        { x: 296, y: 365 }, { x: 296, y: 280 }, { x: 296, y: 190 },
        { x: 296, y: 100 }, { x: 296, y: 10 }, { x: 296, y: -60 },
      ]),
      simpleStroke("lift, draw 目's top horizontal, then turn down the right side without lifting", [
        { x: 296, y: 365 }, { x: 420, y: 365 }, { x: 550, y: 365 },
        { x: 680, y: 365 }, { x: 805, y: 365 }, { x: 805, y: 275 },
        { x: 805, y: 180 }, { x: 805, y: 85 }, { x: 805, y: -50 },
      ]),
      simpleStroke("lift, then draw 目's first inner horizontal left to right", [
        { x: 315, y: 240 }, { x: 430, y: 240 }, { x: 550, y: 240 },
        { x: 675, y: 240 }, { x: 790, y: 240 },
      ]),
      simpleStroke("lift, then draw 目's second inner horizontal left to right", [
        { x: 315, y: 118 }, { x: 430, y: 118 }, { x: 550, y: 118 },
        { x: 675, y: 118 }, { x: 790, y: 118 },
      ]),
      simpleStroke("lift, then close 目 with its bottom horizontal left to right", [
        { x: 310, y: -12 }, { x: 430, y: -12 }, { x: 550, y: -12 },
        { x: 675, y: -12 }, { x: 790, y: -12 },
      ]),
    ],
    source: chineseCharacterSource("看"),
  }],
    ["chinese:书", {
    script: "chinese",
    glyph: "书",
    strokes: [
      simpleStroke("draw the short upper horizontal, then fold down without lifting", [
        { x: 140, y: 630 }, { x: 280, y: 630 }, { x: 440, y: 630 },
        { x: 600, y: 630 }, { x: 760, y: 630 }, { x: 760, y: 540 },
        { x: 760, y: 450 }, { x: 760, y: 365 },
      ]),
      simpleStroke("lift, draw the second horizontal, then fold down and finish with the hook without lifting", [
        { x: 70, y: 360 }, { x: 250, y: 360 }, { x: 470, y: 360 },
        { x: 700, y: 360 }, { x: 900, y: 360 }, { x: 905, y: 280 },
        { x: 900, y: 200 }, { x: 890, y: 125 }, { x: 870, y: 75 },
        { x: 835, y: 55 }, { x: 785, y: 52 }, { x: 720, y: 55 },
        { x: 650, y: 60 },
      ]),
      simpleStroke("lift, then draw the long central upright from top to bottom", [
        { x: 456, y: 820 }, { x: 456, y: 650 }, { x: 456, y: 475 },
        { x: 456, y: 300 }, { x: 456, y: 120 }, { x: 456, y: -60 },
      ]),
      simpleStroke("lift, then add the small upper-right dot", [
        { x: 740, y: 780 }, { x: 790, y: 745 }, { x: 840, y: 710 },
        { x: 900, y: 665 }, { x: 925, y: 645 },
      ]),
    ],
    source: chineseCharacterSource("书"),
  }],
];
