// Authored chinese ductus records. This is the stable source-ownership boundary.

import type { Point, Stroke, StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import chinese from "../../../../../learning/human-languages/data/scripts/chinese.json";

const chineseCharacterSource = (glyph: string): StrokeSource => {
  const letter = chinese.letters.find((candidate) => candidate.glyph === glyph);
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Chinese ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const simpleStroke = (label: string, path: Point[]): Stroke => ({
  segments: [{ label, path }],
});

const compactPath = (encodedCoordinates: string): Point[] => {
  const coordinates = encodedCoordinates.split(" ").map(Number);
  if (coordinates.length % 2 !== 0) {
    throw new Error("compactPath expects x/y pairs");
  }

  const path: Point[] = [];
  for (let index = 0; index < coordinates.length; index += 2) {
    path.push({ x: coordinates[index]!, y: coordinates[index + 1]! });
  }
  return path;
};

export const entries: DuctusEntry[] = [
  // Hanzi Writer Data's ordered medians draw 人 with the left-falling stroke
  // first, then restart at the central junction for the right-falling stroke.
  // The source's Arphic-derived proportions are fitted to the vendored Noto
  // Sans SC outline while preserving both directions and the intervening lift.
  [
    "chinese:人",
    {
      script: "chinese",
      glyph: "人",
      strokes: [
        {
          segments: [
            {
              label: "draw the left-falling piě stroke from the upper centre",
              path: compactPath(
                "500 810 500 740 490 650 470 555 445 465 410 375 365 285 310 200 245 120 175 55 105 5 65 -25",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the right-falling nà stroke from the junction",
              path: compactPath(
                "500 690 515 620 535 535 565 445 605 355 655 265 715 180 785 105 860 45 925 0",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("人"),
    },
  ],
  // The compressed person radical keeps the source dataset's two-run order:
  // a long left-falling stroke, then a separately started vertical. Its Noto
  // Sans SC fit follows the glyph's narrow left-side proportions rather than
  // mechanically squeezing the full 人 path.
  [
    "chinese:亻",
    {
      script: "chinese",
      glyph: "亻",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the left-falling piě stroke from upper right to lower left",
              path: compactPath(
                "440 820 430 790 415 755 395 720 375 680 350 640 325 600 295 560 265 520 230 475 195 435 160 395 125 360 95 330 75 305",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the vertical shù stroke from the junction to the baseline",
              path: compactPath(
                "310 590 310 550 310 500 310 440 310 370 310 295 310 220 310 140 310 60 310 -50",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("亻"),
    },
  ],
  // 口 establishes the first Chinese joined corner in the authored inventory:
  // descend the left side, join the top and right side in one héngzhé run, then
  // close the bottom last. The flat Noto fit preserves those three source runs.
  [
    "chinese:口",
    {
      script: "chinese",
      glyph: "口",
      strokes: [
        {
          segments: [
            {
              label: "draw the left vertical shù stroke from top to bottom",
              path: compactPath(
                "166 700 166 620 166 530 166 440 166 350 166 260 166 170 166 80 166 -35",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the top bar from left to right",
              path: compactPath(
                "166 700 260 700 360 700 470 700 580 700 690 700 785 700 835 700",
              ),
            },
            {
              label:
                "turn the corner without lifting and descend the right side",
              path: compactPath(
                "835 700 835 610 835 520 835 430 835 340 835 250 835 160 835 70 835 -30",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then close the bottom from left to right",
              path: compactPath(
                "166 70 260 70 360 70 470 70 580 70 690 70 785 70 835 70",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("口"),
    },
  ],
  // 女 begins with one bent piědiǎn run: descend down-left, turn at the lower
  // junction, and sweep down-right without lifting. A separately started
  // left-falling piě comes next, then the middle héng crosses left-to-right.
  // The four movements follow the three pinned medians on the Noto Sans SC fit.
  [
    "chinese:女",
    {
      script: "chinese",
      glyph: "女",
      strokes: [
        {
          segments: [
            {
              label: "draw the first piědiǎn stroke down and left",
              path: compactPath(
                "460 840 440 790 415 720 390 650 365 580 340 510 310 440 285 375 255 320 220 275",
              ),
            },
            {
              label: "turn without lifting and sweep down to the lower right",
              path: compactPath(
                "220 275 300 265 400 220 500 175 600 125 700 75 800 20 890 -35",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the left-falling piě stroke from upper right to lower left",
              path: compactPath(
                "717 550 700 490 680 430 650 360 615 295 570 235 520 180 460 125 390 75 310 30 220 -10 130 -45",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the middle horizontal héng from left to right",
              path: compactPath(
                "70 561 180 561 300 561 420 561 540 561 660 561 780 561 890 561 940 561",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("女"),
    },
  ],
  // 子 has two joined turns across its first two strokes: the top horizontal
  // turns down-left, then a separately started vertical hooks left at the base.
  // A second lift precedes the final middle horizontal from left to right.
  [
    "chinese:子",
    {
      script: "chinese",
      glyph: "子",
      strokes: [
        {
          segments: [
            {
              label: "draw the top horizontal héng from left to right",
              path: compactPath(
                "160 735 250 735 350 735 450 735 550 735 650 735 740 735 790 735",
              ),
            },
            {
              label: "turn without lifting and sweep down-left",
              path: compactPath(
                "790 735 750 680 700 640 650 600 600 565 550 535 490 515",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the central vertical",
              path: compactPath(
                "504 530 504 460 504 380 504 300 504 220 504 140 504 70 500 20 500 -35",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath("500 -35 450 -40 390 -40 330 -35 285 -20"),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the middle horizontal héng from left to right",
              path: compactPath(
                "60 357 170 357 290 357 410 357 530 357 650 357 770 357 890 357 945 357",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("子"),
    },
  ],
  // 日 starts with the left side, then joins the top bar to the right side in
  // one héngzhé stroke. The inside bar precedes a separately closing bottom.
  [
    "chinese:日",
    {
      script: "chinese",
      glyph: "日",
      strokes: [
        {
          segments: [
            {
              label: "descend the left vertical shù from top to bottom",
              path: compactPath(
                "214 735 214 630 214 520 214 410 214 300 214 190 214 80 214 0",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the top horizontal héng from left to right",
              path: compactPath(
                "214 735 310 735 410 735 510 735 610 735 710 735 792 735",
              ),
            },
            {
              label: "turn without lifting and descend the right side",
              path: compactPath(
                "792 735 792 630 792 520 792 410 792 300 792 190 792 80 792 0",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the middle horizontal héng from left to right",
              path: compactPath(
                "214 389 310 389 410 389 510 389 610 389 710 389 792 389",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then close the bottom horizontal héng from left to right",
              path: compactPath(
                "214 33 310 33 410 33 510 33 610 33 710 33 792 33",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("日"),
    },
  ],
  // 讠 starts with a down-right dot. After one lift, the short horizontal,
  // vertical descent, and rising finish stay joined inside one second stroke.
  [
    "chinese:讠",
    {
      script: "chinese",
      glyph: "讠",
      strokes: [
        {
          segments: [
            {
              label: "draw the top dot down and right",
              path: compactPath("150 780 180 755 215 720 250 685 290 645"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the short horizontal from left to right",
              path: compactPath(
                "60 492 110 492 160 492 210 492 255 492 293 492",
              ),
            },
            {
              label: "turn without lifting and descend the vertical",
              path: compactPath(
                "293 492 293 410 293 320 293 230 293 140 293 60 293 20",
              ),
            },
            {
              label: "turn without lifting and rise to the upper right",
              path: compactPath("293 20 330 35 370 60 410 85 445 110 475 140"),
            },
          ],
        },
      ],
      source: chineseCharacterSource("讠"),
    },
  ],
  // 氵 stacks two separately drawn down-right dots above a third stroke that
  // begins at the bottom and rises to the upper right. The three pinned
  // medians remain separate while fitting the narrow Noto Sans SC radical.
  [
    "chinese:氵",
    {
      script: "chinese",
      glyph: "氵",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper dot down and right",
              path: compactPath(
                "155 785 195 770 235 745 275 720 315 695 350 675",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the middle dot down and right",
              path: compactPath(
                "72 515 110 505 150 485 190 465 230 445 270 420",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then begin the bottom stroke with a slight rise left",
              path: compactPath("158 -58 150 -32 155 0"),
            },
            {
              label:
                "continue without lifting in a long rise to the upper right",
              path: compactPath(
                "155 0 185 45 220 95 255 145 290 195 325 245 360 295",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("氵"),
    },
  ],
  // 宀 places its top dot first, then a separate down-left stroke on the left.
  // After the second lift, the roof crosses left-to-right and hooks down-left
  // without breaking. The Noto fit keeps that source order and joined hook.
  [
    "chinese:宀",
    {
      script: "chinese",
      glyph: "宀",
      strokes: [
        {
          segments: [
            {
              label: "draw the top dot down and right",
              path: compactPath(
                "440 805 455 790 470 770 485 750 500 730 515 715",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the left-side stroke down and left",
              path: compactPath(
                "150 660 145 625 138 585 130 545 122 505 112 475",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the horizontal roof from left to right",
              path: compactPath(
                "150 646 250 646 360 646 470 646 580 646 690 646 790 646 875 646",
              ),
            },
            {
              label: "hook down and left without lifting",
              path: compactPath(
                "875 646 880 620 875 585 865 545 850 505 833 475",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("宀"),
    },
  ],
  // 你 writes 亻 first, then the five strokes of 尔: a falling stroke, a
  // joined horizontal hook, a joined vertical hook, and two separate dots.
  // The seven Noto-fitted runs preserve that component order and six lifts.
  [
    "chinese:你",
    {
      script: "chinese",
      glyph: "你",
      strokes: [
        {
          segments: [
            {
              label: "draw the left-falling stroke of the person radical",
              path: compactPath(
                "300 810 285 760 265 705 240 650 210 595 175 540 140 495 105 455 70 425 45 410",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then descend the vertical stroke of the person radical",
              path: compactPath(
                "196 605 196 520 196 430 196 340 196 250 196 160 196 70 196 -50",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the upper-right left-falling stroke",
              path: compactPath(
                "500 810 490 765 475 715 455 660 430 605 405 550 375 500 345 455 325 435",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the upper horizontal from left to right",
              path: compactPath(
                "450 612 530 612 620 612 710 612 800 612 890 612",
              ),
            },
            {
              label: "hook down and left without lifting",
              path: compactPath("890 612 900 580 900 540 895 500 885 465"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the central vertical",
              path: compactPath(
                "649 590 649 500 649 400 649 300 649 200 649 100 649 15 645 -30",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath("645 -30 615 -40 580 -42 545 -40 515 -30"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the lower-left dot down and left",
              path: compactPath(
                "485 380 470 330 450 275 425 220 400 165 370 110 345 80",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the lower-right dot down and right",
              path: compactPath(
                "790 380 815 335 840 285 865 235 885 185 900 135 915 90",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("你"),
    },
  ],
  // 好 writes all three strokes of 女 before the three strokes of 子. The
  // first strokes of both components turn without lifting, and 子's vertical
  // keeps its base hook joined. Six Noto-fitted runs preserve five lifts.
  [
    "chinese:好",
    {
      script: "chinese",
      glyph: "好",
      strokes: [
        {
          segments: [
            {
              label: "draw 女's first bent stroke down and left",
              path: compactPath(
                "218 820 205 750 190 675 175 600 155 520 135 440 120 365 100 320 82 300",
              ),
            },
            {
              label: "turn without lifting and sweep right",
              path: compactPath(
                "82 300 145 270 205 225 265 175 325 120 375 70 410 40",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 女's left-falling stroke",
              path: compactPath(
                "390 620 380 550 365 475 345 395 320 310 290 225 255 150 215 80 165 20 95 -45",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 女's horizontal stroke left to right",
              path: compactPath(
                "45 600 100 600 160 600 220 600 280 600 335 600 370 600",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 子's top horizontal left to right",
              path: compactPath(
                "485 730 555 730 630 730 705 730 780 730 850 730",
              ),
            },
            {
              label: "turn without lifting and sweep down-left",
              path: compactPath(
                "850 730 840 700 820 665 790 625 755 585 715 545 680 520",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 子's vertical stroke",
              path: compactPath(
                "700 520 700 440 700 350 700 260 700 170 700 80 700 15 695 -25",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath("695 -25 665 -35 625 -40 585 -38 545 -25"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 子's middle horizontal left to right",
              path: compactPath(
                "440 380 520 380 610 380 700 380 790 380 880 380 950 380",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("好"),
    },
  ],
  // 我 has seven sourced strokes. Only the vertical and its base hook remain
  // joined; the long curved slash also hooks upward without lifting, producing
  // nine visible movements and six pen lifts.
  [
    "chinese:我",
    {
      script: "chinese",
      glyph: "我",
      strokes: [
        {
          segments: [
            {
              label: "draw the short upper-left falling stroke",
              path: compactPath(
                "450 800 390 785 325 770 255 755 185 740 105 720",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the upper horizontal left to right",
              path: compactPath(
                "65 510 180 510 300 510 420 510 540 510 660 510 780 510 900 510 940 510",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the vertical stroke",
              path: compactPath(
                "307 720 307 620 307 520 307 420 307 320 307 220 307 120 307 20 302 -25",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath("302 -25 275 -35 235 -40 195 -38 155 -25"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the lower rising stroke",
              path: compactPath(
                "55 215 120 230 190 245 265 262 340 280 415 298 490 315",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the long curved slash down and right",
              path: compactPath(
                "600 810 600 700 605 590 615 480 635 365 660 255 700 150 750 65 805 5 850 -35 875 -45",
              ),
            },
            {
              label: "hook upward on the right without lifting",
              path: compactPath("875 -45 895 5 905 55 915 105 925 145"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the separate rising slash up and left",
              path: compactPath(
                "850 390 815 325 770 260 720 200 660 140 595 85 525 35 455 -5",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the upper-right dot down and right",
              path: compactPath(
                "755 785 785 755 815 720 845 685 875 650 895 625",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("我"),
    },
  ],
  // 是 closes 日 in four strokes before drawing the five-stroke lower body.
  // Only 日's top-right corner remains joined: nine strokes, eight lifts, and
  // ten visible movements on the Noto Sans SC fit.
  [
    "chinese:是",
    {
      script: "chinese",
      glyph: "是",
      strokes: [
        {
          segments: [
            {
              label: "draw 日's left vertical",
              path: compactPath(
                "200 770 200 710 200 650 200 590 200 530 200 490",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 日's top horizontal",
              path: compactPath(
                "200 760 300 760 400 760 500 760 600 760 700 760 795 760",
              ),
            },
            {
              label: "turn down the right side without lifting",
              path: compactPath(
                "795 760 795 700 795 640 795 580 795 520 795 490",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 日's inner horizontal",
              path: compactPath(
                "235 634 330 634 430 634 530 634 630 634 730 634 760 634",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then close 日 with the bottom horizontal",
              path: compactPath(
                "235 500 330 500 430 500 530 500 630 500 730 500 760 500",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the wide middle horizontal",
              path: compactPath(
                "65 365 180 365 300 365 420 365 540 365 660 365 780 365 900 365 940 365",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the central vertical",
              path: compactPath(
                "508 350 508 300 508 245 508 190 508 130 508 70 508 10",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the short lower-right horizontal",
              path: compactPath(
                "510 185 580 185 650 185 720 185 790 185 850 185",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the lower-left falling stroke",
              path: compactPath(
                "265 280 250 230 230 180 205 130 175 85 140 45 100 10 60 -25",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the long finishing stroke down and right",
              path: compactPath(
                "245 180 280 140 320 105 370 65 430 25 500 -5 580 -25 670 -25 760 -25 850 -25 920 -20",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("是"),
    },
  ],
  // 不 places four independent strokes: top horizontal, long falling stroke,
  // central vertical, then the right-falling dot. Four strokes mean three
  // lifts and four visible movements on the Noto Sans SC fit.
  [
    "chinese:不",
    {
      script: "chinese",
      glyph: "不",
      strokes: [
        {
          segments: [
            {
              label: "draw the top horizontal left-to-right",
              path: compactPath(
                "85 730 200 730 320 730 440 730 560 730 680 730 800 730 915 730",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the long stroke down-left",
              path: compactPath(
                "545 710 525 650 490 590 440 525 380 460 315 395 245 330 175 275 105 225",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the central vertical",
              path: compactPath(
                "500 550 500 475 500 400 500 325 500 250 500 175 500 100 500 20 500 -55",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the separate right-falling dot",
              path: compactPath(
                "610 470 660 430 715 390 770 350 825 305 875 260 920 220",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("不"),
    },
  ],
  // 名 completes 夕 before 口. The second 夕 stroke joins its horizontal to the
  // long down-left fall, and 口 joins its top to the right side: six strokes,
  // five lifts, and eight visible movements on the Noto Sans SC fit.
  [
    "chinese:名",
    {
      script: "chinese",
      glyph: "名",
      strokes: [
        {
          segments: [
            {
              label: "draw 夕's upper left-falling stroke",
              path: compactPath(
                "445 820 420 775 385 730 340 685 285 640 225 600 165 560 105 525",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 夕's horizontal",
              path: compactPath(
                "350 705 440 705 530 705 620 705 710 705 775 705",
              ),
            },
            {
              label: "continue down-left without lifting",
              path: compactPath(
                "775 705 745 650 700 590 640 530 570 470 490 415 400 360 305 315 205 275 110 240",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place 夕's inner down-right dot",
              path: compactPath(
                "300 540 330 515 365 490 400 460 435 430 470 400",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 口's left side",
              path: compactPath(
                "290 305 290 245 290 185 290 125 290 65 290 5 290 -50",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 口's top horizontal",
              path: compactPath(
                "300 305 385 305 470 305 555 305 640 305 725 305 810 305",
              ),
            },
            {
              label: "turn down the right side without lifting",
              path: compactPath(
                "810 305 810 245 810 185 810 125 810 65 810 5 810 -50",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then close 口 with the bottom horizontal",
              path: compactPath("300 5 385 5 470 5 555 5 640 5 725 5 800 5"),
            },
          ],
        },
      ],
      source: chineseCharacterSource("名"),
    },
  ],
  // 字 writes 宀 before 子. The roof ends in one joined hook; 子 then keeps its
  // top turn and vertical base hook joined: six strokes, five lifts, and nine
  // visible movements on the Noto Sans SC fit.
  [
    "chinese:字",
    {
      script: "chinese",
      glyph: "字",
      strokes: [
        {
          segments: [
            {
              label: "draw 宀's top dot down-right",
              path: compactPath("455 825 475 800 495 775 520 750 545 725"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 宀's left-side stroke down-left",
              path: compactPath("125 690 120 650 115 610 105 570 95 535"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 宀's horizontal roof",
              path: compactPath(
                "140 700 250 700 360 700 470 700 580 700 690 700 800 700 880 700",
              ),
            },
            {
              label: "hook down-left without lifting",
              path: compactPath("880 700 875 660 865 620 855 580 850 545"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 子's top horizontal",
              path: compactPath(
                "260 515 345 515 430 515 515 515 600 515 685 515 735 515",
              ),
            },
            {
              label: "turn down-left without lifting",
              path: compactPath(
                "735 515 700 480 660 445 615 410 570 380 525 350 490 330",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 子's vertical",
              path: compactPath(
                "500 350 500 290 500 230 500 170 500 110 500 50 500 5",
              ),
            },
            {
              label: "hook left without lifting",
              path: compactPath(
                "500 5 480 -20 450 -35 410 -40 365 -40 325 -35",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 子's middle horizontal",
              path: compactPath(
                "85 265 200 265 315 265 430 265 545 265 660 265 775 265 900 265",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("字"),
    },
  ],
  // 谢 writes 讠, then 身, then 寸. Its twelve cited strokes preserve the two
  // turns in 讠's second run, 身's two-turn enclosure, and 寸's base hook:
  // twelve strokes, eleven lifts, and seventeen visible movements.
  [
    "chinese:谢",
    {
      script: "chinese",
      glyph: "谢",
      strokes: [
        {
          segments: [
            {
              label: "draw 讠's top dot down-right",
              path: compactPath("90 780 120 755 150 725 185 690 225 650"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 讠's short horizontal",
              path: compactPath("50 490 85 490 120 490 155 490 195 490"),
            },
            {
              label: "turn down without lifting",
              path: compactPath(
                "195 490 195 400 195 300 195 200 195 100 195 10",
              ),
            },
            {
              label: "turn and finish rising up-right without lifting",
              path: compactPath("195 10 225 25 255 50 285 80 315 115 335 145"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 身's upper falling stroke",
              path: compactPath("505 825 500 790 485 755 465 720 440 685"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 身's left side",
              path: compactPath(
                "375 680 375 600 375 520 375 440 375 360 375 285",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 身's top horizontal",
              path: compactPath("405 695 450 695 495 695 540 695 580 695"),
            },
            {
              label: "turn and descend 身's right side without lifting",
              path: compactPath(
                "580 695 580 575 580 455 580 335 580 215 580 95 580 10",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath("580 10 565 -10 540 -25 510 -35 475 -35"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 身's upper inner horizontal",
              path: compactPath(
                "390 565 430 565 470 565 510 565 550 565 580 565",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 身's lower inner horizontal",
              path: compactPath(
                "390 430 430 430 470 430 510 430 550 430 580 430",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 身's wide lower horizontal",
              path: compactPath(
                "290 285 350 285 410 285 470 285 530 285 585 285",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 身's lower falling stroke down-left",
              path: compactPath(
                "535 270 510 220 480 170 440 120 395 75 345 30 295 -10",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 寸's horizontal",
              path: compactPath(
                "650 585 700 585 750 585 800 585 850 585 900 585 950 585",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 寸's vertical",
              path: compactPath(
                "855 825 855 700 855 575 855 450 855 325 855 200 855 75 855 5",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath(
                "855 5 840 -15 815 -30 785 -40 750 -40 720 -35",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place 寸's dot down-right",
              path: compactPath("680 430 700 390 720 350 740 310 760 270"),
            },
          ],
        },
      ],
      source: chineseCharacterSource("谢"),
    },
  ],
  // 请 writes 讠 before 青. The speech radical keeps both turns inside its
  // second run; 青 closes with a joined top, right side, and leftward base hook:
  // ten strokes, nine lifts, and fourteen visible movements.
  [
    "chinese:请",
    {
      script: "chinese",
      glyph: "请",
      strokes: [
        {
          segments: [
            {
              label: "draw 讠's top dot down-right",
              path: compactPath("135 780 165 750 195 715 225 680 255 650"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 讠's short horizontal",
              path: compactPath(
                "45 490 80 490 120 490 160 490 200 490 235 490",
              ),
            },
            {
              label: "turn down without lifting",
              path: compactPath(
                "235 490 235 400 235 300 235 200 235 100 235 10",
              ),
            },
            {
              label: "turn and finish rising up-right without lifting",
              path: compactPath("235 10 265 25 295 50 330 80 360 110 390 145"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 青's top horizontal",
              path: compactPath(
                "385 735 475 735 565 735 655 735 745 735 835 735 925 735",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 青's second horizontal",
              path: compactPath(
                "410 610 490 610 570 610 650 610 730 610 810 610 895 610",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 青's upper vertical",
              path: compactPath(
                "650 835 650 765 650 695 650 625 650 555 650 485",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 青's wide middle horizontal",
              path: compactPath(
                "355 485 455 485 555 485 655 485 755 485 855 485 955 485",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 青's lower left side",
              path: compactPath(
                "460 370 460 295 460 220 460 145 460 70 460 -5 460 -70",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 青's lower top horizontal",
              path: compactPath(
                "490 370 550 370 610 370 670 370 730 370 790 370 845 370",
              ),
            },
            {
              label: "turn and descend the right side without lifting",
              path: compactPath("845 370 845 295 845 220 845 145 845 70 845 5"),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath(
                "845 5 830 -15 805 -30 775 -40 740 -40 705 -35",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 青's upper inner horizontal",
              path: compactPath(
                "480 235 540 235 600 235 660 235 720 235 780 235 825 235",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 青's lower inner horizontal",
              path: compactPath(
                "480 100 540 100 600 100 660 100 720 100 780 100 825 100",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("请"),
    },
  ],
  // 再 opens with the upper horizontal, then builds the central frame before
  // closing with the long bottom bar: six strokes, five lifts, eight movements.
  [
    "chinese:再",
    {
      script: "chinese",
      glyph: "再",
      strokes: [
        {
          segments: [
            {
              label: "draw the top horizontal left-to-right",
              path: compactPath(
                "80 745 220 745 360 745 500 745 640 745 780 745 920 745",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the left side",
              path: compactPath(
                "195 575 195 475 195 375 195 275 195 175 195 75 195 -70",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the frame's top horizontal",
              path: compactPath(
                "225 575 315 575 405 575 495 575 585 575 675 575 800 575",
              ),
            },
            {
              label: "turn and descend the right side without lifting",
              path: compactPath(
                "800 575 800 475 800 375 800 275 800 175 800 75 800 10",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath(
                "800 10 785 -10 760 -25 730 -35 695 -35 655 -30",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the central vertical",
              path: compactPath(
                "495 755 495 665 495 575 495 485 495 395 495 305 495 205",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the inner horizontal",
              path: compactPath(
                "210 390 310 390 410 390 510 390 610 390 710 390 790 390",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then close with the long bottom horizontal",
              path: compactPath(
                "45 195 195 195 345 195 495 195 645 195 795 195 955 195",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("再"),
    },
  ],
  // 见 completes its open upper frame before drawing the two lower runs:
  // four strokes, three lifts, seven movements.
  [
    "chinese:见",
    {
      script: "chinese",
      glyph: "见",
      strokes: [
        {
          segments: [
            {
              label: "descend the frame's left side",
              path: compactPath(
                "215 755 215 670 215 585 215 500 215 415 215 330 215 235",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the frame's top horizontal",
              path: compactPath(
                "220 745 310 745 400 745 490 745 580 745 670 745 780 745",
              ),
            },
            {
              label: "turn and descend the right side without lifting",
              path: compactPath(
                "780 745 780 660 780 575 780 490 780 405 780 320 780 235",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the inner left-falling leg",
              path: compactPath(
                "490 600 490 520 485 430 475 340 450 250 420 170 380 100 325 45 260 0 180 -40 90 -65",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the second leg",
              path: compactPath(
                "555 285 555 235 555 185 555 135 555 85 555 55",
              ),
            },
            {
              label: "bend right along the base without lifting",
              path: compactPath(
                "555 55 570 20 610 -10 665 -20 725 -20 785 -15 835 5 885 40",
              ),
            },
            {
              label: "finish with an upward hook without lifting",
              path: compactPath(
                "885 40 895 70 905 100 915 125 925 145 930 150",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("见"),
    },
  ],
  // 什 completes both strokes of 亻 before writing 十: four separate strokes,
  // three lifts, and four movements.
  [
    "chinese:什",
    {
      script: "chinese",
      glyph: "什",
      strokes: [
        {
          segments: [
            {
              label:
                "draw 亻's left-falling stroke from the upper centre down-left",
              path: compactPath(
                "280 810 265 760 245 700 220 640 190 580 155 525 120 480 85 450 50 430",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 亻's vertical stroke to the baseline",
              path: compactPath(
                "225 590 225 480 225 370 225 260 225 150 225 40 225 -65",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 十's horizontal stroke left-to-right",
              path: compactPath(
                "340 457 440 457 540 457 640 457 740 457 840 457 940 457",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then descend 十's vertical stroke through the horizontal",
              path: compactPath(
                "646 810 646 680 646 550 646 420 646 290 646 160 646 30 646 -65",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("什"),
    },
  ],
  // 么 places its upper falling stroke, joins the second fall to its rightward
  // base sweep, then adds the final dot: three strokes, two lifts, four movements.
  [
    "chinese:么",
    {
      script: "chinese",
      glyph: "么",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper left-falling stroke down-left",
              path: compactPath(
                "475 805 455 755 420 700 375 640 325 580 270 520 215 470 165 430 120 400 75 410",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the second left-falling stroke down-left",
              path: compactPath(
                "650 580 620 520 575 450 520 375 455 300 390 225 325 155 260 95 205 50 175 30",
              ),
            },
            {
              label: "turn and sweep right along the base without lifting",
              path: compactPath(
                "175 30 270 35 380 45 490 55 600 70 705 85 805 105",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the final dot down-right",
              path: compactPath(
                "670 295 715 245 760 185 805 125 845 65 885 5 905 -30",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("么"),
    },
  ],
  // 早 completes 日 before writing 十 below it. The top and right sides of 日
  // stay joined: six strokes, five lifts, and seven learner movements.
  [
    "chinese:早",
    {
      script: "chinese",
      glyph: "早",
      strokes: [
        {
          segments: [
            {
              label: "descend 日's left side from the upper left",
              path: compactPath(
                "189 759 189 690 189 620 189 550 189 480 189 412",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 日's top horizontal left-to-right",
              path: compactPath(
                "189 759 290 759 395 759 500 759 605 759 710 759 806 759",
              ),
            },
            {
              label: "turn without lifting and descend 日's right side",
              path: compactPath(
                "806 759 806 690 806 620 806 550 806 480 806 412",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 日's middle horizontal left-to-right",
              path: compactPath(
                "189 587 290 587 395 587 500 587 605 587 710 587 806 587",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then close 日 with its bottom horizontal left-to-right",
              path: compactPath(
                "189 412 290 412 395 412 500 412 605 412 710 412 806 412",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw 十's horizontal left-to-right",
              path: compactPath(
                "60 193 190 193 345 193 500 193 655 193 810 193 944 193",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 十's vertical through the horizontal",
              path: compactPath(
                "496 389 496 310 496 230 496 150 496 70 496 -65",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("早"),
    },
  ],
  // 上 descends its vertical first, then places the short middle horizontal
  // before the long base: three separate strokes, two lifts, three movements.
  [
    "chinese:上",
    {
      script: "chinese",
      glyph: "上",
      strokes: [
        {
          segments: [
            {
              label: "descend the central vertical from top to bottom",
              path: compactPath(
                "466 810 466 680 466 550 466 420 466 290 466 160 466 20",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the short middle horizontal left-to-right",
              path: compactPath(
                "470 478 550 478 630 478 710 478 790 478 868 478",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the long base horizontal left-to-right",
              path: compactPath("65 5 210 5 355 5 500 5 645 5 790 5 936 5"),
            },
          ],
        },
      ],
      source: chineseCharacterSource("上"),
    },
  ],
  // These six entries close the font-checked ductus for the family and school
  // vocabulary tranche. Their paths preserve Hanzi Writer Data's pinned PRC
  // stroke order while fitting the bundled Noto Sans SC outlines.
  [
    "chinese:儿",
    {
      script: "chinese",
      glyph: "儿",
      strokes: [
        simpleStroke(
          "draw the short left-falling piě stroke from the upper centre down-left",
          compactPath(
            "296 752 296 376 288 368 288 312 280 304 272 232 264 224 248 160 208 88 160 32",
          ),
        ),
        simpleStroke(
          "lift, then draw the vertical shù stroke down from the upper right, bend it right along the baseline, and hook upward without lifting",
          compactPath(
            "664 736 664 64 672 56 672 0 680 -16 704 -32 872 -32 888 -24 912 8 912 40 920 48 920 160 904 176 904 192",
          ),
        ),
      ],
      source: chineseCharacterSource("儿"),
    },
  ],
  [
    "chinese:家",
    {
      script: "chinese",
      glyph: "家",
      strokes: [
        simpleStroke(
          "draw the top dot down and right",
          compactPath("432 824 456 808 480 808 488 792 488 768 496 760"),
        ),
        simpleStroke(
          "lift, then draw the left-side roof dot down and left",
          compactPath("224 712 128 712 120 704 120 576"),
        ),
        simpleStroke(
          "lift, then draw the horizontal roof left-to-right and hook down-left without lifting",
          compactPath(
            "264 712 488 712 496 720 512 720 520 712 872 712 880 704 880 616",
          ),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal of the lower body from left to right",
          compactPath("248 552 456 552 464 544 488 544 496 552 640 552"),
        ),
        simpleStroke(
          "lift, then draw its left-falling stroke",
          compactPath(
            "456 504 448 496 448 480 424 456 376 448 304 408 288 408 264 392 248 392 240 384 224 384 192 368 168 368 160 360 112 360",
          ),
        ),
        simpleStroke(
          "lift, then descend the centre and curve to a hook at the base without lifting",
          compactPath(
            "416 456 432 464 448 448 448 440 472 416 488 384 488 368 504 352 512 352 528 328 536 296 552 280 552 224 560 216 560 184 568 176 568 96 560 88 552 32 512 -24 480 -40 456 -48 392 -48 360 -24",
          ),
        ),
        simpleStroke(
          "lift, then add the short left-falling stroke beside the centre",
          compactPath(
            "456 432 472 416 488 384 488 368 496 360 480 344 448 336 392 296 264 232 248 232 232 216 216 216 208 208 192 208 160 192 136 192",
          ),
        ),
        simpleStroke(
          "lift, then add the longer left-falling stroke below it",
          compactPath(
            "512 344 528 328 528 312 552 272 552 224 536 208 520 208 488 192 472 176 464 176 424 144 400 136 384 120 288 72 272 72 216 40 200 40 192 32 176 32 144 16 96 16",
          ),
        ),
        simpleStroke(
          "lift, then add the lower left-falling stroke",
          compactPath(
            "832 432 792 432 776 416 768 416 736 384 680 352 664 320 648 304 624 304 616 296 552 280 536 296 528 328",
          ),
        ),
        simpleStroke(
          "lift, then finish with the long right-falling nà stroke",
          compactPath(
            "528 328 528 312 552 280 568 288 592 288 624 304 648 304 656 312 672 296 672 280 720 184 760 136 760 128 832 56 840 56 888 16 928 16 936 24",
          ),
        ),
      ],
      source: chineseCharacterSource("家"),
    },
  ],
  [
    "chinese:大",
    {
      script: "chinese",
      glyph: "大",
      strokes: [
        simpleStroke(
          "draw the horizontal héng stroke from left to right",
          compactPath("104 512 472 512 488 496 504 496 520 512 896 512"),
        ),
        simpleStroke(
          "lift, then start above the bar and draw the left-falling piě stroke through it",
          compactPath(
            "496 792 496 624 488 616 488 536 472 512 496 488 496 472 464 440 456 424 448 376 440 368 432 328 416 304 416 288 352 176 320 144 320 136 232 48 224 48 200 24 168 8",
          ),
        ),
        simpleStroke(
          "lift, return near the crossing, and draw the long right-falling nà stroke",
          compactPath(
            "464 432 464 440 496 472 536 432 552 400 552 384 576 344 576 328 616 248 632 232 640 208 696 144 696 136 800 32 808 32 832 8 856 0 880 -24 896 -16 920 -16 928 -8",
          ),
        ),
      ],
      source: chineseCharacterSource("大"),
    },
  ],
  [
    "chinese:小",
    {
      script: "chinese",
      glyph: "小",
      strokes: [
        simpleStroke(
          "draw the centre vertical shù stroke downward and hook left without lifting",
          compactPath("504 776 504 48 496 40 496 -8 488 -24 464 -40 384 -40"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling piě stroke",
          compactPath(
            "224 496 216 488 216 472 200 440 200 416 192 408 184 368 96 192",
          ),
        ),
        simpleStroke(
          "lift, then draw the right dot downward",
          compactPath(
            "776 520 792 488 808 472 824 440 824 424 864 352 864 336 880 312 888 272 904 248 920 184",
          ),
        ),
      ],
      source: chineseCharacterSource("小"),
    },
  ],
  [
    "chinese:中",
    {
      script: "chinese",
      glyph: "中",
      strokes: [
        simpleStroke(
          "draw the left vertical shù stroke from top to bottom",
          compactPath("152 616 152 520 152 424 152 328 152 232"),
        ),
        simpleStroke(
          "lift, then draw the top horizontal left-to-right and turn down the right side without lifting",
          compactPath(
            "132 624 320 624 504 624 688 624 864 624 864 520 864 416 864 312 864 220",
          ),
        ),
        simpleStroke(
          "lift, then close the box with the bottom horizontal héng stroke left-to-right",
          compactPath("152 284 320 284 488 284 656 284 824 284"),
        ),
        simpleStroke(
          "lift, then draw the central vertical shù stroke from top through the box to the base",
          compactPath("496 824 496 640 496 456 496 272 496 88 496 -48"),
        ),
      ],
      source: chineseCharacterSource("中"),
    },
  ],
  [
    "chinese:同",
    {
      script: "chinese",
      glyph: "同",
      strokes: [
        simpleStroke(
          "draw the outer left vertical shù stroke from top to bottom",
          compactPath("152 744 152 584 152 424 152 264 152 104 152 -32"),
        ),
        simpleStroke(
          "lift, then draw the outer top horizontal and turn down the right side without lifting",
          compactPath(
            "124 752 312 752 500 752 688 752 877 752 877 584 877 416 877 248 877 48 870 -16 840 -38 780 -45 720 -42",
          ),
        ),
        simpleStroke(
          "lift, then draw the short inner horizontal héng stroke left-to-right",
          compactPath("280 580 392 580 504 580 616 580 720 580"),
        ),
        simpleStroke(
          "lift, then draw 口's left vertical",
          compactPath("336 408 336 312 336 216 336 120"),
        ),
        simpleStroke(
          "lift, then draw 口's top horizontal and turn down its right side without lifting",
          compactPath(
            "352 408 432 408 512 408 592 408 672 408 672 312 672 216 672 136",
          ),
        ),
        simpleStroke(
          "lift, then close 口 with its bottom horizontal left-to-right",
          compactPath("336 152 420 152 504 152 588 152 672 152"),
        ),
      ],
      source: chineseCharacterSource("同"),
    },
  ],
  [
    "chinese:学",
    {
      script: "chinese",
      glyph: "学",
      strokes: [
        simpleStroke(
          "draw the centre top dot downward",
          compactPath("232 768 232 760 272 704 272 648 280 640 384 640"),
        ),
        simpleStroke(
          "lift, then draw the left top dot downward",
          compactPath("448 816 464 816 496 776 520 728 520 688"),
        ),
        simpleStroke(
          "lift, then draw the right top stroke down-left",
          compactPath("768 744 768 736 744 712 736 656 720 640 592 640"),
        ),
        simpleStroke(
          "lift, then draw the left dot of the cover",
          compactPath("236 466 264 464"),
        ),
        simpleStroke(
          "lift, then draw the horizontal cover and hook downward without lifting",
          compactPath("112 568 112 632 120 640 872 640 888 624 888 512"),
        ),
        simpleStroke(
          "lift, then draw 子's top horizontal and turn down-left without lifting",
          compactPath(
            "336 464 656 464 664 456 664 440 656 432 656 416 640 400 504 328",
          ),
        ),
        simpleStroke(
          "lift, then draw 子's vertical and hook left without lifting",
          compactPath(
            "504 328 496 312 496 248 504 240 496 232 496 0 488 -8 488 -24 456 -40 352 -40",
          ),
        ),
        simpleStroke(
          "lift, then draw 子's bottom horizontal from left to right",
          compactPath("96 240 488 240 496 248 504 240 896 240"),
        ),
      ],
      source: chineseCharacterSource("学"),
    },
  ],
  [
    "chinese:生",
    {
      script: "chinese",
      glyph: "生",
      strokes: [
        simpleStroke(
          "draw the short upper left-falling piě stroke",
          compactPath(
            "288 792 256 760 240 696 224 672 224 632 216 624 208 600 192 568 176 536 152 504 134 480",
          ),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal héng stroke",
          compactPath("320 608 488 608 496 600 504 608 856 608"),
        ),
        simpleStroke(
          "lift, then draw the shorter middle horizontal héng stroke",
          compactPath("168 312 488 312 496 304 504 312 824 312"),
        ),
        simpleStroke(
          "lift, then draw the vertical shù stroke through the two bars",
          compactPath(
            "496 792 496 616 504 608 496 600 496 320 504 312 496 304 496 96",
          ),
        ),
        simpleStroke(
          "lift, then draw the long bottom horizontal héng stroke",
          compactPath("144 -16 488 -16 496 -8 504 -16 904 -16"),
        ),
      ],
      source: chineseCharacterSource("生"),
    },
  ],
  // Hanzi Writer Data draws 一 with a single left-to-right héng. There is nothing
  // to lift between, so the ductus is one stroke of one segment -- the shortest
  // entry in this table, and the reason the lesson can say the stroke count IS
  // the number.
  [
    "chinese:一",
    {
      script: "chinese",
      glyph: "一",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the horizontal héng stroke straight across the middle, left to right",
              path: compactPath(
                "52 390 127 390 202 390 277 390 352 390 427 390 502 390 577 390 652 390 727 390 802 390 877 390 952 390",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("一"),
    },
  ],
  // Two héng strokes, top before bottom, the lower one markedly wider. The source's
  // ordered medians give the same two runs; the widths are read off the vendored
  // Noto Sans SC outline rather than the Arphic-derived source graphics.
  [
    "chinese:二",
    {
      script: "chinese",
      glyph: "二",
      strokes: [
        {
          segments: [
            {
              label:
                "draw the upper, shorter horizontal héng stroke from left to right",
              path: compactPath(
                "152 656 210 656 268 656 326 656 384 656 442 656 500 656 558 656 616 656 674 656 732 656 790 656 848 656",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the lower, longer horizontal héng stroke from left to right",
              path: compactPath(
                "66 62 139 61 211 62 284 61 356 62 429 61 501 62 574 61 646 62 719 61 791 62 864 61 936 62",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("二"),
    },
  ],
  // Three héng strokes, ordered top, middle, bottom. The middle is the shortest and
  // the base the widest -- the proportions that stop 三 reading as a tally, and the
  // reason this is the last numeral whose strokes can be counted for its value.
  [
    "chinese:三",
    {
      script: "chinese",
      glyph: "三",
      strokes: [
        {
          segments: [
            {
              label: "draw the top horizontal héng stroke from left to right",
              path: compactPath(
                "172 704 224 705 276 704 329 705 381 704 433 705 485 704 537 705 589 704 642 705 694 704 746 705 798 704",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the middle horizontal héng stroke, the shortest of the three",
              path: compactPath(
                "212 378 258 378 303 378 349 378 394 378 440 378 485 378 531 378 576 378 622 378 667 378 713 378 758 378",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the bottom horizontal héng stroke, the longest of the three",
              path: compactPath(
                "74 31 145 30 216 31 287 30 358 31 429 30 500 31 571 30 642 31 713 30 784 31 855 30 926 31",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("三"),
    },
  ],
  // Five strokes. Medians 1-2 build the box -- the left wall, then the top and right
  // side in ONE turning héngzhé traced here as two joined segments, which is why the
  // corner counts as one stroke and not two. Medians 3-4 are the two inner pieces,
  // and median 5 closes the bottom last.
  [
    "chinese:四",
    {
      script: "chinese",
      glyph: "四",
      strokes: [
        {
          segments: [
            {
              label: "draw the left vertical shù stroke from top to bottom",
              path: compactPath(
                "135 690 126 629 125 568 126 508 125 447 126 386 125 325 126 264 125 203 126 143 134 82 125 21 126 -40",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the horizontal-turning héngzhé stroke across the top",
              path: compactPath(
                "100 706 164 717 229 716 293 717 357 707 421 702 486 717 550 716 614 707 678 716 743 717 807 716 871 707",
              ),
            },
            {
              label: "and down the right side without lifting",
              path: compactPath(
                "871 707 870 630 870 570 870 510 870 450 870 390 870 330 870 270 870 210 870 150 862 90 858 30 870 -30",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the short inner left-falling piě stroke",
              path: compactPath(
                "388 670 386 630 385 591 383 551 379 512 375 472 368 433 358 393 344 353 328 314 308 274 281 235 241 195",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the inner stroke down",
              path: compactPath(
                "600 670 600 643 600 615 600 588 600 560 600 533 600 505 600 478 600 450 600 423 600 395 600 368 602 340",
              ),
            },
            {
              label: "and turning up to the right at its foot",
              path: compactPath(
                "602 340 618 312 635 312 652 290 669 289 686 288 703 289 720 288 737 289 754 288 771 289 788 290 805 292",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then close the bottom with a horizontal héng stroke from left to right",
              path: compactPath(
                "175 65 229 65 283 65 337 65 391 65 445 65 499 65 553 65 607 65 661 65 715 65 769 65 823 65",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("四"),
    },
  ],
  // Four strokes for the number five. The top bar, then a shù descending from it and
  // leaning left, then a héngzhé that crosses right and turns down, then the widest
  // stroke in the character closing it along the bottom. Where the descender crosses
  // the middle bar the traced band is clamped to one stroke's width, or its centre
  // would land between the two.
  [
    "chinese:五",
    {
      script: "chinese",
      glyph: "五",
      strokes: [
        {
          segments: [
            {
              label: "draw the top horizontal héng stroke from left to right",
              path: compactPath(
                "130 706 191 706 253 706 314 706 375 706 436 697 498 705 559 705 620 705 681 705 743 705 804 705 865 705",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the shù stroke descending from the top bar, leaning left",
              path: compactPath(
                "446 665 440 615 432 564 425 514 416 463 416 413 400 363 391 312 382 262 373 211 365 161 355 110 345 60",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the horizontal-turning héngzhé stroke across to the right",
              path: compactPath(
                "190 414 235 414 281 414 326 414 372 405 417 405 463 414 508 414 553 414 599 414 644 414 690 414 735 406",
              ),
            },
            {
              label: "and then down",
              path: compactPath(
                "735 406 724 390 733 360 730 330 727 300 725 270 723 240 720 210 716 180 714 150 710 120 708 90 704 60",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the long bottom horizontal héng stroke, the widest in the character",
              path: compactPath(
                "62 12 135 12 208 12 281 12 354 21 427 11 500 11 573 11 646 11 719 20 792 12 865 12 938 12",
              ),
            },
          ],
        },
      ],
      source: chineseCharacterSource("五"),
    },
  ],
  // These four entries preserve Hanzi Writer Data's pinned medians directly.
  // Their coordinates therefore describe the authoritative stroke centre-lines;
  // the separately rendered Noto Sans SC outline may vary in width and proportion.
  [
    "chinese:汉",
    {
      script: "chinese",
      glyph: "汉",
      strokes: [
        simpleStroke(
          "draw the upper water dot down and right",
          compactPath("115 790 190 750 265 700"),
        ),
        simpleStroke(
          "lift, then draw the middle water dot down and right",
          compactPath("75 520 150 480 225 430"),
        ),
        simpleStroke(
          "lift, then draw the lower water stroke rising up and right",
          compactPath("100 0 130 60 165 125 205 200 250 275"),
        ),
        simpleStroke(
          "lift, draw the upper-right horizontal, then turn and sweep down-left without lifting",
          compactPath(
            "360 735 550 735 750 735 860 725 845 620 810 500 760 390 695 285 610 185 560 110 490 55 400 5 315 -20",
          ),
        ),
        simpleStroke(
          "lift, then draw the long right-falling stroke",
          compactPath("415 700 470 500 540 350 620 220 700 130 820 35 940 -20"),
        ),
      ],
      source: chineseCharacterSource("汉"),
    },
  ],
  [
    "chinese:语",
    {
      script: "chinese",
      glyph: "语",
      strokes: [
        simpleStroke(
          "draw the speech-radical dot down and right",
          compactPath("125 780 185 735 250 675"),
        ),
        simpleStroke(
          "lift, draw the speech-radical turn, and finish rising up-right without lifting",
          compactPath(
            "50 490 100 490 160 490 220 490 220 430 220 350 220 250 220 130 250 80 345 145",
          ),
        ),
        simpleStroke(
          "lift, then draw 五's top horizontal left to right",
          compactPath("355 775 490 775 625 775 760 775 920 775"),
        ),
        simpleStroke(
          "lift, then draw 五's descending second stroke",
          compactPath("600 740 585 650 555 520 520 400"),
        ),
        simpleStroke(
          "lift, then draw 五's horizontal-turning third stroke",
          compactPath(
            "390 590 500 590 620 590 740 590 820 585 810 500 790 400",
          ),
        ),
        simpleStroke(
          "lift, then draw 五's long bottom horizontal left to right",
          compactPath("320 395 440 395 560 395 700 395 830 395 950 395"),
        ),
        simpleStroke(
          "lift, then draw 口's left vertical",
          compactPath("440 260 440 100 440 -50"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down the right side without lifting",
          compactPath(
            "440 260 520 260 650 260 760 260 850 260 850 100 850 -45",
          ),
        ),
        simpleStroke(
          "lift, then close 口 along the bottom from left to right",
          compactPath("440 -5 540 -5 650 -5 750 -5 850 -5"),
        ),
      ],
      source: chineseCharacterSource("语"),
    },
  ],
  [
    "chinese:文",
    {
      script: "chinese",
      glyph: "文",
      strokes: [
        simpleStroke(
          "draw the top dot down and right",
          compactPath("440 820 470 770 510 700"),
        ),
        simpleStroke(
          "lift, then draw the horizontal stroke left to right",
          compactPath("50 625 230 625 400 625 570 625 750 625 950 625"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath(
            "250 600 275 520 315 440 360 370 415 300 475 235 500 190 430 130 345 80 250 45 150 15 60 -5",
          ),
        ),
        simpleStroke(
          "lift, return near the centre, and draw the long right-falling stroke",
          compactPath("750 600 700 470 625 340 520 205 650 105 790 40 940 -5"),
        ),
      ],
      source: chineseCharacterSource("文"),
    },
  ],
  [
    "chinese:国",
    {
      script: "chinese",
      glyph: "国",
      strokes: [
        simpleStroke(
          "draw the outer left vertical from top to bottom",
          compactPath("125 780 125 620 125 460 125 300 125 140 125 -45"),
        ),
        simpleStroke(
          "lift, then draw the outer top and turn down the right side without lifting",
          compactPath(
            "125 780 250 780 380 780 510 780 640 780 760 780 875 780 875 620 875 460 875 300 875 140 875 -45",
          ),
        ),
        simpleStroke(
          "lift, then draw 玉's top horizontal left to right",
          compactPath("240 610 340 610 445 610 550 610 650 610 755 610"),
        ),
        simpleStroke(
          "lift, then draw 玉's middle horizontal left to right",
          compactPath("270 405 420 405 575 405 730 405"),
        ),
        simpleStroke(
          "lift, then draw 玉's central vertical from top to bottom",
          compactPath("495 600 495 500 495 400 495 290 495 180"),
        ),
        simpleStroke(
          "lift, then draw 玉's bottom horizontal left to right",
          compactPath("230 180 365 180 500 180 640 180 775 180"),
        ),
        simpleStroke(
          "lift, then add 玉's short dot down and right",
          compactPath("625 330 665 290 705 250"),
        ),
        simpleStroke(
          "lift, then close the outer frame along the bottom left to right",
          compactPath("125 10 310 10 500 10 690 10 875 10"),
        ),
      ],
      source: chineseCharacterSource("国"),
    },
  ],
  [
    "chinese:看",
    {
      script: "chinese",
      glyph: "看",
      strokes: [
        simpleStroke(
          "draw the short top left-falling stroke",
          compactPath(
            "830 805 750 790 650 780 525 768 400 760 275 755 135 755",
          ),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal left to right",
          compactPath("140 632 300 632 500 632 700 632 875 632"),
        ),
        simpleStroke(
          "lift, then draw the next horizontal left to right",
          compactPath("70 496 250 496 500 496 750 496 930 496"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke toward the lower component",
          compactPath(
            "455 750 425 660 390 570 350 485 300 400 240 325 175 260 110 215 50 180",
          ),
        ),
        simpleStroke(
          "lift, then draw 目's left vertical from top to bottom",
          compactPath("296 365 296 280 296 190 296 100 296 10 296 -60"),
        ),
        simpleStroke(
          "lift, draw 目's top horizontal, then turn down the right side without lifting",
          compactPath(
            "296 365 420 365 550 365 680 365 805 365 805 275 805 180 805 85 805 -50",
          ),
        ),
        simpleStroke(
          "lift, then draw 目's first inner horizontal left to right",
          compactPath("315 240 430 240 550 240 675 240 790 240"),
        ),
        simpleStroke(
          "lift, then draw 目's second inner horizontal left to right",
          compactPath("315 118 430 118 550 118 675 118 790 118"),
        ),
        simpleStroke(
          "lift, then close 目 with its bottom horizontal left to right",
          compactPath("310 -12 430 -12 550 -12 675 -12 790 -12"),
        ),
      ],
      source: chineseCharacterSource("看"),
    },
  ],
  [
    "chinese:书",
    {
      script: "chinese",
      glyph: "书",
      strokes: [
        simpleStroke(
          "draw the short upper horizontal, then fold down without lifting",
          compactPath(
            "140 630 280 630 440 630 600 630 760 630 760 540 760 450 760 365",
          ),
        ),
        simpleStroke(
          "lift, draw the second horizontal, then fold down and finish with the hook without lifting",
          compactPath(
            "70 360 250 360 470 360 700 360 900 360 905 280 900 200 890 125 870 75 835 55 785 52 720 55 650 60",
          ),
        ),
        simpleStroke(
          "lift, then draw the long central upright from top to bottom",
          compactPath("456 820 456 650 456 475 456 300 456 120 456 -60"),
        ),
        simpleStroke(
          "lift, then add the small upper-right dot",
          compactPath("740 780 790 745 840 710 900 665 925 645"),
        ),
      ],
      source: chineseCharacterSource("书"),
    },
  ],
  // 吗 is 口 + 马, and the pinned medians draw the left component FIRST and
  // whole: medians 1-3 are the standalone 口 route -- left vertical, joined top
  // and right side, closing bottom bar. Medians 4-6 then draw 马: a horizontal
  // that turns down, a long stroke that turns right, turns down again and hooks
  // back to the left below the baseline, and a wide finishing héng. Six strokes,
  // five lifts. The paths below are fitted to the vendored Noto Sans SC outline
  // -- the source's Arphic-derived proportions differ, and in this face the 口
  // is tall rather than compressed and the finishing héng stops short of the
  // right descender instead of crossing it -- with the source's order,
  // directions and two joined corners preserved.
  [
    "chinese:吗",
    {
      script: "chinese",
      glyph: "吗",
      strokes: [
        simpleStroke(
          "draw 口's left vertical shù stroke from top to bottom",
          compactPath("110 730 110 610 110 480 110 350 110 230 110 120"),
        ),
        {
          segments: [
            {
              label: "lift, then draw 口's top bar from left to right",
              path: compactPath("100 705 155 705 210 705 260 705 300 705"),
            },
            {
              label:
                "turn the corner without lifting and descend 口's right side",
              path: compactPath("300 705 300 585 300 465 300 340 300 215"),
            },
          ],
        },
        simpleStroke(
          "lift, then close 口's bottom from left to right",
          compactPath("100 220 155 220 210 220 260 220 300 220"),
        ),
        {
          segments: [
            {
              label: "lift, then draw 马's top horizontal from left to right",
              path: compactPath("440 750 530 750 620 750 715 750 800 750"),
            },
            {
              label: "turn the corner without lifting and descend to the right",
              path: compactPath("800 750 800 665 800 580 800 500 800 430"),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend 马's inner vertical",
              path: compactPath("500 630 500 555 500 480 500 400"),
            },
            {
              label: "turn without lifting and cross to the right",
              path: compactPath("500 400 600 375 700 372 800 370 880 370"),
            },
            {
              label: "turn again without lifting and descend the right side",
              path: compactPath("880 370 880 290 875 210 868 130 858 60 845 5"),
            },
            {
              label:
                "finish with the hook, which turns back below the baseline",
              path: compactPath("845 5 820 -20 770 -30 710 -32"),
            },
          ],
        },
        simpleStroke(
          "lift, then draw 马's wide finishing horizontal from left to right",
          compactPath("420 170 510 170 600 170 690 170 770 170"),
        ),
      ],
      source: chineseCharacterSource("吗"),
    },
  ],
  // The numeral tranche keeps the pinned Hanzi Writer medians as its authored
  // centre lines. These six characters are deliberately not approximated by
  // reusing a superficially similar component: 七 and 九 each contain a joined
  // hook, while 百 closes its box only after the inner horizontal.
  [
    "chinese:六",
    {
      script: "chinese",
      glyph: "六",
      strokes: [
        simpleStroke(
          "draw the short top dot down and to the right",
          compactPath("445 800 470 750 510 690 545 635"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal left to right",
          compactPath("75 536 250 536 500 536 750 536 930 536"),
        ),
        simpleStroke(
          "lift, then draw the left-falling leg",
          compactPath("335 350 300 275 255 190 200 105 125 -30"),
        ),
        simpleStroke(
          "lift, then draw the right-falling leg",
          compactPath("640 350 700 260 770 160 835 65 900 -30"),
        ),
      ],
      source: chineseCharacterSource("六"),
    },
  ],
  [
    "chinese:七",
    {
      script: "chinese",
      glyph: "七",
      strokes: [
        simpleStroke(
          "draw the rising horizontal from left to right",
          compactPath("65 405 250 435 500 475 750 515 935 545"),
        ),
        simpleStroke(
          "lift, descend, bend right along the foot and hook upward",
          compactPath(
            "380 790 380 500 380 150 390 65 450 0 600 -8 760 -8 820 25 850 100 870 170",
          ),
        ),
      ],
      source: chineseCharacterSource("七"),
    },
  ],
  [
    "chinese:八",
    {
      script: "chinese",
      glyph: "八",
      strokes: [
        simpleStroke(
          "draw the left-falling stroke",
          compactPath("345 720 325 560 290 390 240 220 175 80 90 -30"),
        ),
        simpleStroke(
          "lift, then draw the separate right-falling stroke",
          compactPath("625 735 645 585 680 420 730 260 805 115 915 -30"),
        ),
      ],
      source: chineseCharacterSource("八"),
    },
  ],
  [
    "chinese:九",
    {
      script: "chinese",
      glyph: "九",
      strokes: [
        simpleStroke(
          "draw the long left-falling stroke",
          compactPath("390 800 390 620 375 440 335 275 260 125 100 -30"),
        ),
        simpleStroke(
          "lift, cross right, turn downward and hook left at the foot",
          compactPath(
            "95 546 300 546 520 546 690 546 690 380 690 160 690 45 720 0 800 -28 875 -28 920 15 940 80 945 120",
          ),
        ),
      ],
      source: chineseCharacterSource("九"),
    },
  ],
  [
    "chinese:十",
    {
      script: "chinese",
      glyph: "十",
      strokes: [
        simpleStroke(
          "draw the horizontal from left to right",
          compactPath("70 427 300 427 500 427 700 427 935 427"),
        ),
        simpleStroke(
          "lift, then draw the crossing vertical top to bottom",
          compactPath("501 820 501 600 501 350 501 100 501 -60"),
        ),
      ],
      source: chineseCharacterSource("十"),
    },
  ],
  [
    "chinese:百",
    {
      script: "chinese",
      glyph: "百",
      strokes: [
        simpleStroke(
          "draw the top horizontal from left to right",
          compactPath("80 749 300 749 500 749 700 749 920 749"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke",
          compactPath("490 730 480 655 455 575 430 530"),
        ),
        simpleStroke(
          "lift, then draw the box's left vertical",
          compactPath("215 530 215 350 215 150 215 -60"),
        ),
        simpleStroke(
          "lift, then draw the box's top and turn down its right side",
          compactPath(
            "220 528 400 528 600 528 800 528 798 350 798 150 798 -50",
          ),
        ),
        simpleStroke(
          "lift, then draw the inner horizontal",
          compactPath("245 275 420 275 600 275 770 275"),
        ),
        simpleStroke(
          "lift, then close the box with the bottom horizontal",
          compactPath("245 19 420 19 600 19 770 19"),
        ),
      ],
      source: chineseCharacterSource("百"),
    },
  ],
  // Store dense CJK path coordinates as x/y pairs. This keeps the lazy
  // handwriting bundle compact without reducing any authored geometry.
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:的",
    {
      script: "chinese",
      glyph: "的",
      strokes: [
        simpleStroke(
          "draw the short left-falling stroke above 白",
          compactPath("256 812 252 740 220 644 120 632"),
        ),
        simpleStroke(
          "lift, then draw the left side of 白",
          compactPath("120 636 120 -28"),
        ),
        simpleStroke(
          "lift, then draw 白's joined top and right side",
          compactPath("120 620 140 644 400 632 400 40"),
        ),
        simpleStroke(
          "lift, then draw 白's inner horizontal",
          compactPath("120 360 400 360"),
        ),
        simpleStroke(
          "lift, then close 白 with its bottom horizontal",
          compactPath("120 40 400 40"),
        ),
        simpleStroke(
          "lift, then draw 勺's left-falling stroke",
          compactPath("624 800 576 644 524 540"),
        ),
        simpleStroke(
          "lift, then draw 勺's long hooked enclosure",
          compactPath("608 648 892 636 880 40 840 0 640 0"),
        ),
        simpleStroke(
          "lift, then place the final dot inside 勺",
          compactPath("620 380 680 320 720 260"),
        ),
      ],
      source: chineseCharacterSource("的"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:有",
    {
      script: "chinese",
      glyph: "有",
      strokes: [
        simpleStroke(
          "draw the top horizontal from left to right",
          compactPath("100 672 352 668 752 672 900 672"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("416 796 376 672 348 648 264 444 80 300 60 304"),
        ),
        simpleStroke(
          "lift, then draw 月's left side",
          compactPath(
            "316 488 288 476 304 320 312 320 292 296 296 148 292 -40",
          ),
        ),
        simpleStroke(
          "lift, then draw 月's joined top, right side and hook",
          compactPath(
            "348 488 748 488 784 472 772 320 664 320 784 308 772 148 668 148 784 136 764 -32 620 -32 612 -24",
          ),
        ),
        simpleStroke(
          "lift, then draw 月's upper inner horizontal",
          compactPath("352 320 556 320"),
        ),
        simpleStroke(
          "lift, then draw 月's lower inner horizontal",
          compactPath("340 148 556 148"),
        ),
      ],
      source: chineseCharacterSource("有"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:个",
    {
      script: "chinese",
      glyph: "个",
      strokes: [
        simpleStroke(
          "draw the left-falling stroke of 人",
          compactPath("516 788 496 740 204 488 88 436"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke of 人",
          compactPath("544 712 804 488 920 436 960 444"),
        ),
        simpleStroke(
          "lift, then draw the central vertical",
          compactPath("496 504 496 -60"),
        ),
      ],
      source: chineseCharacterSource("个"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:了",
    {
      script: "chinese",
      glyph: "了",
      strokes: [
        simpleStroke(
          "draw the upper horizontal and turn downward",
          compactPath("112 724 632 724 788 712 732 624 536 492"),
        ),
        simpleStroke(
          "lift, then draw the long vertical and finish with a left hook",
          compactPath("516 480 500 160 500 -44 420 -60 300 -48"),
        ),
      ],
      source: chineseCharacterSource("了"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:呢",
    {
      script: "chinese",
      glyph: "呢",
      strokes: [
        simpleStroke("draw 口's left side", compactPath("104 632 104 308")),
        simpleStroke(
          "lift, then draw 口's joined top and right side",
          compactPath("104 596 116 708 292 696 292 440"),
        ),
        simpleStroke(
          "lift, then close 口 with its bottom horizontal",
          compactPath("104 368 104 380 116 200 292 212 292 408"),
        ),
        simpleStroke(
          "lift, then draw 尸's turning top stroke",
          compactPath("532 760 876 748 864 548 756 548"),
        ),
        simpleStroke(
          "lift, then draw 尸's inner horizontal",
          compactPath("512 548 832 548"),
        ),
        simpleStroke(
          "lift, then draw 尸's long left-falling stroke",
          compactPath(
            "448 760 480 760 444 744 456 548 472 548 444 532 420 168 340 -16",
          ),
        ),
        simpleStroke(
          "lift, then draw 匕's left-falling stroke",
          compactPath("872 364 600 224 600 208"),
        ),
        simpleStroke(
          "lift, then draw 匕's hooked vertical",
          compactPath("600 408 604 8 652 -32 884 -24 912 88 896 120"),
        ),
      ],
      source: chineseCharacterSource("呢"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:对",
    {
      script: "chinese",
      glyph: "对",
      strokes: [
        simpleStroke(
          "draw 又's turning left-falling stroke",
          compactPath("136 668 412 656 344 332 320 280 268 172 104 -20 76 -16"),
        ),
        simpleStroke(
          "lift, then draw 又's right-falling stroke",
          compactPath("144 452 320 300 324 304 332 236 416 156 440 120"),
        ),
        simpleStroke(
          "lift, then draw 寸's horizontal",
          compactPath("520 560 920 560"),
        ),
        simpleStroke(
          "lift, then draw 寸's hooked vertical",
          compactPath("800 800 812 560 800 168 784 -28 644 -28 624 -12"),
        ),
        simpleStroke(
          "lift, then place 寸's final dot",
          compactPath("552 376 628 232"),
        ),
      ],
      source: chineseCharacterSource("对"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:岁",
    {
      script: "chinese",
      glyph: "岁",
      strokes: [
        simpleStroke(
          "draw 山's centre vertical",
          compactPath("500 804 500 620"),
        ),
        simpleStroke(
          "lift, then draw 山's joined left side and base",
          compactPath("176 732 188 592 564 592 736 592"),
        ),
        simpleStroke(
          "lift, then draw 山's right vertical",
          compactPath("836 752 824 592 768 592"),
        ),
        simpleStroke(
          "lift, then draw 夕's left-falling stroke",
          compactPath("408 556 360 420 148 296"),
        ),
        simpleStroke(
          "lift, then draw 夕's turning hooked stroke",
          compactPath(
            "392 412 700 412 668 412 780 400 720 264 544 116 500 120 500 124 488 76 164 -40",
          ),
        ),
        simpleStroke(
          "lift, then place 夕's inner dot",
          compactPath(
            "260 360 360 420 760 412 776 376 672 212 532 112 496 128 340 260 320 264 412 216 424 204",
          ),
        ),
      ],
      source: chineseCharacterSource("岁"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:和",
    {
      script: "chinese",
      glyph: "和",
      strokes: [
        simpleStroke(
          "draw 禾's short top left-falling stroke",
          compactPath("476 776 344 764 104 724 100 724"),
        ),
        simpleStroke(
          "lift, then draw 禾's horizontal",
          compactPath("84 508 456 508"),
        ),
        simpleStroke(
          "lift, then draw 禾's central vertical",
          compactPath("284 696 288 508 272 408 292 396 284 4 284 -40"),
        ),
        simpleStroke(
          "lift, then draw 禾's left-falling stroke",
          compactPath("272 480 260 400 200 336 72 104 68 92"),
        ),
        simpleStroke(
          "lift, then draw 禾's right-falling dot",
          compactPath("352 368 444 256"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("564 484 564 124"),
        ),
        simpleStroke(
          "lift, then draw 口's joined top and right side",
          compactPath("564 444 576 708 864 696 864 344 864 280"),
        ),
        simpleStroke(
          "lift, then close 口 with its bottom horizontal",
          compactPath("564 196 564 216 576 80 864 92 864 240"),
        ),
      ],
      source: chineseCharacterSource("和"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:喜",
    {
      script: "chinese",
      glyph: "喜",
      strokes: [
        simpleStroke(
          "draw 士's top horizontal",
          compactPath("84 740 496 736 852 740"),
        ),
        simpleStroke(
          "lift, then draw 士's central vertical",
          compactPath("496 800 508 740 520 740 496 720 496 640"),
        ),
        simpleStroke(
          "lift, then draw 士's lower horizontal",
          compactPath("164 612 848 612"),
        ),
        simpleStroke(
          "lift, then draw the upper 口's left side",
          compactPath("272 512 316 512 232 500 244 380 364 380"),
        ),
        simpleStroke(
          "lift, then draw the upper 口's joined top and right side",
          compactPath("352 512 728 512 688 512"),
        ),
        simpleStroke(
          "lift, then close the upper 口",
          compactPath("384 380 756 380 768 424"),
        ),
        simpleStroke(
          "lift, then draw the left dot below the upper 口",
          compactPath("288 380 324 364 348 248 384 248"),
        ),
        simpleStroke(
          "lift, then draw the right dot below the upper 口",
          compactPath("672 380 700 380 640 248 616 248"),
        ),
        simpleStroke(
          "lift, then draw the wide central horizontal",
          compactPath("76 248 476 248 876 248 916 248"),
        ),
        simpleStroke(
          "lift, then draw the lower 口's left side",
          compactPath("272 132 320 132 216 120 228 -24 376 -24"),
        ),
        simpleStroke(
          "lift, then draw the lower 口's joined top and right side",
          compactPath("344 132 744 132 788 112 776 -24 696 -24"),
        ),
        simpleStroke(
          "lift, then close the lower 口",
          compactPath("392 -24 788 -16"),
        ),
      ],
      source: chineseCharacterSource("喜"),
    },
  ],
  // Hanzi Writer Data's ordered medians are fitted to the vendored Noto
  // Sans SC outline without changing stroke order, direction or pen lifts.
  [
    "chinese:欢",
    {
      script: "chinese",
      glyph: "欢",
      strokes: [
        simpleStroke(
          "draw 又's turning left-falling stroke",
          compactPath(
            "108 536 264 336 308 376 384 676 236 692 220 692 384 680 304 360 272 300 260 248 120 76",
          ),
        ),
        simpleStroke(
          "lift, then draw 又's right-falling stroke",
          compactPath("160 460 272 328 284 340 284 252 360 140"),
        ),
        simpleStroke(
          "lift, then draw 欠's short top left-falling stroke",
          compactPath("576 800 524 580 480 480"),
        ),
        simpleStroke(
          "lift, then draw 欠's turning horizontal hook",
          compactPath("540 628 576 652 896 640 872 536"),
        ),
        simpleStroke(
          "lift, then draw 欠's long left-falling stroke",
          compactPath("664 520 656 292 528 64 400 -32 376 -32"),
        ),
        simpleStroke(
          "lift, then draw 欠's final right-falling stroke",
          compactPath("668 324 828 40 924 -24 956 -16"),
        ),
      ],
      source: chineseCharacterSource("欢"),
    },
  ],
  // 尔 is 你 without its person radical, and Hanzi Writer Data's 尔.json
  // gives the same five ordered medians: a left-falling stroke across the top,
  // a horizontal that hooks down-left without lifting, a central vertical that
  // hooks left at its foot, then the lower-left and lower-right dots. Each run
  // is fitted to the standalone Noto Sans SC 尔, whose proportions are wider
  // than the right half of 你, keeping every direction and the four lifts.
  [
    "chinese:尔",
    {
      script: "chinese",
      glyph: "尔",
      strokes: [
        simpleStroke(
          "draw the upper left-falling stroke",
          compactPath(
            "335 815 322 770 305 722 285 675 262 628 234 580 203 533 168 488 130 450 90 420",
          ),
        ),
        {
          segments: [
            {
              label: "lift, then draw the upper horizontal from left to right",
              path: compactPath(
                "300 620 400 620 500 620 600 620 700 620 800 620 880 622",
              ),
            },
            {
              label: "hook down and left without lifting",
              path: compactPath(
                "880 622 905 615 895 570 870 525 840 480 805 432",
              ),
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then descend the central vertical",
              path: compactPath(
                "506 610 506 500 506 400 506 300 506 200 506 100 506 20",
              ),
            },
            {
              label: "hook left at the base without lifting",
              path: compactPath(
                "506 20 497 -20 470 -36 430 -40 390 -40 350 -40 305 -42",
              ),
            },
          ],
        },
        simpleStroke(
          "lift, then draw the lower-left dot down and left",
          compactPath(
            "308 395 290 345 262 290 228 235 188 182 145 135 100 100",
          ),
        ),
        simpleStroke(
          "lift, then draw the lower-right dot down and right",
          compactPath("705 385 740 335 775 285 810 235 845 180 880 125 905 85"),
        ),
      ],
      source: chineseCharacterSource("尔"),
    },
  ],
  // 很 is two pieces, and Hanzi Writer Data's nine ordered medians keep them
  // apart: medians 1-3 write 彳 (two left-falling strokes, then a vertical
  // that restarts under the lower one), medians 4-9 write 艮 (a horizontal
  // that turns down the right side of the box, two horizontals, the left side
  // as a vertical that flicks up to the right at its foot, a short
  // left-falling stroke, then the long right-falling stroke). Each run is
  // fitted to the vendored Noto Sans SC 很, whose box is closed by one
  // outline and whose flick is a separate outline joined to the left side;
  // the order, directions, the turn, the flick and all eight lifts are the
  // source's.
  [
    "chinese:很",
    {
      script: "chinese",
      glyph: "很",
      strokes: [
        simpleStroke(
          "draw 彳's short upper left-falling stroke",
          compactPath("272 812 257 780 240 760 204 720 162 680 115 640 98 605"),
        ),
        simpleStroke(
          "lift, then draw 彳's longer lower left-falling stroke",
          compactPath(
            "290 588 265 540 236 500 203 460 168 420 128 380 104 350 86 318 76 292",
          ),
        ),
        simpleStroke(
          "lift, then draw 彳's vertical down to the baseline",
          compactPath("218 450 218 400 218 250 218 100 218 -70"),
        ),
        simpleStroke(
          "lift, then draw 艮's top and turn down its right side",
          compactPath("450 762 600 762 750 762 834 762 834 650 834 520 834 400"),
        ),
        simpleStroke(
          "lift, then draw 艮's short inner horizontal",
          compactPath("450 577 640 577 812 577"),
        ),
        simpleStroke(
          "lift, then close 艮's box with a horizontal",
          compactPath("450 388 640 388 826 388"),
        ),
        simpleStroke(
          "lift, then draw 艮's left side and flick up to the right",
          compactPath(
            "436 788 436 600 436 400 436 200 436 40 430 -15 460 -17 520 0 580 15 645 35",
          ),
        ),
        simpleStroke(
          "lift, then draw 艮's short left-falling stroke",
          compactPath("872 305 853 262 825 240 797 220 768 200 743 180 728 160"),
        ),
        simpleStroke(
          "lift, then draw 艮's long right-falling stroke",
          compactPath(
            "590 395 604 360 617 320 632 280 649 240 670 200 693 160 723 120 758 80 803 40 865 0 890 -40 902 -58",
          ),
        ),
      ],
      source: chineseCharacterSource("很"),
    },
  ],
  // 他: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 他. Medians 1–2 draw 亻 on the left: a left-falling stroke, then a
  // vertical restarting under it, as 你 opens. Medians 3–5 draw 也 on the right:
  // a horizontal that rises to the right, turns down and hooks to the left
  // without lifting, a vertical through the middle, then a long vertical that
  // bends right along the foot and hooks up without lifting. Noto's 也 is wider
  // than the source's, so its turn and hook sit further right. The order, the
  // directions, the two turns, the two hooks and all four lifts are the
  // source's.
  [
    "chinese:他",
    {
      script: "chinese",
      glyph: "他",
      strokes: [
        simpleStroke(
          "draw 亻's left-falling stroke",
          compactPath("284 812 289 773 167 534 55 400"),
        ),
        simpleStroke(
          "lift, then draw 亻's vertical down to the baseline",
          compactPath("170 610 209 400 193 31 202 -68"),
        ),
        simpleStroke(
          "lift, then draw 也's rising horizontal, turn down the right side and hook left",
          compactPath("297 401 870 615 880 560 876 280 850 232 752 221"),
        ),
        simpleStroke(
          "lift, then draw 也's middle vertical",
          compactPath("630 820 660 741 642 157"),
        ),
        simpleStroke(
          "lift, then draw 也's long vertical, bend right along the foot and hook up",
          compactPath(
            "426 710 434 100 445 10 480 -28 827 -28 880 -15 892 5 911 52 912 117",
          ),
        ),
      ],
      source: chineseCharacterSource("他"),
    },
  ],
  // 们: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 们. Medians 1–2 draw 亻 on the left: a left-falling stroke, then a
  // vertical restarting under it. Medians 3–5 draw 门 on the right: a dot, the
  // left vertical, then a horizontal that turns down the right side and hooks
  // to the left at the foot without lifting. The order, the directions, the
  // turn, the hook and all four lifts are the source's.
  [
    "chinese:们",
    {
      script: "chinese",
      glyph: "们",
      strokes: [
        simpleStroke(
          "draw 亻's left-falling stroke",
          compactPath("245 813 259 780 182 587 174 534 71 394"),
        ),
        simpleStroke(
          "lift, then draw 亻's vertical down to the baseline",
          compactPath("138 566 194 396 178 19 187 -66"),
        ),
        simpleStroke(
          "lift, then draw 门's dot",
          compactPath("411 788 488 710 526 647"),
        ),
        simpleStroke(
          "lift, then draw 门's left vertical",
          compactPath("365 622 380 574 368 -68"),
        ),
        simpleStroke(
          "lift, then draw 门's top, turn down the right side and hook left",
          compactPath(
            "588 758 706 746 813 772 835 767 879 660 878 12 863 -23 845 -34 788 -43 721 -27",
          ),
        ),
      ],
      source: chineseCharacterSource("们"),
    },
  ],
  // 妈: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 妈. Medians 1–3 draw 女 on the left: a stroke that falls to the left
  // and turns down to the right without lifting, a left-falling stroke, then a
  // horizontal that rises as it crosses. Medians 4–6 draw 马 on the right, as 吗
  // closes: a horizontal that turns down, a long stroke that turns twice and
  // hooks, then a rising horizontal. Noto's narrow 女 crosses its horizontal
  // higher than the source's. The order, the directions, the four turns, the
  // hook and all five lifts are the source's.
  [
    "chinese:妈",
    {
      script: "chinese",
      glyph: "妈",
      strokes: [
        simpleStroke(
          "draw 女's first stroke down-left, then turn down to the right",
          compactPath("210 823 183 642 108 322 112 300 152 257 260 160 347 40"),
        ),
        simpleStroke(
          "lift, then draw 女's left-falling stroke",
          compactPath("332 623 340 507 318 353 258 148 143 14 71 -36"),
        ),
        simpleStroke(
          "lift, then draw 女's rising horizontal",
          compactPath("65 595 362 606"),
        ),
        simpleStroke(
          "lift, then draw 马's top and turn down",
          compactPath(
            "432 749 550 740 746 758 783 749 811 717 785 458 755 351",
          ),
        ),
        simpleStroke(
          "lift, then draw 马's long stroke down, right, down and hook left",
          compactPath(
            "488 633 499 619 483 394 504 375 550 364 837 374 878 366 902 336 856 75 822 -21 793 -46 676 -34",
          ),
        ),
        simpleStroke(
          "lift, then draw 马's rising horizontal",
          compactPath("395 184 495 165 772 173"),
        ),
      ],
      source: chineseCharacterSource("妈"),
    },
  ],
  // 男: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 男. Medians 1–5 draw 田 on top: its left side, a horizontal that
  // turns down the right side without lifting, the middle horizontal, the
  // middle vertical, then the horizontal that closes the box. Medians 6–7 draw
  // 力 below: a horizontal that turns down and hooks to the left without
  // lifting, then a long left-falling stroke through it. The order, the
  // directions, the two turns, the hook and all six lifts are the source's.
  [
    "chinese:男",
    {
      script: "chinese",
      glyph: "男",
      strokes: [
        simpleStroke(
          "draw 田's left side",
          compactPath("190 774 194 396"),
        ),
        simpleStroke(
          "lift, then draw 田's top and turn down its right side",
          compactPath(
            "166 768 313 744 426 757 795 750 828 708 788 587 800 498 774 398",
          ),
        ),
        simpleStroke(
          "lift, then draw 田's middle horizontal",
          compactPath("253 571 529 589 808 584"),
        ),
        simpleStroke(
          "lift, then draw 田's middle vertical",
          compactPath("462 774 507 669 505 518 470 454"),
        ),
        simpleStroke(
          "lift, then close 田 with a horizontal",
          compactPath("236 413 498 422 603 412 830 426"),
        ),
        simpleStroke(
          "lift, then draw 力's horizontal, turn down and hook left",
          compactPath(
            "82 270 242 244 479 261 558 248 790 262 846 249 801 47 757 -31 681 -54 598 -37 571 -20",
          ),
        ),
        simpleStroke(
          "lift, then draw 力's long left-falling stroke",
          compactPath("440 435 495 369 453 230 380 120 255 20 78 -44"),
        ),
      ],
      source: chineseCharacterSource("男"),
    },
  ],
  // 叫: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 叫. Medians 1–3 draw 口 on the left in the order the standalone
  // character uses: its left side, its joined top and right side, and its
  // closing bottom. Medians 4–5 draw the right side: a vertical that flicks up
  // to the right at its foot without lifting, then a long vertical from the top
  // to the baseline. The order, the directions, the turn, the flick and all
  // four lifts are the source's.
  [
    "chinese:叫",
    {
      script: "chinese",
      glyph: "叫",
      strokes: [
        simpleStroke(
          "draw 口's left side",
          compactPath("130 735 130 94"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("106 708 375 700 375 186"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("106 210 395 210"),
        ),
        simpleStroke(
          "lift, then draw the short vertical and flick up to the right",
          compactPath("561 733 558 230 525 140 790 232"),
        ),
        simpleStroke(
          "lift, then draw the long vertical down to the baseline",
          compactPath("832 816 854 731 842 -67"),
        ),
      ],
      source: chineseCharacterSource("叫"),
    },
  ],
  // 来: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 来. Median 1 draws the short top horizontal; medians 2–3 draw the
  // two short strokes under it, a dot on the left falling to the right and a
  // short stroke on the right falling to the left; median 4 draws the long
  // horizontal; median 5 draws the vertical from the top through both
  // horizontals; medians 6–7 draw the left-falling and right-falling pair that
  // spreads from the centre. The order, the directions and all six lifts are
  // the source's.
  [
    "chinese:来",
    {
      script: "chinese",
      glyph: "来",
      strokes: [
        simpleStroke(
          "draw the short top horizontal",
          compactPath("117 705 271 679 447 682 480 664 614 688 830 688"),
        ),
        simpleStroke(
          "lift, then draw the dot on the left",
          compactPath("206 595 275 525 317 440"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke on the right",
          compactPath("802 599 686 432"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("72 367 158 350 362 367 589 360 741 371 890 336"),
        ),
        simpleStroke(
          "lift, then draw the vertical down through both horizontals",
          compactPath("480 826 493 750 489 -65"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke",
          compactPath("525 417 361 216 239 106 68 8"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("470 415 645 201 752 105 933 7"),
        ),
      ],
      source: chineseCharacterSource("来"),
    },
  ],
  // 这: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 这. in two pieces, the walking piece written last. Medians 1–4 draw
  // 文 on the upper right as 文 is written alone: a dot, a horizontal, a left-
  // falling stroke, then a shortened right-falling stroke across it. Medians
  // 5–7 draw 辶 under it: a dot, a stroke that runs right, turns down, turns
  // again and falls to the left without lifting, then the long flat right-
  // falling stroke along the foot. The order, the directions, the turns and all
  // six lifts are the source's.
  [
    "chinese:这",
    {
      script: "chinese",
      glyph: "这",
      strokes: [
        simpleStroke(
          "draw 文's dot",
          compactPath("551 814 648 710"),
        ),
        simpleStroke(
          "lift, then draw 文's horizontal",
          compactPath("306 645 496 636 792 655 930 641"),
        ),
        simpleStroke(
          "lift, then draw 文's left-falling stroke",
          compactPath("786 655 697 425 625 324 510 214 321 136"),
        ),
        simpleStroke(
          "lift, then draw 文's short right-falling stroke",
          compactPath("365 514 577 367 810 183 857 117"),
        ),
        simpleStroke(
          "lift, then draw 辶's dot",
          compactPath("94 763 238 640"),
        ),
        simpleStroke(
          "lift, then draw 辶's short horizontal, turn down, turn again and fall left",
          compactPath("63 427 220 432 223 391 202 311 227 171 209 127 199 44"),
        ),
        simpleStroke(
          "lift, then draw 辶's long sweep along the foot",
          compactPath("64 11 224 72 458 -12 800 -30 935 6"),
        ),
      ],
      source: chineseCharacterSource("这"),
    },
  ],
  // 那: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 那. Medians 1–4 draw the left piece: a horizontal that turns down
  // the right side and hooks to the left at the foot without lifting, two
  // horizontals, then a left-falling stroke from the top through both. Medians
  // 5–6 draw 阝 on the right: a stroke that runs right, falls back to the left,
  // swells round to the right and hooks to the left without lifting, then the
  // vertical. Noto's left piece reaches lower than the source's, so its hook
  // sits at the foot. The order, the directions, the turns, the two hooks and
  // all five lifts are the source's.
  [
    "chinese:那",
    {
      script: "chinese",
      glyph: "那",
      strokes: [
        simpleStroke(
          "draw the top, turn down the right side and hook left",
          compactPath("66 757 462 757 455 40 430 -15 291 -26"),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal",
          compactPath("79 517 474 517"),
        ),
        simpleStroke(
          "lift, then draw the lower horizontal",
          compactPath("73 285 474 285"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke through both",
          compactPath("249 774 238 400 210 240 90 -52"),
        ),
        simpleStroke(
          "lift, then draw 阝's ear across, back, round and hook left",
          compactPath(
            "599 757 900 752 770 460 868 330 908 230 898 150 868 115 714 105",
          ),
        ),
        simpleStroke(
          "lift, then draw 阝's vertical",
          compactPath("602 772 626 707 625 -65"),
        ),
      ],
      source: chineseCharacterSource("那"),
    },
  ],
  // 吃: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 吃. Medians 1–3 draw 口 on the left in the order the standalone
  // character uses. Medians 4–6 draw 乞 on the right: a short left-falling
  // stroke, a horizontal, then a horizontal that turns down to the left, runs
  // along the foot and hooks up without lifting. Noto's last stroke turns
  // further right than the source's. The order, the directions, the turns, the
  // hook and all five lifts are the source's.
  [
    "chinese:吃",
    {
      script: "chinese",
      glyph: "吃",
      strokes: [
        simpleStroke(
          "draw 口's left side",
          compactPath("112 735 112 102"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("88 705 307 705 307 178"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("88 195 326 195"),
        ),
        simpleStroke(
          "lift, then draw 乞's short left-falling stroke",
          compactPath("546 822 565 785 522 661 466 575 395 498"),
        ),
        simpleStroke(
          "lift, then draw 乞's horizontal",
          compactPath("462 634 792 659 930 643"),
        ),
        simpleStroke(
          "lift, then draw 乞's last stroke across, down to the left, along the foot and hook up",
          compactPath(
            "480 440 840 440 836 449 672 302 468 104 456 28 484 0 520 -20 700 -30 880 -20 915 40 921 134",
          ),
        ),
      ],
      source: chineseCharacterSource("吃"),
    },
  ],
  // 饭: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 饭. Medians 1–3 draw the food piece 饣 on the left: a left-falling
  // stroke, a short horizontal that hooks down to the left without lifting,
  // then a vertical that flicks up to the right at its foot without lifting.
  // Medians 4–7 draw 反 on the right: a flat left-falling stroke across the top,
  // a long left-falling stroke down the left side, then 又 as 对 draws it, a
  // horizontal that turns down to the left without lifting and the right-
  // falling stroke. The order, the directions, the turns, the hook, the flick
  // and all six lifts are the source's.
  [
    "chinese:饭",
    {
      script: "chinese",
      glyph: "饭",
      strokes: [
        simpleStroke(
          "draw 饣's left-falling stroke",
          compactPath("191 815 165 687 123 567 54 445"),
        ),
        simpleStroke(
          "lift, then draw 饣's short horizontal and hook down",
          compactPath("121 632 320 655 369 646 332 543 297 486"),
        ),
        simpleStroke(
          "lift, then draw 饣's vertical and flick up to the right",
          compactPath("184 473 207 346 196 42 232 52 338 133"),
        ),
        simpleStroke(
          "lift, then draw 反's flat top stroke from right to left",
          compactPath("857 748 704 746 513 700"),
        ),
        simpleStroke(
          "lift, then draw 反's long left-falling stroke",
          compactPath("455 639 485 593 489 564 485 401 453 162 410 35 343 -34"),
        ),
        simpleStroke(
          "lift, then draw 又's horizontal and turn down to the left",
          compactPath(
            "555 468 744 510 830 474 831 386 753 210 719 183 678 102 592 18 470 -25",
          ),
        ),
        simpleStroke(
          "lift, then draw 又's right-falling stroke",
          compactPath("590 350 720 119 826 4 863 -22 933 -36"),
        ),
      ],
      source: chineseCharacterSource("饭"),
    },
  ],
  // 喝: Hanzi Writer Data's twelve ordered medians, fitted to the vendored Noto
  // Sans SC 喝. Medians 1–3 draw 口 on the left. Medians 4–12 draw 曷 on the
  // right: 日 first, as 日 is written alone (its left side, the joined top and
  // right side, the middle horizontal, the closing bottom), then a short left-
  // falling stroke, a horizontal that turns down the right side and hooks to
  // the left without lifting, a left-falling stroke and a dot inside it, and
  // last a vertical that turns along the foot without lifting. The order, the
  // directions, the turns, the hook and all eleven lifts are the source's.
  [
    "chinese:喝",
    {
      script: "chinese",
      glyph: "喝",
      strokes: [
        simpleStroke(
          "draw 口's left side",
          compactPath("107 728 107 100"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("87 705 293 705 293 196"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("87 225 310 225"),
        ),
        simpleStroke(
          "lift, then draw 日's left side",
          compactPath("453 785 453 478"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("433 765 860 765 860 476"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("430 642 574 629 704 639 866 632"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("433 495 880 495"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke under 日",
          compactPath("495 510 485 420 374 238"),
        ),
        simpleStroke(
          "lift, then draw the frame's top, turn down and hook left",
          compactPath("449 359 905 367 895 60 870 -20 830 -55 750 -40"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke inside",
          compactPath("678 298 652 237 605 177 542 129"),
        ),
        simpleStroke(
          "lift, then draw the dot beside it",
          compactPath("632 232 712 199 776 133"),
        ),
        simpleStroke(
          "lift, then draw the last stroke down and along the foot",
          compactPath("446 231 446 67 802 67"),
        ),
      ],
      source: chineseCharacterSource("喝"),
    },
  ],
  // 水: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 水. Median 1 draws the vertical down the middle and hooks it to the
  // left without lifting; median 2 draws the left side, a horizontal that turns
  // to fall to the lower left without lifting; medians 3–4 draw the right side,
  // a short left-falling stroke, then the right-falling stroke from the
  // vertical. The order, the directions, the turn, the hook and all three lifts
  // are the source's.
  [
    "chinese:水",
    {
      script: "chinese",
      glyph: "水",
      strokes: [
        simpleStroke(
          "draw the middle vertical and hook left",
          compactPath("478 823 503 744 490 6 468 -47 344 -25"),
        ),
        simpleStroke(
          "lift, then draw the left side across and down to the left",
          compactPath(
            "82 549 167 535 308 536 345 461 307 344 236 207 129 79 88 52",
          ),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke on the right",
          compactPath("778 593 790 568 606 352"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("483 625 556 461 671 284 758 180 812 136 845 131 943 76"),
        ),
      ],
      source: chineseCharacterSource("水"),
    },
  ],
  // 买: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 买. Median 1 draws the top, a horizontal that hooks down to the left
  // without lifting; medians 2–6 draw 头 under it: two dots falling to the
  // right, the long horizontal, a left-falling stroke from above the horizontal
  // through it, then the dot at the lower right. The order, the directions, the
  // hook and all five lifts are the source's.
  [
    "chinese:买",
    {
      script: "chinese",
      glyph: "买",
      strokes: [
        simpleStroke(
          "draw the top horizontal and hook down to the left",
          compactPath(
            "122 749 197 733 599 732 705 742 777 733 848 688 781 558",
          ),
        ),
        simpleStroke(
          "lift, then draw the upper dot",
          compactPath("258 622 422 514"),
        ),
        simpleStroke(
          "lift, then draw the lower dot",
          compactPath("148 473 267 410 306 372"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("79 275 163 261 233 271 686 266 803 277 922 251"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke through it",
          compactPath(
            "542 625 553 580 521 315 469 167 415 97 327 38 244 -2 90 -48",
          ),
        ),
        simpleStroke(
          "lift, then draw the last dot",
          compactPath("576 155 581 137 765 47 887 -49"),
        ),
      ],
      source: chineseCharacterSource("买"),
    },
  ],
  // 多: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 多. in two pieces, the same three strokes twice. Medians 1–3 draw
  // the upper 夕: a short left-falling stroke, a horizontal that turns to fall
  // to the lower left without lifting, then a dot inside. Medians 4–6 draw the
  // lower 夕 in the same order, larger. The order, the directions, the two turns
  // and all five lifts are the source's.
  [
    "chinese:多",
    {
      script: "chinese",
      glyph: "多",
      strokes: [
        simpleStroke(
          "draw the upper 夕's short left-falling stroke",
          compactPath("470 820 486 804 449 748 261 624 139 587"),
        ),
        simpleStroke(
          "lift, then draw its horizontal and turn down to the left",
          compactPath(
            "379 708 731 722 759 714 688 631 516 496 347 417 120 345",
          ),
        ),
        simpleStroke(
          "lift, then draw its dot",
          compactPath("332 581 436 523 488 460"),
        ),
        simpleStroke(
          "lift, then draw the lower 夕's left-falling stroke",
          compactPath("669 465 427 277 235 190"),
        ),
        simpleStroke(
          "lift, then draw its horizontal and sweep down to the left",
          compactPath(
            "485 340 714 371 889 375 906 358 816 252 747 188 652 125 462 35 266 -26 113 -50",
          ),
        ),
        simpleStroke(
          "lift, then draw its dot",
          compactPath("463 206 549 171 636 80"),
        ),
      ],
      source: chineseCharacterSource("多"),
    },
  ],
  // 少: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 少. Medians 1–3 draw 小 on top, its vertical without a hook: the
  // vertical, a short stroke on the left falling to the lower left, then a dot
  // on the right falling to the lower right. Median 4 draws the long left-
  // falling stroke that sweeps under it. The order, the directions and all
  // three lifts are the source's.
  [
    "chinese:少",
    {
      script: "chinese",
      glyph: "少",
      strokes: [
        simpleStroke(
          "draw the middle vertical",
          compactPath("466 824 497 755 481 481 491 238"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke",
          compactPath("237 662 196 514 104 354"),
        ),
        simpleStroke(
          "lift, then draw the dot on the right",
          compactPath("731 651 850 502 917 371"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("783 298 667 169 546 94 449 49 73 -49"),
        ),
      ],
      source: chineseCharacterSource("少"),
    },
  ],
  // 去: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 去. Medians 1–3 draw 土 on top: the upper horizontal, the vertical
  // through it, then the long horizontal it stands on. Medians 4–5 draw 厶 under
  // it: a stroke that falls to the lower left and turns right along the foot
  // without lifting, then a dot. The order, the directions, the turn and all
  // four lifts are the source's.
  [
    "chinese:去",
    {
      script: "chinese",
      glyph: "去",
      strokes: [
        simpleStroke(
          "draw the upper horizontal",
          compactPath("178 661 497 625 624 650 829 647"),
        ),
        simpleStroke(
          "lift, then draw the vertical through it",
          compactPath("472 824 506 757 496 387 478 356"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("67 387 148 372 331 379 406 365 829 391 936 362"),
        ),
        simpleStroke(
          "lift, then draw 厶's stroke down to the left and turn along the foot",
          compactPath("440 406 316 197 190 40 175 0 260 20 740 50"),
        ),
        simpleStroke(
          "lift, then draw 厶's dot",
          compactPath("647 259 774 92 803 14"),
        ),
      ],
      source: chineseCharacterSource("去"),
    },
  ],
  // 车: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 车. Median 1 draws the top horizontal; median 2 draws a stroke that
  // falls to the lower left and turns right across the middle without lifting;
  // median 3 draws the long bottom horizontal; median 4 draws the vertical
  // last, from the middle down through it. The order, the directions, the turn
  // and all three lifts are the source's.
  [
    "chinese:车",
    {
      script: "chinese",
      glyph: "车",
      strokes: [
        simpleStroke(
          "draw the top horizontal",
          compactPath("129 680 337 641 462 654 568 648 673 666 873 653"),
        ),
        simpleStroke(
          "lift, then draw the stroke down to the left and turn right across the middle",
          compactPath("404 822 280 500 180 370 844 375"),
        ),
        simpleStroke(
          "lift, then draw the long bottom horizontal",
          compactPath(
            "76 163 171 136 286 153 363 148 556 165 634 148 789 161 865 155 927 128",
          ),
        ),
        simpleStroke(
          "lift, then draw the vertical down through it",
          compactPath("526 547 538 489 531 41 552 -65"),
        ),
      ],
      source: chineseCharacterSource("车"),
    },
  ],
  // 天: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 天. Median 1 draws the upper horizontal; medians 2–4 draw 大 under it
  // as 大 is written alone: the long horizontal, the left-falling stroke from
  // under the upper horizontal through the long one, then the right-falling
  // stroke. The order, the directions and all three lifts are the source's.
  [
    "chinese:天",
    {
      script: "chinese",
      glyph: "天",
      strokes: [
        simpleStroke(
          "draw the upper horizontal",
          compactPath("145 742 308 713 438 720 475 704 672 733 840 730"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("79 424 157 409 411 408 722 434 913 392"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke through the long horizontal",
          compactPath("493 690 482 453 446 312 395 205 308 87 76 -34"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("462 466 514 342 702 81 810 0 932 -31"),
        ),
      ],
      source: chineseCharacterSource("天"),
    },
  ],
  // 今: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 今. Medians 1–2 draw the roof as 人 is written alone: the left-
  // falling stroke, then the right-falling stroke from a little under the top;
  // median 3 draws the dot under it; median 4 draws a horizontal that turns to
  // fall to the lower left without lifting. Noto's last stroke runs further
  // right before it turns than the source's. The order, the directions, the
  // turn and all three lifts are the source's.
  [
    "chinese:今",
    {
      script: "chinese",
      glyph: "今",
      strokes: [
        simpleStroke(
          "draw the roof's left-falling stroke",
          compactPath("507 821 447 732 328 620 138 480 71 458"),
        ),
        simpleStroke(
          "lift, then draw the roof's right-falling stroke",
          compactPath("494 819 565 707 776 504 931 469"),
        ),
        simpleStroke(
          "lift, then draw the dot under the roof",
          compactPath("432 550 531 465 583 395"),
        ),
        simpleStroke(
          "lift, then draw the horizontal and turn down to the left",
          compactPath("174 315 812 315 520 -58"),
        ),
      ],
      source: chineseCharacterSource("今"),
    },
  ],
  // 星: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 星. Medians 1–4 draw 日 on top as 日 is written alone: its left side,
  // the joined top and right side, the middle horizontal, the closing bottom.
  // Medians 5–9 draw 生 under it as 生 is written alone: a left-falling stroke,
  // two horizontals, the vertical through them, then the long horizontal at the
  // foot. The order, the directions, the turn and all eight lifts are the
  // source's.
  [
    "chinese:星",
    {
      script: "chinese",
      glyph: "星",
      strokes: [
        simpleStroke(
          "draw 日's left side",
          compactPath("205 787 205 458"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("185 770 796 770 796 458"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("185 622 820 622"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("185 473 820 473"),
        ),
        simpleStroke(
          "lift, then draw 生's left-falling stroke",
          compactPath("242 426 263 398 241 356 186 280 89 200"),
        ),
        simpleStroke(
          "lift, then draw 生's upper horizontal",
          compactPath("159 292 493 314 825 303"),
        ),
        simpleStroke(
          "lift, then draw 生's middle horizontal",
          compactPath("197 155 358 142 506 158 784 135"),
        ),
        simpleStroke(
          "lift, then draw 生's vertical",
          compactPath("487 405 506 384 493 384 497 78 472 -44"),
        ),
        simpleStroke(
          "lift, then draw 生's long bottom horizontal",
          compactPath("80 -13 163 -36 236 -20 497 -10 867 -17 923 -34"),
        ),
      ],
      source: chineseCharacterSource("星"),
    },
  ],
  // 期: Hanzi Writer Data's twelve ordered medians, fitted to the vendored Noto
  // Sans SC 期. Medians 1–8 draw 其 on the left: the top horizontal, the left and
  // right verticals, the two horizontals between them, the long horizontal
  // across their foot, then a left-falling stroke and a dot under it. Medians
  // 9–12 draw 月 on the right: a vertical that curves to the lower left, a
  // horizontal that turns down the right side and hooks to the left without
  // lifting, then two horizontals inside. The order, the directions, the turn,
  // the hook and all eleven lifts are the source's.
  [
    "chinese:期",
    {
      script: "chinese",
      glyph: "期",
      strokes: [
        simpleStroke(
          "draw 其's top horizontal",
          compactPath("63 675 516 675"),
        ),
        simpleStroke(
          "lift, then draw 其's left vertical",
          compactPath("171 813 171 178"),
        ),
        simpleStroke(
          "lift, then draw 其's right vertical",
          compactPath("422 813 422 178"),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal between them",
          compactPath("171 525 422 525"),
        ),
        simpleStroke(
          "lift, then draw the lower horizontal between them",
          compactPath("171 367 422 367"),
        ),
        simpleStroke(
          "lift, then draw 其's long bottom horizontal",
          compactPath("52 195 516 195"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke under it",
          compactPath("214 119 77 -43"),
        ),
        simpleStroke(
          "lift, then draw the dot under it",
          compactPath("361 119 444 -12"),
        ),
        simpleStroke(
          "lift, then draw 月's left side, curving to the lower left",
          compactPath("614 775 605 240 580 100 533 -53"),
        ),
        simpleStroke(
          "lift, then draw 月's top, turn down its right side and hook left",
          compactPath("590 757 891 757 891 100 880 -20 820 -40 744 -40"),
        ),
        simpleStroke(
          "lift, then draw 月's upper horizontal",
          compactPath("614 525 891 525"),
        ),
        simpleStroke(
          "lift, then draw 月's lower horizontal",
          compactPath("614 293 891 293"),
        ),
      ],
      source: chineseCharacterSource("期"),
    },
  ],
  // 年: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 年. Median 1 draws a short left-falling stroke; medians 2–3 draw the
  // upper and middle horizontals; median 4 draws the short vertical on the left
  // down from the middle horizontal; median 5 draws the long horizontal; median
  // 6 draws the long vertical last, from above the top horizontal through all
  // three. The order, the directions and all five lifts are the source's.
  [
    "chinese:年",
    {
      script: "chinese",
      glyph: "年",
      strokes: [
        simpleStroke(
          "draw the short left-falling stroke",
          compactPath("296 822 308 787 270 692 198 593 95 523"),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal",
          compactPath("245 680 471 673 647 690 867 682"),
        ),
        simpleStroke(
          "lift, then draw the middle horizontal",
          compactPath("226 458 866 458"),
        ),
        simpleStroke(
          "lift, then draw the short vertical on the left",
          compactPath("250 458 250 215"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("60 188 940 188"),
        ),
        simpleStroke(
          "lift, then draw the long vertical through all three",
          compactPath("489 706 511 668 534 655 548 609 534 -66"),
        ),
      ],
      source: chineseCharacterSource("年"),
    },
  ],
  // 明: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 明. Medians 1–4 draw 日 on the left as 日 is written alone. Medians
  // 5–8 draw 月 on the right: a vertical that curves to the lower left, a
  // horizontal that turns down the right side and hooks to the left without
  // lifting, then two horizontals inside. The order, the directions, the two
  // turns, the hook and all seven lifts are the source's.
  [
    "chinese:明",
    {
      script: "chinese",
      glyph: "明",
      strokes: [
        simpleStroke(
          "draw 日's left side",
          compactPath("90 764 119 665 123 511 110 321 122 100"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("90 744 184 729 334 736 364 714 371 194"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("115 488 372 488"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("115 218 372 218"),
        ),
        simpleStroke(
          "lift, then draw 月's left side, curving to the lower left",
          compactPath(
            "524 783 546 733 551 643 535 362 544 285 476 95 409 11 346 -45",
          ),
        ),
        simpleStroke(
          "lift, then draw 月's top, turn down its right side and hook left",
          compactPath(
            "615 740 666 749 673 762 829 767 866 752 882 729 888 41 860 -28 758 -36 710 -25",
          ),
        ),
        simpleStroke(
          "lift, then draw 月's upper horizontal",
          compactPath("537 519 890 519"),
        ),
        simpleStroke(
          "lift, then draw 月's lower horizontal",
          compactPath("530 278 890 278"),
        ),
      ],
      source: chineseCharacterSource("明"),
    },
  ],
  // 点: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 点. Medians 1–5 draw 占 on top: a vertical, a short horizontal to its
  // right, then 口 in the order the standalone character uses. Medians 6–9 draw
  // the four dots along the foot from left to right, the first falling to the
  // lower left and the other three to the lower right. The order, the
  // directions, the turn and all eight lifts are the source's.
  [
    "chinese:点",
    {
      script: "chinese",
      glyph: "点",
      strokes: [
        simpleStroke(
          "draw the vertical at the top",
          compactPath("492 827 492 520"),
        ),
        simpleStroke(
          "lift, then draw the short horizontal to its right",
          compactPath("520 697 896 697"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("201 518 201 228"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("201 495 797 495 797 255"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("201 255 817 255"),
        ),
        simpleStroke(
          "lift, then draw the first dot, falling to the left",
          compactPath("215 129 94 -55"),
        ),
        simpleStroke(
          "lift, then draw the second dot",
          compactPath("356 114 384 67 417 -50"),
        ),
        simpleStroke(
          "lift, then draw the third dot",
          compactPath("569 119 664 -44"),
        ),
        simpleStroke(
          "lift, then draw the fourth dot",
          compactPath("782 130 863 50 908 -41"),
        ),
      ],
      source: chineseCharacterSource("点"),
    },
  ],
  // 午: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 午. Median 1 draws a short left-falling stroke; medians 2–3 draw the
  // upper and the long horizontal; median 4 draws the vertical last, from above
  // the upper horizontal through both. The order, the directions and all three
  // lifts are the source's.
  [
    "chinese:午",
    {
      script: "chinese",
      glyph: "午",
      strokes: [
        simpleStroke(
          "draw the short left-falling stroke",
          compactPath("276 821 290 786 264 701 196 590 82 490"),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal",
          compactPath("224 649 446 662 483 645 669 674 846 679"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("68 355 154 339 227 350 689 345 807 355 932 323"),
        ),
        simpleStroke(
          "lift, then draw the vertical through both",
          compactPath("458 691 510 546 503 -67"),
        ),
      ],
      source: chineseCharacterSource("午"),
    },
  ],
  // 晚: Hanzi Writer Data's eleven ordered medians, fitted to the vendored Noto
  // Sans SC 晚. Medians 1–4 draw 日 on the left as 日 is written alone. Medians
  // 5–11 draw 免 on the right: a short left-falling stroke, a horizontal that
  // turns to fall to the lower left without lifting, a box drawn as 口 is (its
  // left side, the joined top and right side, the closing bottom), then the two
  // strokes of 儿 from inside it: a left-falling stroke, and a vertical that
  // bends right along the foot and hooks up without lifting. The order, the
  // directions, the turns, the hook and all ten lifts are the source's.
  [
    "chinese:晚",
    {
      script: "chinese",
      glyph: "晚",
      strokes: [
        simpleStroke(
          "draw 日's left side",
          compactPath("90 699 116 625 121 550 105 155 114 48"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("93 716 258 731 293 720 317 625 308 130"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("111 445 315 445"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("111 150 315 150"),
        ),
        simpleStroke(
          "lift, then draw 免's short left-falling stroke",
          compactPath("563 821 567 773 547 727 487 645 375 533"),
        ),
        simpleStroke(
          "lift, then draw the horizontal and turn down to the left",
          compactPath("504 718 711 718 754 697 638 502"),
        ),
        simpleStroke(
          "lift, then draw the box's left side",
          compactPath("451 548 451 320"),
        ),
        simpleStroke(
          "lift, then draw the box's top and turn down its right side",
          compactPath("451 552 885 552 885 330"),
        ),
        simpleStroke(
          "lift, then close the box with a horizontal",
          compactPath("451 310 909 310"),
        ),
        simpleStroke(
          "lift, then draw 儿's left-falling stroke",
          compactPath("636 573 654 409 622 265 579 157 460 29 308 -55"),
        ),
        simpleStroke(
          "lift, then draw 儿's vertical, bend right along the foot and hook up",
          compactPath(
            "698 328 717 231 707 104 716 -22 844 -33 917 -1 928 11 933 82 924 105",
          ),
        ),
      ],
      source: chineseCharacterSource("晚"),
    },
  ],
  // 手: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 手. Median 1 draws a flat left-falling stroke across the top;
  // medians 2–3 draw the two horizontals; median 4 draws the vertical last,
  // from the top through all three, hooking to the left at its foot without
  // lifting. The order, the directions, the hook and all three lifts are the
  // source's.
  [
    "chinese:手",
    {
      script: "chinese",
      glyph: "手",
      strokes: [
        simpleStroke(
          "draw the flat top stroke from right to left",
          compactPath("817 799 483 746 140 724"),
        ),
        simpleStroke(
          "lift, then draw the middle horizontal",
          compactPath("163 537 371 519 500 534 605 520 826 542"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("64 289 150 273 226 287 525 299 635 287 818 307 941 273"),
        ),
        simpleStroke(
          "lift, then draw the vertical through all three and hook left",
          compactPath("449 763 486 710 507 614 509 57 481 -17 453 -43 273 -18"),
        ),
      ],
      source: chineseCharacterSource("手"),
    },
  ],
  // 说: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 说. Medians 1–2 draw the speech piece 讠 as 谢 opens: a dot, then a
  // short horizontal that turns down and flicks up to the right without
  // lifting. Medians 3–9 draw 兑 on the right: two short strokes spreading at
  // the top, 口 in the order the standalone character uses, then 儿: a left-
  // falling stroke and a vertical that bends right along the foot and hooks up
  // without lifting. The order, the directions, the turns, the flick, the hook
  // and all eight lifts are the source's.
  [
    "chinese:说",
    {
      script: "chinese",
      glyph: "说",
      strokes: [
        simpleStroke(
          "draw 讠's dot",
          compactPath("138 779 233 715 281 647"),
        ),
        simpleStroke(
          "lift, then draw 讠's short horizontal, turn down and flick up to the right",
          compactPath("59 495 228 495 225 120 192 15 280 94 379 153"),
        ),
        simpleStroke(
          "lift, then draw the dot at the top",
          compactPath("452 806 557 672"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke at the top right",
          compactPath("789 817 795 753 678 584"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("420 617 420 348"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("420 607 834 615 834 360"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("420 353 834 353"),
        ),
        simpleStroke(
          "lift, then draw 儿's left-falling stroke",
          compactPath("522 375 526 251 538 218 519 150 450 36 343 -51"),
        ),
        simpleStroke(
          "lift, then draw 儿's vertical, bend right along the foot and hook up",
          compactPath(
            "663 372 708 314 697 143 713 -14 804 -30 897 -18 914 2 913 97",
          ),
        ),
      ],
      source: chineseCharacterSource("说"),
    },
  ],
  // 话: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 话. Medians 1–2 draw the speech piece 讠 as 谢 opens. Medians 3–8 draw
  // 舌 on the right: a flat left-falling stroke across the top, the long
  // horizontal, a vertical down from the top stroke, then 口 under them in the
  // order the standalone character uses. The order, the directions, the turns,
  // the flick and all seven lifts are the source's.
  [
    "chinese:话",
    {
      script: "chinese",
      glyph: "话",
      strokes: [
        simpleStroke(
          "draw 讠's dot",
          compactPath("137 785 267 661"),
        ),
        simpleStroke(
          "lift, then draw 讠's short horizontal, turn down and flick up to the right",
          compactPath("55 492 218 492 216 120 183 10 365 137"),
        ),
        simpleStroke(
          "lift, then draw 舌's flat top stroke from right to left",
          compactPath("876 785 388 721"),
        ),
        simpleStroke(
          "lift, then draw 舌's long horizontal",
          compactPath("386 502 934 502"),
        ),
        simpleStroke(
          "lift, then draw 舌's vertical down to 口",
          compactPath("655 745 650 280"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("454 277 454 -48"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("454 258 862 258 862 -5"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("454 -8 862 -8"),
        ),
      ],
      source: chineseCharacterSource("话"),
    },
  ],
  // 想: Hanzi Writer Data's thirteen ordered medians, fitted to the vendored
  // Noto Sans SC 想. in three pieces. Medians 1–4 draw 木 at the upper left: a
  // horizontal, a vertical, a left-falling stroke and a dot. Medians 5–9 draw 目
  // beside it: its left side, the joined top and right side, two horizontals
  // inside and the closing bottom. Medians 10–13 draw 心 under them: a dot, a
  // curve that runs along the foot and hooks up without lifting, then two dots.
  // The order, the directions, the turn, the hook and all twelve lifts are the
  // source's.
  [
    "chinese:想",
    {
      script: "chinese",
      glyph: "想",
      strokes: [
        simpleStroke(
          "draw 木's horizontal",
          compactPath("69 660 273 646 455 655"),
        ),
        simpleStroke(
          "lift, then draw 木's vertical",
          compactPath("264 820 281 772 285 665 272 271"),
        ),
        simpleStroke(
          "lift, then draw 木's left-falling stroke",
          compactPath("296 701 228 541 106 386 62 352"),
        ),
        simpleStroke(
          "lift, then draw 木's dot",
          compactPath("234 576 417 460 442 422"),
        ),
        simpleStroke(
          "lift, then draw 目's left side",
          compactPath("522 721 545 643 547 279"),
        ),
        simpleStroke(
          "lift, then draw 目's top and turn down its right side",
          compactPath("525 753 790 758 813 748 865 685 862 385 849 278"),
        ),
        simpleStroke(
          "lift, then draw 目's upper horizontal",
          compactPath("525 566 559 591 651 608 824 601"),
        ),
        simpleStroke(
          "lift, then draw 目's lower horizontal",
          compactPath("524 426 606 448 843 440"),
        ),
        simpleStroke(
          "lift, then close 目 with a horizontal",
          compactPath("524 291 868 292"),
        ),
        simpleStroke(
          "lift, then draw 心's left dot",
          compactPath("170 183 153 110 99 -1"),
        ),
        simpleStroke(
          "lift, then draw 心's curve along the foot and hook up",
          compactPath("307 187 333 19 385 -17 508 -29 683 -15 725 6 700 99"),
        ),
        simpleStroke(
          "lift, then draw 心's middle dot",
          compactPath("441 234 529 176 577 123"),
        ),
        simpleStroke(
          "lift, then draw 心's right dot",
          compactPath("789 195 840 158 922 29"),
        ),
      ],
      source: chineseCharacterSource("想"),
    },
  ],
  // 机: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 机. Medians 1–4 draw 木 on the left as it opens 想: a horizontal, a
  // vertical, a left-falling stroke and a dot. Medians 5–6 draw 几 on the right:
  // a vertical that curves to the lower left, then a horizontal that turns down
  // the right side, bends right along the foot and hooks up without lifting.
  // The order, the directions, the turns, the hook and all five lifts are the
  // source's.
  [
    "chinese:机",
    {
      script: "chinese",
      glyph: "机",
      strokes: [
        simpleStroke(
          "draw 木's horizontal",
          compactPath("63 587 225 573 422 592"),
        ),
        simpleStroke(
          "lift, then draw 木's vertical",
          compactPath("237 825 264 745 255 -66"),
        ),
        simpleStroke(
          "lift, then draw 木's left-falling stroke",
          compactPath("275 663 216 454 127 259 54 162"),
        ),
        simpleStroke(
          "lift, then draw 木's dot",
          compactPath("229 470 420 284"),
        ),
        simpleStroke(
          "lift, then draw 几's left side, curving to the lower left",
          compactPath("534 771 520 250 480 100 394 -50"),
        ),
        simpleStroke(
          "lift, then draw 几's top, turn down, bend right along the foot and hook up",
          compactPath(
            "534 740 796 740 796 60 815 -20 880 -40 935 -10 948 60 941 152",
          ),
        ),
      ],
      source: chineseCharacterSource("机"),
    },
  ],
  // 白: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 白. Median 1 draws a short left-falling stroke at the top; medians
  // 2–5 draw 日 under it as 日 is written alone: its left side, the joined top
  // and right side, the middle horizontal, the closing bottom. The order, the
  // directions, the turn and all four lifts are the source's.
  [
    "chinese:白",
    {
      script: "chinese",
      glyph: "白",
      strokes: [
        simpleStroke(
          "draw the short left-falling stroke at the top",
          compactPath("471 826 445 729 401 645"),
        ),
        simpleStroke(
          "lift, then draw 日's left side",
          compactPath("181 667 181 -66"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("181 640 818 640 818 -62"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("181 335 818 335"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("181 30 818 30"),
        ),
      ],
      source: chineseCharacterSource("白"),
    },
  ],
  // 长: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 长. Median 1 draws a short left-falling stroke at the upper right;
  // median 2 draws the long horizontal; median 3 draws the vertical on the left
  // through it, which flicks up to the right at its foot without lifting;
  // median 4 draws the right-falling stroke from under the horizontal. The
  // order, the directions, the flick and all three lifts are the source's.
  [
    "chinese:长",
    {
      script: "chinese",
      glyph: "长",
      strokes: [
        simpleStroke(
          "draw the short left-falling stroke",
          compactPath("809 786 560 600 474 548 438 540"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath(
            "67 422 166 394 241 402 278 385 429 407 504 394 766 417 926 390",
          ),
        ),
        simpleStroke(
          "lift, then draw the vertical and flick up to the right",
          compactPath("262 822 297 771 294 32 318 -16 553 72"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("427 435 505 384 607 209 759 53 817 24 933 4"),
        ),
      ],
      source: chineseCharacterSource("长"),
    },
  ],
  // 快: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 快. Medians 1–3 draw the heart piece 忄 on the left: a dot on the
  // left, a dot on the right, then the vertical between them. Medians 4–7 draw
  // 夬 on the right: a horizontal that turns down without lifting, the long
  // horizontal, a left-falling stroke from the top through both, then the
  // right-falling stroke. The order, the directions, the turn and all six lifts
  // are the source's.
  [
    "chinese:快",
    {
      script: "chinese",
      glyph: "快",
      strokes: [
        simpleStroke(
          "draw 忄's left dot",
          compactPath("99 628 92 500 58 393"),
        ),
        simpleStroke(
          "lift, then draw 忄's right dot",
          compactPath("279 655 353 500"),
        ),
        simpleStroke(
          "lift, then draw 忄's vertical",
          compactPath("186 824 217 745 202 -66"),
        ),
        simpleStroke(
          "lift, then draw 夬's top and turn down",
          compactPath("406 638 842 630 842 370"),
        ),
        simpleStroke(
          "lift, then draw 夬's long horizontal",
          compactPath("344 348 436 336 622 358 660 346 812 354 951 327"),
        ),
        simpleStroke(
          "lift, then draw 夬's left-falling stroke through both",
          compactPath("617 824 617 500 600 300 550 150 438 28 338 -47"),
        ),
        simpleStroke(
          "lift, then draw 夬's right-falling stroke",
          compactPath("587 373 611 313 672 231 729 121 794 42 941 -31"),
        ),
      ],
      source: chineseCharacterSource("快"),
    },
  ],
  // 电: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 电. Medians 1–4 draw 日 as 日 is written alone: its left side, the
  // joined top and right side, the middle horizontal, the closing bottom.
  // Median 5 draws the vertical through the middle from above the box, bending
  // right along the foot and hooking up without lifting. The order, the
  // directions, the turns, the hook and all four lifts are the source's.
  [
    "chinese:电",
    {
      script: "chinese",
      glyph: "电",
      strokes: [
        simpleStroke(
          "draw 日's left side",
          compactPath("164 682 164 142"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("164 660 826 660 826 215"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("164 443 826 443"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("164 225 826 225"),
        ),
        simpleStroke(
          "lift, then draw the vertical through the middle, bend right along the foot and hook up",
          compactPath(
            "468 823 492 736 477 198 482 46 494 17 530 -10 617 -29 751 -29 854 -7 908 23 917 142",
          ),
        ),
      ],
      source: chineseCharacterSource("电"),
    },
  ],
  // 打: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 打. Medians 1–3 draw the hand piece 扌 on the left: a horizontal, a
  // vertical that hooks to the left at its foot without lifting, then a stroke
  // that rises to the right across it. Medians 4–5 draw 丁 on the right: a
  // horizontal, then a vertical that hooks to the left at its foot without
  // lifting. The order, the directions, the two hooks and all four lifts are
  // the source's.
  [
    "chinese:打",
    {
      script: "chinese",
      glyph: "打",
      strokes: [
        simpleStroke(
          "draw 扌's horizontal",
          compactPath("60 600 395 600"),
        ),
        simpleStroke(
          "lift, then draw 扌's vertical and hook left",
          compactPath("236 827 236 60 220 -20 108 -25"),
        ),
        simpleStroke(
          "lift, then draw 扌's rising stroke",
          compactPath("65 283 230 320 403 380"),
        ),
        simpleStroke(
          "lift, then draw 丁's horizontal",
          compactPath("444 722 934 722"),
        ),
        simpleStroke(
          "lift, then draw 丁's vertical and hook left",
          compactPath("743 722 743 40 720 -20 640 -33 536 -17"),
        ),
      ],
      source: chineseCharacterSource("打"),
    },
  ],
  // 起: Hanzi Writer Data's ten ordered medians, fitted to the vendored Noto
  // Sans SC 起. Medians 1–7 draw 走 on the left: a horizontal, a vertical through
  // it, the long horizontal, a lower vertical, a short horizontal to its right,
  // a left-falling stroke, then the long right-falling stroke along the foot.
  // Medians 8–10 draw 己 on the right above that stroke: a horizontal that turns
  // down without lifting, the middle horizontal, then a vertical that bends
  // right along the foot and hooks up without lifting. The order, the
  // directions, the turns, the hook and all nine lifts are the source's.
  [
    "chinese:起",
    {
      script: "chinese",
      glyph: "起",
      strokes: [
        simpleStroke(
          "draw 走's upper horizontal",
          compactPath("94 693 446 693"),
        ),
        simpleStroke(
          "lift, then draw the vertical through it",
          compactPath("276 823 276 520"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("68 497 480 497"),
        ),
        simpleStroke(
          "lift, then draw the lower vertical",
          compactPath("293 490 293 52"),
        ),
        simpleStroke(
          "lift, then draw the short horizontal to its right",
          compactPath("292 287 462 287"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke",
          compactPath("134 368 115 110 68 -58"),
        ),
        simpleStroke(
          "lift, then draw the long sweep along the foot",
          compactPath("112 203 216 93 351 2 747 -30 945 -3"),
        ),
        simpleStroke(
          "lift, then draw 己's top and turn down",
          compactPath("558 760 869 760 869 448"),
        ),
        simpleStroke(
          "lift, then draw 己's middle horizontal",
          compactPath("568 483 884 483"),
        ),
        simpleStroke(
          "lift, then draw 己's last stroke down, along the foot and hook up",
          compactPath("583 470 586 160 620 115 890 125 915 170 927 256"),
        ),
      ],
      source: chineseCharacterSource("起"),
    },
  ],
  // 门: Hanzi Writer Data's three ordered medians, fitted to the vendored Noto
  // Sans SC 门. Median 1 draws a dot at the top left; median 2 draws the left
  // vertical; median 3 draws a horizontal that turns down the right side and
  // hooks to the left at the foot without lifting. These are the last three
  // strokes of 们, in the same order. The order, the directions, the turn, the
  // hook and all two lifts are the source's.
  [
    "chinese:门",
    {
      script: "chinese",
      glyph: "门",
      strokes: [
        simpleStroke(
          "draw 门's dot",
          compactPath("158 811 263 704 297 652"),
        ),
        simpleStroke(
          "lift, then draw 门's left vertical",
          compactPath("111 621 140 604 125 -65"),
        ),
        simpleStroke(
          "lift, then draw 门's top, turn down the right side and hook left",
          compactPath(
            "391 791 563 760 799 777 864 758 868 747 874 22 849 -38 756 -42 670 -29",
          ),
        ),
      ],
      source: chineseCharacterSource("门"),
    },
  ],
  // 问: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 问. Medians 1–3 draw 门 as it closes 们: a dot, the left vertical,
  // then a horizontal that turns down the right side and hooks to the left
  // without lifting. Medians 4–6 draw 口 inside it in the order the standalone
  // character uses. The order, the directions, the turns, the hook and all five
  // lifts are the source's.
  [
    "chinese:问",
    {
      script: "chinese",
      glyph: "问",
      strokes: [
        simpleStroke(
          "draw 门's dot",
          compactPath("138 793 277 658"),
        ),
        simpleStroke(
          "lift, then draw 门's left vertical",
          compactPath("117 603 137 580 127 -66"),
        ),
        simpleStroke(
          "lift, then draw 门's top, turn down the right side and hook left",
          compactPath(
            "368 761 537 740 663 741 799 762 858 725 863 6 844 -38 694 -16",
          ),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("334 502 355 428 355 299 380 172"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("335 507 415 494 603 498 655 449 632 373 606 183"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("336 189 423 205 617 205 661 192"),
        ),
      ],
      source: chineseCharacterSource("问"),
    },
  ],
  // 气: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 气. Median 1 draws a short left-falling stroke; medians 2–3 draw two
  // horizontals, the lower one shorter and set to the right; median 4 draws a
  // horizontal that turns down, sweeps out to the right along the foot and
  // hooks up without lifting. The order, the directions, the turn, the hook and
  // all three lifts are the source's.
  [
    "chinese:气",
    {
      script: "chinese",
      glyph: "气",
      strokes: [
        simpleStroke(
          "draw the left-falling stroke",
          compactPath("276 820 281 769 249 687 155 554 61 466"),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal",
          compactPath("223 690 899 698"),
        ),
        simpleStroke(
          "lift, then draw the shorter horizontal",
          compactPath("278 560 829 560"),
        ),
        simpleStroke(
          "lift, then draw the last stroke across, down, out to the right and hook up",
          compactPath(
            "176 413 735 405 740 250 795 30 840 -25 890 -38 925 4 935 84",
          ),
        ),
      ],
      source: chineseCharacterSource("气"),
    },
  ],
  // 雨: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 雨. Median 1 draws the top horizontal; medians 2–3 draw the frame,
  // its left side, then a horizontal that turns down the right side and hooks
  // to the left without lifting; median 4 draws the vertical down the middle;
  // medians 5–8 draw the four dots, the two on the left first, upper then
  // lower, then the two on the right. The order, the directions, the turn, the
  // hook and all seven lifts are the source's.
  [
    "chinese:雨",
    {
      script: "chinese",
      glyph: "雨",
      strokes: [
        simpleStroke(
          "draw the top horizontal",
          compactPath("133 763 307 732 405 735 478 720 723 747 919 730"),
        ),
        simpleStroke(
          "lift, then draw the frame's left side",
          compactPath("110 512 135 432 141 272 129 54 157 -64"),
        ),
        simpleStroke(
          "lift, then draw the frame's top, turn down and hook left",
          compactPath(
            "111 493 177 530 258 535 495 525 849 541 868 531 878 479 859 -3 827 -44 716 -37 687 -19",
          ),
        ),
        simpleStroke(
          "lift, then draw the middle vertical",
          compactPath("470 693 504 612 490 128 496 -54"),
        ),
        simpleStroke(
          "lift, then draw the upper left dot",
          compactPath("240 407 317 371 396 312"),
        ),
        simpleStroke(
          "lift, then draw the lower left dot",
          compactPath("231 211 334 162 397 113"),
        ),
        simpleStroke(
          "lift, then draw the upper right dot",
          compactPath("604 411 768 325"),
        ),
        simpleStroke(
          "lift, then draw the lower right dot",
          compactPath("588 214 756 112"),
        ),
      ],
      source: chineseCharacterSource("雨"),
    },
  ],
  // 马: Hanzi Writer Data's three ordered medians, fitted to the vendored Noto
  // Sans SC 马. Median 1 draws a horizontal that turns down without lifting;
  // median 2 draws a long stroke that runs down, turns right, turns down again
  // and hooks to the left without lifting; median 3 draws a rising stroke.
  // These are the last three strokes of 吗 and 妈, in the same order. The order,
  // the directions, the turns, the hook and all two lifts are the source's.
  [
    "chinese:马",
    {
      script: "chinese",
      glyph: "马",
      strokes: [
        simpleStroke(
          "draw the top and turn down",
          compactPath("153 764 349 737 657 750 730 706 729 618 686 339"),
        ),
        simpleStroke(
          "lift, then draw the long stroke down, right, down and hook left",
          compactPath(
            "248 618 275 576 254 381 398 346 470 359 811 364 847 360 881 328 832 77 787 -25 759 -46 596 -28",
          ),
        ),
        simpleStroke(
          "lift, then draw the rising horizontal",
          compactPath("72 170 169 149 468 171 638 168 695 150"),
        ),
      ],
      source: chineseCharacterSource("马"),
    },
  ],
  // 山: Hanzi Writer Data's three ordered medians, fitted to the vendored Noto
  // Sans SC 山. Median 1 draws the tall vertical down the middle; median 2 draws
  // the left side and turns right along the foot without lifting; median 3
  // draws the right vertical last. The order, the directions, the turn and all
  // two lifts are the source's.
  [
    "chinese:山",
    {
      script: "chinese",
      glyph: "山",
      strokes: [
        simpleStroke(
          "draw the middle vertical",
          compactPath("498 812 498 40"),
        ),
        simpleStroke(
          "lift, then draw the left side and turn along the foot",
          compactPath("146 613 146 35 830 35"),
        ),
        simpleStroke(
          "lift, then draw the right vertical",
          compactPath("854 616 854 -62"),
        ),
      ],
      source: chineseCharacterSource("山"),
    },
  ],
  // 做: Hanzi Writer Data's eleven ordered medians, fitted to the vendored Noto
  // Sans SC 做. in three pieces. Medians 1–2 draw 亻 on the left as 你 opens.
  // Medians 3–7 draw 古 in the middle: 十's horizontal and vertical, then 口 under
  // them in the order the standalone character uses. Medians 8–11 draw 攵 on the
  // right: a short left-falling stroke, a horizontal, a long left-falling
  // stroke from the end of the horizontal, then the right-falling stroke across
  // it. The order, the directions, the turn and all ten lifts are the source's.
  [
    "chinese:做",
    {
      script: "chinese",
      glyph: "做",
      strokes: [
        simpleStroke(
          "draw 亻's left-falling stroke",
          compactPath("249 813 258 773 215 671 119 487 53 393"),
        ),
        simpleStroke(
          "lift, then draw 亻's vertical down to the baseline",
          compactPath("154 615 193 405 189 -67"),
        ),
        simpleStroke(
          "lift, then draw 古's horizontal",
          compactPath("289 590 355 589 390 601 459 593 598 622"),
        ),
        simpleStroke(
          "lift, then draw 古's vertical",
          compactPath("423 809 457 730 431 361 408 333"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("332 371 332 -22"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("332 345 560 345 560 80"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("332 67 560 67"),
        ),
        simpleStroke(
          "lift, then draw 攵's short left-falling stroke",
          compactPath("709 822 716 759 700 657 637 464 606 404 538 307"),
        ),
        simpleStroke(
          "lift, then draw 攵's horizontal",
          compactPath("672 597 680 606 937 606"),
        ),
        simpleStroke(
          "lift, then draw 攵's long left-falling stroke",
          compactPath("889 630 817 270 724 90 579 -48"),
        ),
        simpleStroke(
          "lift, then draw 攵's right-falling stroke",
          compactPath("692 440 738 247 792 127 929 -48"),
        ),
      ],
      source: chineseCharacterSource("做"),
    },
  ],
  // 火: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 火. Medians 1–2 draw the two short strokes on either side; medians
  // 3–4 draw the long left-falling stroke from the top and the right-falling
  // stroke from its middle, as 人 is drawn. Noto's left stroke leans to the left
  // where the source's leans right; the pen still writes it downward. The
  // order, the directions and all three lifts are the source's.
  [
    "chinese:火",
    {
      script: "chinese",
      glyph: "火",
      strokes: [
        simpleStroke(
          "draw the short stroke on the left",
          compactPath("240 612 168 430 118 358"),
        ),
        simpleStroke(
          "lift, then draw the short stroke on the right",
          compactPath("876 606 792 428 741 348"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath(
            "478 815 493 716 480 492 450 349 409 230 368 142 313 78 216 12 90 -40",
          ),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("454 481 504 344 563 231 664 98 779 8 928 -26"),
        ),
      ],
      source: chineseCharacterSource("火"),
    },
  ],
  // 开: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 开. Medians 1–2 draw the two horizontals, the lower one longer;
  // median 3 draws the left leg, down through both and curving out to the lower
  // left; median 4 draws the right leg, a vertical through both. The order, the
  // directions and all three lifts are the source's.
  [
    "chinese:开",
    {
      script: "chinese",
      glyph: "开",
      strokes: [
        simpleStroke(
          "draw the upper horizontal",
          compactPath("169 752 356 729 847 745"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath(
            "63 400 161 372 318 389 590 383 704 394 743 382 874 385 934 363",
          ),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke through both",
          compactPath(
            "306 675 324 593 341 583 338 402 313 228 283 139 249 86 87 -38",
          ),
        ),
        simpleStroke(
          "lift, then draw the vertical through both",
          compactPath("607 763 677 706 684 654 684 -66"),
        ),
      ],
      source: chineseCharacterSource("开"),
    },
  ],
  // 下: Hanzi Writer Data's three ordered medians, fitted to the vendored Noto
  // Sans SC 下. Median 1 draws the horizontal; median 2 draws the vertical down
  // from it; median 3 draws the dot to the right of the vertical. The order,
  // the directions and all two lifts are the source's.
  [
    "chinese:下",
    {
      script: "chinese",
      glyph: "下",
      strokes: [
        simpleStroke(
          "draw the horizontal",
          compactPath("66 743 165 717 276 727 497 712 791 736 932 711"),
        ),
        simpleStroke(
          "lift, then draw the vertical down from it",
          compactPath("451 748 484 637 486 -65"),
        ),
        simpleStroke(
          "lift, then draw the dot",
          compactPath("526 498 849 286"),
        ),
      ],
      source: chineseCharacterSource("下"),
    },
  ],
  // 进: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 进. in two pieces, the walking piece written last. Medians 1–4 draw
  // 井 on the upper right: two horizontals, then a left-falling stroke and a
  // vertical through both. Medians 5–7 draw 辶 under it as 这 closes: a dot, a
  // stroke that runs right, turns down, turns again and falls to the left
  // without lifting, then the long flat right-falling stroke along the foot.
  // The order, the directions, the turns and all six lifts are the source's.
  [
    "chinese:进",
    {
      script: "chinese",
      glyph: "进",
      strokes: [
        simpleStroke(
          "draw 井's upper horizontal",
          compactPath("361 625 899 625"),
        ),
        simpleStroke(
          "lift, then draw 井's lower horizontal",
          compactPath("356 377 919 377"),
        ),
        simpleStroke(
          "lift, then draw 井's left-falling stroke through both",
          compactPath("518 807 518 500 505 320 470 200 395 104"),
        ),
        simpleStroke(
          "lift, then draw 井's vertical through both",
          compactPath("730 804 749 765 750 92"),
        ),
        simpleStroke(
          "lift, then draw 辶's dot",
          compactPath("107 782 225 696 254 645"),
        ),
        simpleStroke(
          "lift, then draw 辶's short horizontal, turn down, turn again and fall left",
          compactPath("71 435 224 435 222 110 160 50 95 20"),
        ),
        simpleStroke(
          "lift, then draw 辶's long sweep along the foot",
          compactPath(
            "62 10 213 91 252 84 312 43 418 -1 538 -16 758 -24 934 5",
          ),
        ),
      ],
      source: chineseCharacterSource("进"),
    },
  ],
  // 客: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 客. Medians 1–3 draw the roof piece 宀 as 家 opens: a dot, the left
  // side, then a horizontal that hooks down at the right without lifting.
  // Medians 4–9 draw 各 under it: a left-falling stroke, a horizontal that turns
  // to fall to the lower left without lifting, the right-falling stroke across
  // it, then 口 in the order the standalone character uses. The order, the
  // directions, the turns, the hook and all eight lifts are the source's.
  [
    "chinese:客",
    {
      script: "chinese",
      glyph: "客",
      strokes: [
        simpleStroke(
          "draw 宀's top dot",
          compactPath("459 822 521 764 566 694"),
        ),
        simpleStroke(
          "lift, then draw 宀's left side",
          compactPath("112 734 124 682 102 574"),
        ),
        simpleStroke(
          "lift, then draw 宀's top and hook down at the right",
          compactPath("103 713 885 713 885 591"),
        ),
        simpleStroke(
          "lift, then draw 各's left-falling stroke",
          compactPath("394 646 345 570 250 480 134 415"),
        ),
        simpleStroke(
          "lift, then draw 各's horizontal and sweep down to the left",
          compactPath("292 545 340 555 740 555 650 458 500 358 300 266 77 197"),
        ),
        simpleStroke(
          "lift, then draw 各's right-falling stroke",
          compactPath("262 522 379 454 506 359 658 282 756 247 935 245"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("268 205 268 -68"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("268 195 739 195 739 -10"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("268 -15 739 -15"),
        ),
      ],
      source: chineseCharacterSource("客"),
    },
  ],
  // 乐: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 乐. Median 1 draws a flat left-falling stroke across the top; median
  // 2 draws the left side and turns right across the middle without lifting;
  // medians 3–5 draw 小 under it as 小 is written alone: the vertical down the
  // middle, hooked to the left at its foot without lifting, a short stroke on
  // the left, then a dot on the right. The order, the directions, the turn, the
  // hook and all four lifts are the source's.
  [
    "chinese:乐",
    {
      script: "chinese",
      glyph: "乐",
      strokes: [
        simpleStroke(
          "draw the flat top stroke from right to left",
          compactPath("773 753 620 767 535 743 275 731 274 721 183 708"),
        ),
        simpleStroke(
          "lift, then draw the left side and turn right across the middle",
          compactPath(
            "183 751 191 659 207 628 173 398 469 392 807 413 905 376",
          ),
        ),
        simpleStroke(
          "lift, then draw the middle vertical and hook left",
          compactPath("494 623 520 526 510 5 486 -36 372 -35 338 -20"),
        ),
        simpleStroke(
          "lift, then draw the lower left stroke",
          compactPath("257 258 182 124 91 10"),
        ),
        simpleStroke(
          "lift, then draw the lower right dot",
          compactPath("722 245 886 80 920 18"),
        ),
      ],
      source: chineseCharacterSource("乐"),
    },
  ],
  // 会: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 会. Medians 1–2 draw the roof as 今 opens: the left-falling stroke,
  // then the right-falling stroke from a little under the top; medians 3–4 draw
  // two horizontals, the lower one longer; medians 5–6 draw 厶 at the foot as 去
  // closes: a stroke that falls to the lower left and turns right along the
  // foot without lifting, then a dot. The order, the directions, the turn and
  // all five lifts are the source's.
  [
    "chinese:会",
    {
      script: "chinese",
      glyph: "会",
      strokes: [
        simpleStroke(
          "draw the roof's left-falling stroke",
          compactPath("521 813 429 700 258 567 138 497 68 489"),
        ),
        simpleStroke(
          "lift, then draw the roof's right-falling stroke",
          compactPath("485 790 578 694 688 608 864 492 927 492"),
        ),
        simpleStroke(
          "lift, then draw the short horizontal",
          compactPath("284 490 720 490"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("111 300 896 300"),
        ),
        simpleStroke(
          "lift, then draw 厶's stroke down to the left and turn along the foot",
          compactPath("450 273 175 -25 300 8 760 40"),
        ),
        simpleStroke(
          "lift, then draw 厶's dot",
          compactPath("658 201 772 74 802 -1"),
        ),
      ],
      source: chineseCharacterSource("会"),
    },
  ],
  // 课: Hanzi Writer Data's ten ordered medians, fitted to the vendored Noto
  // Sans SC 课. Medians 1–2 draw the speech piece 讠 as 谢 opens. Medians 3–10
  // draw 果 on the right: the box of 田 (its left side, the joined top and right
  // side, the middle horizontal, the closing bottom), then 木's horizontal, one
  // long vertical from the top of the box down through both, and 木's left-
  // falling and right-falling strokes. The order, the directions, the turns,
  // the flick and all nine lifts are the source's.
  [
    "chinese:课",
    {
      script: "chinese",
      glyph: "课",
      strokes: [
        simpleStroke(
          "draw 讠's dot",
          compactPath("126 787 220 718 258 660"),
        ),
        simpleStroke(
          "lift, then draw 讠's short horizontal, turn down and flick up to the right",
          compactPath("66 485 219 485 217 120 178 10 343 139"),
        ),
        simpleStroke(
          "lift, then draw 田's left side",
          compactPath("426 778 426 425"),
        ),
        simpleStroke(
          "lift, then draw 田's top and turn down its right side",
          compactPath("426 763 857 755 857 430"),
        ),
        simpleStroke(
          "lift, then draw 田's middle horizontal",
          compactPath("426 605 857 605"),
        ),
        simpleStroke(
          "lift, then close 田 with a horizontal",
          compactPath("426 433 857 433"),
        ),
        simpleStroke(
          "lift, then draw 木's horizontal",
          compactPath("361 290 926 290"),
        ),
        simpleStroke(
          "lift, then draw the long vertical through 田 and 木",
          compactPath("648 782 648 -68"),
        ),
        simpleStroke(
          "lift, then draw 木's left-falling stroke",
          compactPath("633 277 456 82 347 -8"),
        ),
        simpleStroke(
          "lift, then draw 木's right-falling stroke",
          compactPath("664 285 852 70 937 9"),
        ),
      ],
      source: chineseCharacterSource("课"),
    },
  ],
  // 里: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 里. Medians 1–4 draw the box of 田: its left side, the joined top and
  // right side, the middle horizontal, the closing bottom; median 5 draws the
  // vertical from the top of the box down through it; medians 6–7 draw the two
  // horizontals of 土 on that vertical, the lower one longer. The order, the
  // directions, the turn and all six lifts are the source's.
  [
    "chinese:里",
    {
      script: "chinese",
      glyph: "里",
      strokes: [
        simpleStroke(
          "draw 田's left side",
          compactPath("191 778 191 360"),
        ),
        simpleStroke(
          "lift, then draw 田's top and turn down its right side",
          compactPath("191 763 822 755 822 380"),
        ),
        simpleStroke(
          "lift, then draw 田's middle horizontal",
          compactPath("191 575 822 575"),
        ),
        simpleStroke(
          "lift, then close 田 with a horizontal",
          compactPath("191 385 822 385"),
        ),
        simpleStroke(
          "lift, then draw the long middle vertical",
          compactPath("503 778 503 -10"),
        ),
        simpleStroke(
          "lift, then draw the short lower horizontal",
          compactPath("144 200 872 200"),
        ),
        simpleStroke(
          "lift, then draw the long bottom horizontal",
          compactPath("76 -17 924 -17"),
        ),
      ],
      source: chineseCharacterSource("里"),
    },
  ],
  // 前: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 前. Medians 1–3 draw the top: a dot, a short left-falling stroke,
  // then the long horizontal. Medians 4–7 draw 月 on the lower left: its left
  // side, a horizontal that turns down the right side and hooks to the left
  // without lifting, then two horizontals inside. Medians 8–9 draw the knife
  // piece 刂 on the lower right: a short vertical, then a long one that hooks to
  // the left at its foot without lifting. The order, the directions, the turn,
  // the two hooks and all eight lifts are the source's.
  [
    "chinese:前",
    {
      script: "chinese",
      glyph: "前",
      strokes: [
        simpleStroke(
          "draw the dot at the top left",
          compactPath("237 810 300 763 329 728 335 701"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke at the top right",
          compactPath("733 825 715 770 723 751 684 701 592 624"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("81 645 919 645"),
        ),
        simpleStroke(
          "lift, then draw 月's left side",
          compactPath("151 502 151 -62"),
        ),
        simpleStroke(
          "lift, then draw 月's top, turn down its right side and hook left",
          compactPath("151 495 445 495 443 0 420 -45 324 -45"),
        ),
        simpleStroke(
          "lift, then draw 月's upper horizontal",
          compactPath("151 330 445 330"),
        ),
        simpleStroke(
          "lift, then draw 月's lower horizontal",
          compactPath("151 165 445 165"),
        ),
        simpleStroke(
          "lift, then draw 刂's short vertical",
          compactPath("638 491 638 126"),
        ),
        simpleStroke(
          "lift, then draw 刂's long vertical and hook left",
          compactPath("844 520 844 20 820 -40 696 -37"),
        ),
      ],
      source: chineseCharacterSource("前"),
    },
  ],
  // 在: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 在. Medians 1–2 draw the top as 有 opens: a long horizontal, then a
  // long left-falling stroke from above it; median 3 draws a vertical on the
  // left, down from the left-falling stroke; medians 4–6 draw 土 on the right: a
  // short horizontal, the vertical through it, then the long horizontal along
  // the foot. The order, the directions and all five lifts are the source's.
  [
    "chinese:在",
    {
      script: "chinese",
      glyph: "在",
      strokes: [
        simpleStroke(
          "draw the long horizontal",
          compactPath("93 650 911 650"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("426 817 385 692 313 552 78 246"),
        ),
        simpleStroke(
          "lift, then draw the vertical on the left",
          compactPath("189 404 235 320 226 17 234 -62"),
        ),
        simpleStroke(
          "lift, then draw 土's short horizontal",
          compactPath("396 330 876 330"),
        ),
        simpleStroke(
          "lift, then draw 土's vertical",
          compactPath("611 529 638 435 630 127 610 39"),
        ),
        simpleStroke(
          "lift, then draw 土's long bottom horizontal",
          compactPath("346 -11 435 -25 809 -6 924 -35"),
        ),
      ],
      source: chineseCharacterSource("在"),
    },
  ],
  // 哪: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 哪. Medians 1–3 draw 口 on the left in the order the standalone
  // character uses. Medians 4–9 draw 那 on the right in the order 那 is written
  // alone: a horizontal that turns down the right side and hooks to the left
  // without lifting, two horizontals, a left-falling stroke through them, then
  // 阝: a stroke that runs right, falls back, swells round to the right and
  // hooks to the left without lifting, and the vertical. The order, the
  // directions, the turns, the two hooks and all eight lifts are the source's.
  [
    "chinese:哪",
    {
      script: "chinese",
      glyph: "哪",
      strokes: [
        simpleStroke(
          "draw 口's left side",
          compactPath("102 725 102 106"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("102 705 255 705 255 225"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("102 225 262 225"),
        ),
        simpleStroke(
          "lift, then draw the top, turn down the right side and hook left",
          compactPath("346 757 589 765 585 30 560 -30 461 -45"),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal",
          compactPath("344 525 599 525"),
        ),
        simpleStroke(
          "lift, then draw the lower horizontal",
          compactPath("344 285 596 285"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke through the long horizontal",
          compactPath("442 777 442 450 425 250 390 120 295 -46"),
        ),
        simpleStroke(
          "lift, then draw 阝's ear across, back, round and hook left",
          compactPath("706 757 919 752 825 460 900 280 905 160 878 115 816 97"),
        ),
        simpleStroke(
          "lift, then draw 阝's vertical",
          compactPath("709 779 719 -68"),
        ),
      ],
      source: chineseCharacterSource("哪"),
    },
  ],
  // 没: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 没. in three pieces. Medians 1–3 draw the water piece 氵 as 汉 opens:
  // two dots, then a rising stroke. Medians 4–5 draw the small roof: a short
  // left-falling stroke, then a horizontal that turns down and runs out to the
  // right without lifting. Medians 6–7 draw 又 under it as 对 draws it: a
  // horizontal that turns down to the left without lifting, then the right-
  // falling stroke. The order, the directions, the turns and all six lifts are
  // the source's.
  [
    "chinese:没",
    {
      script: "chinese",
      glyph: "没",
      strokes: [
        simpleStroke(
          "draw 氵's upper dot",
          compactPath("122 803 246 723 266 682"),
        ),
        simpleStroke(
          "lift, then draw 氵's middle dot",
          compactPath("75 534 158 482 215 416"),
        ),
        simpleStroke(
          "lift, then draw 氵's rising stroke",
          compactPath("136 -27 131 28 257 277"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke",
          compactPath("461 789 480 752 480 662 463 595 424 511 332 445"),
        ),
        simpleStroke(
          "lift, then draw the top, turn down and turn right",
          compactPath(
            "461 759 490 749 614 776 683 762 738 723 722 605 736 528 767 503 884 512 941 502",
          ),
        ),
        simpleStroke(
          "lift, then draw 又's horizontal and turn down to the left",
          compactPath("366 363 860 355 702 160 525 40 343 -40"),
        ),
        simpleStroke(
          "lift, then draw 又's right-falling stroke",
          compactPath("355 382 418 344 465 242 589 111 706 21 813 -20 932 -31"),
        ),
      ],
      source: chineseCharacterSource("没"),
    },
  ],
  // 两: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 两. Median 1 draws the top horizontal; medians 2–3 draw the frame,
  // its left side, then a horizontal that turns down the right side and hooks
  // to the left without lifting; medians 4–7 draw the two small 人, left then
  // right, each a left-falling stroke then a short right-falling one. The
  // order, the directions, the turn, the hook and all six lifts are the
  // source's.
  [
    "chinese:两",
    {
      script: "chinese",
      glyph: "两",
      strokes: [
        simpleStroke(
          "draw the top horizontal",
          compactPath("106 755 274 720 505 730 620 715 735 732 929 712"),
        ),
        simpleStroke(
          "lift, then draw the frame's left side",
          compactPath("114 507 143 386 145 141 127 30 133 -67"),
        ),
        simpleStroke(
          "lift, then draw the frame's top, turn down and hook left",
          compactPath(
            "117 482 147 492 165 514 281 522 369 509 441 520 770 521 831 515 861 497 872 457 863 4 829 -46 708 -41 669 -25",
          ),
        ),
        simpleStroke(
          "lift, then draw the left 人's left-falling stroke",
          compactPath("347 668 378 603 343 321 311 228 215 108"),
        ),
        simpleStroke(
          "lift, then draw the left 人's right-falling stroke",
          compactPath("331 364 439 279 472 229"),
        ),
        simpleStroke(
          "lift, then draw the right 人's left-falling stroke",
          compactPath(
            "587 754 613 662 637 622 616 405 587 283 548 198 474 104",
          ),
        ),
        simpleStroke(
          "lift, then draw the right 人's right-falling stroke",
          compactPath("591 384 698 285 758 204 781 150"),
        ),
      ],
      source: chineseCharacterSource("两"),
    },
  ],
  // 给: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 给. Medians 1–3 draw the silk piece 纟 on the left: a stroke that
  // falls to the left and turns right without lifting, a longer one the same
  // way, then a rising stroke. Medians 4–9 draw 合 on the right: a left-falling
  // and a right-falling stroke spreading as 人 does, a horizontal under them,
  // then 口 in the order the standalone character uses. The order, the
  // directions, the turns and all eight lifts are the source's.
  [
    "chinese:给",
    {
      script: "chinese",
      glyph: "给",
      strokes: [
        simpleStroke(
          "draw 纟's first stroke down to the left and turn right",
          compactPath("248 810 200 682 73 477 294 493 325 509"),
        ),
        simpleStroke(
          "lift, then draw 纟's second stroke down to the left and turn right",
          compactPath("344 606 240 420 150 300 78 237 361 284"),
        ),
        simpleStroke(
          "lift, then draw 纟's rising stroke",
          compactPath("61 33 136 24 199 58 279 69 374 116"),
        ),
        simpleStroke(
          "lift, then draw 合's left-falling stroke",
          compactPath("660 812 562 620 407 447"),
        ),
        simpleStroke(
          "lift, then draw 合's right-falling stroke",
          compactPath("683 715 908 461"),
        ),
        simpleStroke(
          "lift, then draw 合's horizontal",
          compactPath("521 485 789 477"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("485 304 485 -48"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("485 297 819 297 819 10"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("485 5 819 5"),
        ),
      ],
      source: chineseCharacterSource("给"),
    },
  ],
  // 能: Hanzi Writer Data's ten ordered medians, fitted to the vendored Noto
  // Sans SC 能. Medians 1–6 draw the left side: 厶 (a stroke that falls to the
  // left and turns right without lifting, then a dot), then 月 under it (its
  // left side, a horizontal that turns down the right side and hooks to the
  // left without lifting, two horizontals inside). Medians 7–10 draw the two 匕
  // on the right, upper then lower, each a left-falling stroke then a vertical
  // that bends right along its foot and hooks up without lifting. The order,
  // the directions, the turns, the three hooks and all nine lifts are the
  // source's.
  [
    "chinese:能",
    {
      script: "chinese",
      glyph: "能",
      strokes: [
        simpleStroke(
          "draw 厶's stroke down to the left and turn right",
          compactPath("229 819 236 781 102 589 336 618 460 620"),
        ),
        simpleStroke(
          "lift, then draw 厶's dot",
          compactPath("340 748 432 627 456 556"),
        ),
        simpleStroke(
          "lift, then draw 月's left side",
          compactPath("110 439 137 370 144 294 122 -65"),
        ),
        simpleStroke(
          "lift, then draw 月's top, turn down its right side and hook left",
          compactPath(
            "112 447 341 451 360 445 408 380 405 -20 390 -40 356 -44 286 -27",
          ),
        ),
        simpleStroke(
          "lift, then draw 月's upper horizontal",
          compactPath("111 275 205 305 401 291"),
        ),
        simpleStroke(
          "lift, then draw 月's lower horizontal",
          compactPath("112 135 167 153 252 154 420 134"),
        ),
        simpleStroke(
          "lift, then draw the upper 匕's left-falling stroke",
          compactPath("861 742 756 685 643 647 563 593"),
        ),
        simpleStroke(
          "lift, then draw the upper 匕's vertical, bend right and hook up",
          compactPath(
            "562 813 597 740 578 578 580 499 600 461 635 444 713 433 852 436 903 476 909 557",
          ),
        ),
        simpleStroke(
          "lift, then draw the lower 匕's left-falling stroke",
          compactPath("879 284 623 181 563 142"),
        ),
        simpleStroke(
          "lift, then draw the lower 匕's vertical, bend right and hook up",
          compactPath(
            "560 358 595 313 578 213 579 15 594 -16 648 -34 755 -39 850 -31 910 -12 922 0 916 103",
          ),
        ),
      ],
      source: chineseCharacterSource("能"),
    },
  ],
  // 要: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 要. Medians 1–6 draw 覀 on top: the top horizontal, the box's left
  // side, its joined top and right side, the two short verticals inside, then
  // the closing bottom. Medians 7–9 draw 女 under it as 女 is written alone: a
  // stroke that falls to the left and turns down to the right without lifting,
  // a left-falling stroke, then the long horizontal. The order, the directions,
  // the turns and all eight lifts are the source's.
  [
    "chinese:要",
    {
      script: "chinese",
      glyph: "要",
      strokes: [
        simpleStroke(
          "draw the top horizontal",
          compactPath("91 765 909 765"),
        ),
        simpleStroke(
          "lift, then draw the box's left side",
          compactPath("154 615 154 417"),
        ),
        simpleStroke(
          "lift, then draw the box's top and turn down its right side",
          compactPath("154 613 850 605 850 430"),
        ),
        simpleStroke(
          "lift, then draw the left inner vertical",
          compactPath("377 733 377 430"),
        ),
        simpleStroke(
          "lift, then draw the right inner vertical",
          compactPath("611 733 611 430"),
        ),
        simpleStroke(
          "lift, then close the box with a horizontal",
          compactPath("154 417 850 417"),
        ),
        simpleStroke(
          "lift, then draw 女's first stroke down-left, then turn down to the right",
          compactPath("414 390 380 300 250 118 445 90 600 33 743 8 860 -47"),
        ),
        simpleStroke(
          "lift, then draw 女's left-falling stroke",
          compactPath("717 283 656 150 600 95 525 50 420 10 94 -47"),
        ),
        simpleStroke(
          "lift, then draw 女's long horizontal",
          compactPath("74 263 918 263"),
        ),
      ],
      source: chineseCharacterSource("要"),
    },
  ],
  // 也: Hanzi Writer Data's three ordered medians, fitted to the vendored Noto
  // Sans SC 也. Median 1 draws a horizontal that rises to the right, turns down
  // and hooks to the left without lifting; median 2 draws a vertical through
  // the middle; median 3 draws a long vertical that bends right along the foot
  // and hooks up without lifting. These are the last three strokes of 他, in the
  // same order. The order, the directions, the two turns, the two hooks and all
  // two lifts are the source's.
  [
    "chinese:也",
    {
      script: "chinese",
      glyph: "也",
      strokes: [
        simpleStroke(
          "draw the rising horizontal, turn down the right side and hook left",
          compactPath(
            "52 408 431 510 835 650 830 400 811 300 780 250 713 225 650 234",
          ),
        ),
        simpleStroke(
          "lift, then draw the middle vertical",
          compactPath("512 823 536 714 524 144"),
        ),
        simpleStroke(
          "lift, then draw the long vertical, bend right along the foot and hook up",
          compactPath(
            "250 756 250 80 270 15 386 -23 672 -28 817 -17 905 30 905 158",
          ),
        ),
      ],
      source: chineseCharacterSource("也"),
    },
  ],
  // 太: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 太. Medians 1–3 draw 大 as 大 is written alone: the horizontal, the
  // left-falling stroke from above it, then the right-falling stroke; median 4
  // draws the dot between the two legs. The order, the directions and all three
  // lifts are the source's.
  [
    "chinese:太",
    {
      script: "chinese",
      glyph: "太",
      strokes: [
        simpleStroke(
          "draw the horizontal",
          compactPath("77 537 209 518 403 530 476 509 731 540 856 508"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke",
          compactPath(
            "477 825 492 789 494 732 479 577 449 402 384 246 339 152 286 90 182 8 77 -36",
          ),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("508 585 559 381 614 261 686 145 755 65 825 8 935 -29"),
        ),
        simpleStroke(
          "lift, then draw the dot between the legs",
          compactPath("315 172 411 132 450 102 549 -51"),
        ),
      ],
      source: chineseCharacterSource("太"),
    },
  ],
  // 都: Hanzi Writer Data's ten ordered medians, fitted to the vendored Noto
  // Sans SC 都. Medians 1–8 draw 者 on the left: a short horizontal, the vertical
  // through it, the long horizontal, a long left-falling stroke across them,
  // then 日 under them as 日 is written alone. Medians 9–10 draw 阝 on the right
  // as 那 closes: a stroke that runs right, falls back to the left, swells round
  // to the right and hooks to the left without lifting, then the vertical. The
  // order, the directions, the turns, the hook and all nine lifts are the
  // source's.
  [
    "chinese:都",
    {
      script: "chinese",
      glyph: "都",
      strokes: [
        simpleStroke(
          "draw the short top horizontal",
          compactPath("111 695 411 695"),
        ),
        simpleStroke(
          "lift, then draw the vertical through it",
          compactPath("278 807 278 520"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("64 500 531 500"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("530 783 485 680 428 590 300 440 209 363 57 268"),
        ),
        simpleStroke(
          "lift, then draw 日's left side",
          compactPath("183 330 183 -55"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("183 343 479 343 479 -32"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("183 185 479 185"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("183 13 479 13"),
        ),
        simpleStroke(
          "lift, then draw 阝's ear across, back, round and hook left",
          compactPath(
            "626 755 945 755 904 700 790 450 899 290 913 230 905 150 851 97 761 90",
          ),
        ),
        simpleStroke(
          "lift, then draw 阝's vertical",
          compactPath("640 769 640 -68"),
        ),
      ],
      source: chineseCharacterSource("都"),
    },
  ],
  // 为: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 为. Median 1 draws a dot at the upper left; median 2 draws the long
  // left-falling stroke from the top; median 3 draws a horizontal that turns
  // down the right side and hooks to the left without lifting, as 男's 力 does;
  // median 4 draws the dot inside. The order, the directions, the turn, the
  // hook and all three lifts are the source's.
  [
    "chinese:为",
    {
      script: "chinese",
      glyph: "为",
      strokes: [
        simpleStroke(
          "draw the dot at the upper left",
          compactPath("198 787 307 665"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("452 823 443 570 401 374 347 254 267 134 110 -42"),
        ),
        simpleStroke(
          "lift, then draw the horizontal, turn down and hook left",
          compactPath("95 560 855 560 852 213 822 10 780 -30 680 -30 600 -15"),
        ),
        simpleStroke(
          "lift, then draw the dot inside",
          compactPath("534 370 627 269 668 206"),
        ),
      ],
      source: chineseCharacterSource("为"),
    },
  ],
  // 因: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 因. Medians 1–2 open the enclosure as 国 does: its left side, then a
  // horizontal that turns down the right side without lifting; medians 3–5 draw
  // 大 inside as 大 is written alone; median 6 closes the enclosure along the
  // foot. The order, the directions, the turn and all five lifts are the
  // source's.
  [
    "chinese:因",
    {
      script: "chinese",
      glyph: "因",
      strokes: [
        simpleStroke(
          "draw the frame's left side",
          compactPath("95 784 127 672 116 20 129 -65"),
        ),
        simpleStroke(
          "lift, then draw the frame's top and turn down its right side",
          compactPath(
            "111 787 242 759 797 769 832 761 873 732 884 711 882 51 864 -3 749 23",
          ),
        ),
        simpleStroke(
          "lift, then draw 大's horizontal",
          compactPath("223 498 491 479 770 498"),
        ),
        simpleStroke(
          "lift, then draw 大's left-falling stroke",
          compactPath("486 656 510 572 468 330 395 215 309 142 247 109"),
        ),
        simpleStroke(
          "lift, then draw 大's right-falling stroke",
          compactPath("462 417 669 224 740 96"),
        ),
        simpleStroke(
          "lift, then close the frame with a horizontal",
          compactPath("94 -14 300 6 676 9 903 -8"),
        ),
      ],
      source: chineseCharacterSource("因"),
    },
  ],
  // 时: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 时. Medians 1–4 draw 日 on the left as 日 is written alone. Medians
  // 5–7 draw 寸 on the right as 对 ends: a horizontal, a vertical that hooks to
  // the left at its foot without lifting, then a dot. The order, the
  // directions, the turn, the hook and all six lifts are the source's.
  [
    "chinese:时",
    {
      script: "chinese",
      glyph: "时",
      strokes: [
        simpleStroke(
          "draw 日's left side",
          compactPath("94 722 122 652 125 604 114 175 123 37"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("96 728 289 718 342 698 355 614 348 116"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("95 396 220 437 380 442"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("97 119 196 140 381 156"),
        ),
        simpleStroke(
          "lift, then draw 寸's horizontal",
          compactPath("450 590 774 593 876 607 948 595"),
        ),
        simpleStroke(
          "lift, then draw 寸's vertical and hook left",
          compactPath("782 821 796 743 797 33 767 -36 683 -40 588 -13"),
        ),
        simpleStroke(
          "lift, then draw 寸's dot",
          compactPath("509 453 661 240"),
        ),
      ],
      source: chineseCharacterSource("时"),
    },
  ],
  // 洗: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 洗. Medians 1–3 draw the water piece 氵 as 汉 opens: two dots, then a
  // rising stroke. Medians 4–9 draw 先 on the right: a short left-falling
  // stroke, a short horizontal, the vertical through it, the long horizontal,
  // then 儿: a left-falling stroke and a vertical that bends right along the
  // foot and hooks up without lifting. The order, the directions, the hook and
  // all eight lifts are the source's.
  [
    "chinese:洗",
    {
      script: "chinese",
      glyph: "洗",
      strokes: [
        simpleStroke(
          "draw 氵's upper dot",
          compactPath("118 798 219 733 263 684"),
        ),
        simpleStroke(
          "lift, then draw 氵's middle dot",
          compactPath("73 531 190 459 220 420"),
        ),
        simpleStroke(
          "lift, then draw 氵's rising stroke",
          compactPath("133 -39 137 40 225 223 235 278"),
        ),
        simpleStroke(
          "lift, then draw 先's short left-falling stroke",
          compactPath("472 802 445 700 353 487"),
        ),
        simpleStroke(
          "lift, then draw 先's short horizontal",
          compactPath("394 622 567 638 635 624 703 642 897 628"),
        ),
        simpleStroke(
          "lift, then draw 先's vertical",
          compactPath("619 826 635 748 624 405 599 365"),
        ),
        simpleStroke(
          "lift, then draw 先's long horizontal",
          compactPath("320 382 474 382 503 365 779 391 921 364"),
        ),
        simpleStroke(
          "lift, then draw 儿's left-falling stroke",
          compactPath("500 409 504 281 516 242 487 164 436 64 375 4 299 -46"),
        ),
        simpleStroke(
          "lift, then draw 儿's vertical, bend right along the foot and hook up",
          compactPath(
            "611 454 718 339 705 159 723 -21 740 -36 831 -39 910 -20 933 1 936 118",
          ),
        ),
      ],
      source: chineseCharacterSource("洗"),
    },
  ],
  // 菜: Hanzi Writer Data's eleven ordered medians, fitted to the vendored Noto
  // Sans SC 菜. in three pieces. Medians 1–3 draw the grass piece 艹: a
  // horizontal, then two short verticals through it. Medians 4–7 draw 爫 under
  // it: a flat left-falling stroke, two dots and a short left-falling stroke.
  // Medians 8–11 draw 木 at the foot: a horizontal, a vertical, then the left-
  // falling and right-falling strokes. The order, the directions and all ten
  // lifts are the source's.
  [
    "chinese:菜",
    {
      script: "chinese",
      glyph: "菜",
      strokes: [
        simpleStroke(
          "draw 艹's horizontal",
          compactPath("75 740 172 728 249 738 326 725 702 742 928 720"),
        ),
        simpleStroke(
          "lift, then draw 艹's left vertical",
          compactPath("253 754 311 724 328 659 349 639"),
        ),
        simpleStroke(
          "lift, then draw 艹's right vertical",
          compactPath("648 826 679 781 653 722 664 700 639 641"),
        ),
        simpleStroke(
          "lift, then draw 爫's flat top stroke",
          compactPath("838 596 770 602 327 553 113 559"),
        ),
        simpleStroke(
          "lift, then draw 爫's left dot",
          compactPath("167 461 265 344"),
        ),
        simpleStroke(
          "lift, then draw 爫's middle dot",
          compactPath("436 483 519 393"),
        ),
        simpleStroke(
          "lift, then draw 爫's short left-falling stroke",
          compactPath("827 500 811 450 737 328"),
        ),
        simpleStroke(
          "lift, then draw 木's horizontal",
          compactPath("72 235 147 223 712 241 870 208"),
        ),
        simpleStroke(
          "lift, then draw 木's vertical",
          compactPath("468 328 501 307 488 18 503 -66"),
        ),
        simpleStroke(
          "lift, then draw 木's left-falling stroke",
          compactPath("525 306 345 116 211 38 67 -15"),
        ),
        simpleStroke(
          "lift, then draw 木's right-falling stroke",
          compactPath("472 293 495 248 707 73 868 7 927 -4"),
        ),
      ],
      source: chineseCharacterSource("菜"),
    },
  ],
  // 用: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 用. Medians 1–2 draw the frame as 月 draws it: a left side that
  // curves to the lower left, then a horizontal that turns down the right side
  // and hooks to the left without lifting; medians 3–4 draw the two horizontals
  // inside; median 5 draws the vertical down the middle last. The order, the
  // directions, the turn, the hook and all four lifts are the source's.
  [
    "chinese:用",
    {
      script: "chinese",
      glyph: "用",
      strokes: [
        simpleStroke(
          "draw the left side, curving to the lower left",
          compactPath("175 754 183 703 197 686 198 273 154 82 67 -45"),
        ),
        simpleStroke(
          "lift, then draw the top, turn down the right side and hook left",
          compactPath(
            "190 755 318 720 412 730 485 717 750 740 802 731 844 695 842 4 821 -40 654 -30",
          ),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal",
          compactPath("229 490 504 481 632 504 818 515"),
        ),
        simpleStroke(
          "lift, then draw the lower horizontal",
          compactPath("222 275 427 253 537 271 850 246"),
        ),
        simpleStroke(
          "lift, then draw the middle vertical",
          compactPath("478 683 517 628 505 -60"),
        ),
      ],
      source: chineseCharacterSource("用"),
    },
  ],
  // 本: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 本. Medians 1–4 draw 木: a horizontal, a vertical, then the left-
  // falling and right-falling strokes; median 5 draws a short horizontal across
  // the vertical near its foot. The order, the directions and all four lifts
  // are the source's.
  [
    "chinese:本",
    {
      script: "chinese",
      glyph: "本",
      strokes: [
        simpleStroke(
          "draw the horizontal",
          compactPath("78 611 256 574 329 586 402 574 548 597 703 599 876 571"),
        ),
        simpleStroke(
          "lift, then draw the vertical",
          compactPath("475 825 491 747 488 -66"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke",
          compactPath("433 613 380 480 312 357 199 218 69 125"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("557 612 634 440 763 248 933 133"),
        ),
        simpleStroke(
          "lift, then draw the short horizontal across the vertical",
          compactPath("240 154 353 138 524 163 746 131"),
        ),
      ],
      source: chineseCharacterSource("本"),
    },
  ],
  // 站: Hanzi Writer Data's ten ordered medians, fitted to the vendored Noto
  // Sans SC 站. Medians 1–5 draw 立 on the left: a dot, a horizontal, a short
  // stroke on each side, then a stroke rising to the right along the foot.
  // Medians 6–10 draw 占 on the right as 点 opens: a vertical, a short horizontal
  // to its right, then 口 in the order the standalone character uses. The order,
  // the directions, the turn and all nine lifts are the source's.
  [
    "chinese:站",
    {
      script: "chinese",
      glyph: "站",
      strokes: [
        simpleStroke(
          "draw 立's dot",
          compactPath("203 806 235 783 283 689"),
        ),
        simpleStroke(
          "lift, then draw 立's horizontal",
          compactPath("70 613 298 609 431 622"),
        ),
        simpleStroke(
          "lift, then draw 立's left short stroke",
          compactPath("123 489 192 191"),
        ),
        simpleStroke(
          "lift, then draw 立's right short stroke",
          compactPath("342 528 355 417 317 235 262 85"),
        ),
        simpleStroke(
          "lift, then draw 立's rising bottom stroke",
          compactPath("69 73 140 70 304 132 329 128 422 155"),
        ),
        simpleStroke(
          "lift, then draw 占's vertical",
          compactPath("643 823 679 742 669 328 648 307"),
        ),
        simpleStroke(
          "lift, then draw 占's short horizontal",
          compactPath("642 559 707 587 942 602"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("478 337 509 146 520 0 510 -65"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath(
            "479 333 565 314 804 330 843 323 882 284 887 243 855 -26",
          ),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("479 24 578 11 592 -1 739 11 902 -11"),
        ),
      ],
      source: chineseCharacterSource("站"),
    },
  ],
  // 试: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 试. Medians 1–2 draw the speech piece 讠 as 谢 opens. Medians 3–8 draw
  // 式 on the right: the long horizontal, 工 under it (a horizontal, a vertical
  // and a rising stroke), the long stroke that slants down to the right and
  // hooks up without lifting, then the dot at the upper right last. The order,
  // the directions, the turns, the flick, the hook and all seven lifts are the
  // source's.
  [
    "chinese:试",
    {
      script: "chinese",
      glyph: "试",
      strokes: [
        simpleStroke(
          "draw 讠's dot",
          compactPath("158 793 289 668"),
        ),
        simpleStroke(
          "lift, then draw 讠's short horizontal, turn down and flick up to the right",
          compactPath("72 490 224 490 224 120 190 -25 260 52 363 124"),
        ),
        simpleStroke(
          "lift, then draw 式's long horizontal",
          compactPath("366 600 931 600"),
        ),
        simpleStroke(
          "lift, then draw 工's top horizontal",
          compactPath("391 374 530 370 632 386"),
        ),
        simpleStroke(
          "lift, then draw 工's vertical",
          compactPath("517 382 517 80"),
        ),
        simpleStroke(
          "lift, then draw 工's rising stroke",
          compactPath("380 38 438 39 528 91 567 84 621 96 657 119"),
        ),
        simpleStroke(
          "lift, then draw the long slanting stroke and hook up",
          compactPath(
            "680 797 704 780 718 404 734 260 781 73 839 -8 896 -35 916 23 930 136",
          ),
        ),
        simpleStroke(
          "lift, then draw the dot at the upper right",
          compactPath("819 804 899 681"),
        ),
      ],
      source: chineseCharacterSource("试"),
    },
  ],
  // 网: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 网. Medians 1–2 draw the frame: its left side, then a horizontal
  // that turns down the right side and hooks to the left without lifting;
  // medians 3–6 draw the two small crosses inside, left then right, each a
  // left-falling stroke then a dot. The order, the directions, the turn, the
  // hook and all five lifts are the source's.
  [
    "chinese:网",
    {
      script: "chinese",
      glyph: "网",
      strokes: [
        simpleStroke(
          "draw the frame's left side",
          compactPath("102 765 128 694 132 635 131 164 114 -68"),
        ),
        simpleStroke(
          "lift, then draw the frame's top, turn down and hook left",
          compactPath(
            "108 767 272 738 622 737 777 750 821 742 869 702 877 81 867 5 851 -25 832 -32 757 -40 687 -23",
          ),
        ),
        simpleStroke(
          "lift, then draw the left cross's left-falling stroke",
          compactPath("423 623 430 553 371 331 291 190 198 88"),
        ),
        simpleStroke(
          "lift, then draw the left cross's dot",
          compactPath("223 547 361 373 471 189"),
        ),
        simpleStroke(
          "lift, then draw the right cross's left-falling stroke",
          compactPath("702 622 712 589 654 336 552 144 483 77"),
        ),
        simpleStroke(
          "lift, then draw the right cross's dot",
          compactPath("509 538 587 451 726 249 751 204 758 152"),
        ),
      ],
      source: chineseCharacterSource("网"),
    },
  ],
  // 外: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 外. Medians 1–3 draw 夕 on the left as 多 opens: a left-falling
  // stroke, a horizontal that turns to fall to the lower left without lifting,
  // then a dot. Medians 4–5 draw 卜 on the right: a vertical, then a dot falling
  // to the lower right. The order, the directions, the turn and all four lifts
  // are the source's.
  [
    "chinese:外",
    {
      script: "chinese",
      glyph: "外",
      strokes: [
        simpleStroke(
          "draw 夕's left-falling stroke",
          compactPath("265 820 240 700 200 600 150 500 71 394"),
        ),
        simpleStroke(
          "lift, then draw 夕's horizontal and turn down to the left",
          compactPath(
            "188 634 423 649 444 643 470 560 407 371 346 286 334 234 298 166 204 57 67 -17",
          ),
        ),
        simpleStroke(
          "lift, then draw 夕's dot",
          compactPath("194 410 396 269"),
        ),
        simpleStroke(
          "lift, then draw 卜's vertical",
          compactPath("633 827 646 752 639 -65"),
        ),
        simpleStroke(
          "lift, then draw 卜's dot",
          compactPath("624 529 657 504 746 473 933 300"),
        ),
      ],
      source: chineseCharacterSource("外"),
    },
  ],
  // 边: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 边. in two pieces, the walking piece written last. Medians 1–2 draw
  // 力 as 男 ends: a horizontal that turns down and hooks to the left without
  // lifting, then a long left-falling stroke through it. Medians 3–5 draw 辶 as
  // 这 closes: a dot, a stroke that runs right, turns down, turns again and
  // falls to the left without lifting, then the long flat right-falling stroke
  // along the foot. The order, the directions, the turns, the hook and all four
  // lifts are the source's.
  [
    "chinese:边",
    {
      script: "chinese",
      glyph: "边",
      strokes: [
        simpleStroke(
          "draw 力's horizontal, turn down and hook left",
          compactPath(
            "366 630 879 630 877 450 849 200 820 145 792 132 656 127",
          ),
        ),
        simpleStroke(
          "lift, then draw 力's long left-falling stroke",
          compactPath("591 811 580 570 537 360 495 270 368 107"),
        ),
        simpleStroke(
          "lift, then draw 辶's dot",
          compactPath("112 786 225 702 261 650"),
        ),
        simpleStroke(
          "lift, then draw 辶's short horizontal, turn down, turn again and fall left",
          compactPath("66 465 211 465 211 150 190 90 150 45"),
        ),
        simpleStroke(
          "lift, then draw 辶's long sweep along the foot",
          compactPath(
            "84 33 216 83 242 78 328 20 436 -20 688 -24 756 -40 832 -37 948 -4",
          ),
        ),
      ],
      source: chineseCharacterSource("边"),
    },
  ],
  // 出: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 出. Medians 1–2 draw the upper half: a vertical that turns right
  // along the middle without lifting, then its right side; median 3 draws the
  // long vertical down the middle; medians 4–5 draw the lower half the same
  // way, its right side last. The order, the directions, the two turns and all
  // four lifts are the source's.
  [
    "chinese:出",
    {
      script: "chinese",
      glyph: "出",
      strokes: [
        simpleStroke(
          "draw the upper left side and turn right along the middle",
          compactPath("188 729 188 440 827 440"),
        ),
        simpleStroke(
          "lift, then draw the upper right vertical",
          compactPath("814 729 814 430"),
        ),
        simpleStroke(
          "lift, then draw the long middle vertical",
          compactPath("478 824 506 711 487 129 469 -8"),
        ),
        simpleStroke(
          "lift, then draw the lower left side and turn along the foot",
          compactPath(
            "115 263 152 223 152 111 170 13 396 16 511 36 550 18 765 26 879 7",
          ),
        ),
        simpleStroke(
          "lift, then draw the lower right vertical",
          compactPath("832 327 860 -68"),
        ),
      ],
      source: chineseCharacterSource("出"),
    },
  ],
  // 心: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 心. Median 1 draws the left dot; median 2 draws a curve that runs
  // along the foot and hooks up without lifting; medians 3–4 draw the middle
  // and right dots. These are the last four strokes of 想, in the same order.
  // The order, the directions, the turn, the hook and all three lifts are the
  // source's.
  [
    "chinese:心",
    {
      script: "chinese",
      glyph: "心",
      strokes: [
        simpleStroke(
          "draw 心's left dot",
          compactPath("160 466 153 368 100 102"),
        ),
        simpleStroke(
          "lift, then draw 心's curve along the foot and hook up",
          compactPath(
            "334 536 334 80 345 15 420 -33 690 -18 730 32 745 140 739 177",
          ),
        ),
        simpleStroke(
          "lift, then draw 心's middle dot",
          compactPath("386 781 617 552"),
        ),
        simpleStroke(
          "lift, then draw 心's right dot",
          compactPath("803 485 893 253 920 128"),
        ),
      ],
      source: chineseCharacterSource("心"),
    },
  ],
  // 包: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 包. Medians 1–2 draw 勹: a left-falling stroke, then a horizontal
  // that turns down the right side and hooks to the left without lifting.
  // Medians 3–5 draw 巳 inside: a horizontal that turns down without lifting, a
  // horizontal, then a vertical that bends right along the foot and hooks up
  // without lifting, as 也 ends. The order, the directions, the turns, the two
  // hooks and all four lifts are the source's.
  [
    "chinese:包",
    {
      script: "chinese",
      glyph: "包",
      strokes: [
        simpleStroke(
          "draw 勹's left-falling stroke",
          compactPath("314 827 312 807 332 790 285 718 279 684 213 606 64 489"),
        ),
        simpleStroke(
          "lift, then draw 勹's top, turn down and hook left",
          compactPath(
            "231 667 834 675 827 425 804 240 770 195 713 173 666 173",
          ),
        ),
        simpleStroke(
          "lift, then draw 巳's top and turn down",
          compactPath("232 495 568 495 568 270"),
        ),
        simpleStroke(
          "lift, then draw 巳's horizontal",
          compactPath("232 263 568 263"),
        ),
        simpleStroke(
          "lift, then draw 巳's last stroke down, along the foot and hook up",
          compactPath(
            "232 500 232 60 250 0 358 -27 800 -23 880 0 905 40 911 104",
          ),
        ),
      ],
      source: chineseCharacterSource("包"),
    },
  ],
  // 以: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 以. Medians 1–2 draw the left piece: a vertical that flicks up to
  // the right at its foot without lifting, then a dot beside it. Medians 3–4
  // draw the right piece: a long left-falling stroke, then a dot at its lower
  // right. The order, the directions, the flick and all three lifts are the
  // source's.
  [
    "chinese:以",
    {
      script: "chinese",
      glyph: "以",
      strokes: [
        simpleStroke(
          "draw the left vertical and flick up to the right",
          compactPath("199 747 199 180 180 100 150 60 230 120 458 242"),
        ),
        simpleStroke(
          "lift, then draw the dot",
          compactPath("411 714 501 612 560 514"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath(
            "774 782 799 696 776 519 690 251 644 169 539 53 485 9 384 -39",
          ),
        ),
        simpleStroke(
          "lift, then draw the last dot at the lower right",
          compactPath("677 257 704 207 869 46 909 -54"),
        ),
      ],
      source: chineseCharacterSource("以"),
    },
  ],
  // 些: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 些. Medians 1–6 draw 此 on top: the middle vertical, a short
  // horizontal to its right, a shorter vertical on the left, a stroke that
  // rises to the right along the foot, then 匕, a short left-falling stroke and
  // a vertical that bends right and hooks up without lifting. Medians 7–8 draw
  // 二 under it as 二 is written alone. The order, the directions, the hook and
  // all seven lifts are the source's.
  [
    "chinese:些",
    {
      script: "chinese",
      glyph: "些",
      strokes: [
        simpleStroke(
          "draw the middle vertical",
          compactPath("278 826 294 744 312 726 317 381 298 348"),
        ),
        simpleStroke(
          "lift, then draw the short horizontal to its right",
          compactPath("280 592 321 609 334 627 479 637"),
        ),
        simpleStroke(
          "lift, then draw the short left vertical",
          compactPath("118 701 148 530 143 460 159 357 143 331"),
        ),
        simpleStroke(
          "lift, then draw the rising foot stroke",
          compactPath("60 354 137 340 396 378 488 403"),
        ),
        simpleStroke(
          "lift, then draw 匕's short left-falling stroke",
          compactPath("848 722 793 682 801 664 669 620 565 564"),
        ),
        simpleStroke(
          "lift, then draw 匕's vertical, bend right and hook up",
          compactPath(
            "566 824 594 749 578 542 593 380 707 356 879 368 900 378 913 408 918 502",
          ),
        ),
        simpleStroke(
          "lift, then draw 二's upper horizontal",
          compactPath("200 217 406 189 588 209 779 198"),
        ),
        simpleStroke(
          "lift, then draw 二's long lower horizontal",
          compactPath("67 3 211 -21 283 -11 781 -8 922 -44"),
        ),
      ],
      source: chineseCharacterSource("些"),
    },
  ],
  // 音: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 音. Medians 1–5 draw 立 on top: a dot, a horizontal, two short
  // strokes leaning toward each other, then the long horizontal. Medians 6–9
  // draw 日 under it as 日 is written alone. The order, the directions, the turn
  // and all eight lifts are the source's.
  [
    "chinese:音",
    {
      script: "chinese",
      glyph: "音",
      strokes: [
        simpleStroke(
          "draw 立's dot",
          compactPath("463 827 502 766 585 692"),
        ),
        simpleStroke(
          "lift, then draw 立's horizontal",
          compactPath("153 733 327 705 849 717"),
        ),
        simpleStroke(
          "lift, then draw 立's left short stroke",
          compactPath("291 641 342 491"),
        ),
        simpleStroke(
          "lift, then draw 立's right short stroke",
          compactPath("720 641 655 499"),
        ),
        simpleStroke(
          "lift, then draw 立's long horizontal",
          compactPath("69 486 159 472 688 474 798 485 932 463"),
        ),
        simpleStroke(
          "lift, then draw 日's left side",
          compactPath("230 335 230 -62"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("230 325 778 325 778 -68"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("230 153 778 153"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("230 -13 778 -13"),
        ),
      ],
      source: chineseCharacterSource("音"),
    },
  ],
  // 吵: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 吵. Medians 1–3 draw 口 on the left in the order the standalone
  // character uses. Medians 4–7 draw 少 on the right as 少 is written alone: the
  // vertical, a short stroke on the left, a dot on the right, then the long
  // left-falling stroke. The order, the directions, the turn and all six lifts
  // are the source's.
  [
    "chinese:吵",
    {
      script: "chinese",
      glyph: "吵",
      strokes: [
        simpleStroke(
          "draw 口's left side",
          compactPath("106 719 106 106"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("106 715 299 715 299 240"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("98 224 309 224"),
        ),
        simpleStroke(
          "lift, then draw the middle vertical",
          compactPath("649 812 649 232"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke",
          compactPath("487 645 461 474 422 362"),
        ),
        simpleStroke(
          "lift, then draw the dot on the right",
          compactPath("800 658 931 420"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("841 330 805 240 740 150 650 70 500 0 372 -35"),
        ),
      ],
      source: chineseCharacterSource("吵"),
    },
  ],
  // 病: Hanzi Writer Data's ten ordered medians, fitted to the vendored Noto
  // Sans SC 病. Medians 1–5 draw the sickness piece 疒: a dot, a horizontal, a
  // long left-falling stroke, then a dot and a rising stroke on the left.
  // Medians 6–10 draw 丙 inside: a horizontal, the frame's left side, a
  // horizontal that turns down the right side and hooks to the left without
  // lifting, then the small 人, a left-falling stroke and a short right-falling
  // one. The order, the directions, the turn, the hook and all nine lifts are
  // the source's.
  [
    "chinese:病",
    {
      script: "chinese",
      glyph: "病",
      strokes: [
        simpleStroke(
          "draw 疒's top dot",
          compactPath("551 817 656 671"),
        ),
        simpleStroke(
          "lift, then draw 疒's horizontal",
          compactPath("269 715 426 681 944 699"),
        ),
        simpleStroke(
          "lift, then draw 疒's long left-falling stroke",
          compactPath("238 690 226 250 190 100 105 -40"),
        ),
        simpleStroke(
          "lift, then draw the dot on the left",
          compactPath("93 612 144 468"),
        ),
        simpleStroke(
          "lift, then draw the rising stroke on the left",
          compactPath("56 237 97 237 215 308 256 350"),
        ),
        simpleStroke(
          "lift, then draw 丙's horizontal",
          compactPath("362 555 617 519 686 533 937 542"),
        ),
        simpleStroke(
          "lift, then draw the frame's left side",
          compactPath("373 372 373 -57"),
        ),
        simpleStroke(
          "lift, then draw the frame's top, turn down and hook left",
          compactPath("373 365 885 365 881 -13 868 -38 751 -38"),
        ),
        simpleStroke(
          "lift, then draw the small 人's left-falling stroke",
          compactPath("598 497 635 387 616 265 569 172 531 136 452 92"),
        ),
        simpleStroke(
          "lift, then draw the small 人's right-falling stroke",
          compactPath("595 289 728 175 796 96"),
        ),
      ],
      source: chineseCharacterSource("病"),
    },
  ],
  // 春: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 春. Medians 1–3 draw three horizontals, the lowest longest, as 三 is
  // written; medians 4–5 draw the left-falling stroke through them and the
  // right-falling stroke from the crossing, as 人 spreads; medians 6–9 draw 日
  // under them as 日 is written alone. The order, the directions, the turn and
  // all eight lifts are the source's.
  [
    "chinese:春",
    {
      script: "chinese",
      glyph: "春",
      strokes: [
        simpleStroke(
          "draw the upper horizontal",
          compactPath("128 723 862 723"),
        ),
        simpleStroke(
          "lift, then draw the middle horizontal",
          compactPath("162 595 840 595"),
        ),
        simpleStroke(
          "lift, then draw the long lower horizontal",
          compactPath("74 445 918 445"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke through all three",
          compactPath("469 826 453 680 379 463 238 286 75 196"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("470 450 628 438 660 423 800 270 921 209"),
        ),
        simpleStroke(
          "lift, then draw 日's left side",
          compactPath("278 300 278 -68"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("278 303 725 303 725 -60"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("278 153 725 153"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("278 -5 725 -5"),
        ),
      ],
      source: chineseCharacterSource("春"),
    },
  ],
  // 千: Hanzi Writer Data's three ordered medians, fitted to the vendored Noto
  // Sans SC 千. Median 1 draws a short left-falling stroke across the top;
  // medians 2–3 draw 十 under it as 十 is written alone: the horizontal, then the
  // vertical through it. The order, the directions and all two lifts are the
  // source's.
  [
    "chinese:千",
    {
      script: "chinese",
      glyph: "千",
      strokes: [
        simpleStroke(
          "draw the short top stroke",
          compactPath("798 801 600 750 134 682"),
        ),
        simpleStroke(
          "lift, then draw the horizontal",
          compactPath("63 421 167 397 282 415 661 408 817 421 935 394"),
        ),
        simpleStroke(
          "lift, then draw the vertical through it",
          compactPath("448 742 467 707 487 699 501 643 492 -66"),
        ),
      ],
      source: chineseCharacterSource("千"),
    },
  ],
  // 扫: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 扫. Medians 1–3 draw the hand piece 扌 on the left as 打 opens: a
  // horizontal, a vertical that hooks to the left at its foot without lifting,
  // then a rising stroke. Medians 4–6 draw the right piece: a horizontal that
  // turns down the right side without lifting, a middle horizontal, then the
  // bottom horizontal. Noto's right side runs on below the bottom horizontal,
  // where the source's stops above it. The order, the directions, the hook, the
  // turn and all five lifts are the source's.
  [
    "chinese:扫",
    {
      script: "chinese",
      glyph: "扫",
      strokes: [
        simpleStroke(
          "draw 扌's horizontal",
          compactPath("72 615 374 615"),
        ),
        simpleStroke(
          "lift, then draw 扌's vertical and hook left",
          compactPath("210 822 260 740 246 34 223 -35 103 -36"),
        ),
        simpleStroke(
          "lift, then draw 扌's rising stroke",
          compactPath("70 280 379 369"),
        ),
        simpleStroke(
          "lift, then draw the top and turn down the right side",
          compactPath("441 705 867 705 867 -54"),
        ),
        simpleStroke(
          "lift, then draw the middle horizontal",
          compactPath("465 390 860 390"),
        ),
        simpleStroke(
          "lift, then draw the bottom horizontal",
          compactPath("435 30 860 30"),
        ),
      ],
      source: chineseCharacterSource("扫"),
    },
  ],
  // 重: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 重. Medians 1–2 draw the top as 千 opens: a short left-falling
  // stroke, then the long horizontal; medians 3–6 draw the box of 田 as 里 does:
  // its left side, the joined top and right side, the middle horizontal, the
  // closing bottom; median 7 draws the long vertical from the top stroke down
  // through the box; medians 8–9 draw the two horizontals of 土 on it, the lower
  // one longer. The order, the directions, the turn and all eight lifts are the
  // source's.
  [
    "chinese:重",
    {
      script: "chinese",
      glyph: "重",
      strokes: [
        simpleStroke(
          "draw the short top stroke",
          compactPath("745 772 595 788 498 750 468 764 316 759 197 736"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("70 635 157 623 456 634 494 623 806 639 926 617"),
        ),
        simpleStroke(
          "lift, then draw 田's left side",
          compactPath("195 512 195 253"),
        ),
        simpleStroke(
          "lift, then draw 田's top and turn down its right side",
          compactPath("195 513 809 505 809 258"),
        ),
        simpleStroke(
          "lift, then draw 田's middle horizontal",
          compactPath("195 383 809 383"),
        ),
        simpleStroke(
          "lift, then close 田 with a horizontal",
          compactPath("195 255 809 255"),
        ),
        simpleStroke(
          "lift, then draw the long vertical through the box",
          compactPath("460 783 504 637 495 74 468 -33"),
        ),
        simpleStroke(
          "lift, then draw the short lower horizontal",
          compactPath("148 135 864 135"),
        ),
        simpleStroke(
          "lift, then draw the long bottom horizontal",
          compactPath("93 -1 219 -22 517 -11 935 -23"),
        ),
      ],
      source: chineseCharacterSource("重"),
    },
  ],
  // 节: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 节. Medians 1–3 draw the grass piece 艹 as 菜 opens: a horizontal,
  // then two short verticals. Medians 4–5 draw 卩 under it: a horizontal that
  // turns down the right side and hooks to the left without lifting, then the
  // long vertical through it. The order, the directions, the turn, the hook and
  // all four lifts are the source's.
  [
    "chinese:节",
    {
      script: "chinese",
      glyph: "节",
      strokes: [
        simpleStroke(
          "draw 艹's horizontal",
          compactPath("69 693 156 681 267 689 304 678 843 698 931 676"),
        ),
        simpleStroke(
          "lift, then draw 艹's left vertical",
          compactPath("302 769 345 718 352 683 335 656 341 554"),
        ),
        simpleStroke(
          "lift, then draw 艹's right vertical",
          compactPath("650 825 686 772 664 700 666 592 653 554"),
        ),
        simpleStroke(
          "lift, then draw the horizontal, turn down and hook left",
          compactPath("118 450 810 450 805 140 770 100 690 90 628 100"),
        ),
        simpleStroke(
          "lift, then draw the long vertical",
          compactPath("399 456 399 -62"),
        ),
      ],
      source: chineseCharacterSource("节"),
    },
  ],
  // 每: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 每. Medians 1–2 draw a short left-falling stroke and a short
  // horizontal; median 3 draws the box's left side and turns right along its
  // foot without lifting; median 4 draws its top and turns down the right side
  // to hook to the left without lifting; medians 5–7 draw a dot, the long
  // horizontal that runs out past both sides, then a second dot. The order, the
  // directions, the turns, the hook and all six lifts are the source's.
  [
    "chinese:每",
    {
      script: "chinese",
      glyph: "每",
      strokes: [
        simpleStroke(
          "draw the short top stroke",
          compactPath("295 820 294 808 319 799 275 713 175 599 62 509"),
        ),
        simpleStroke(
          "lift, then draw the short horizontal",
          compactPath("264 707 868 701"),
        ),
        simpleStroke(
          "lift, then draw the left side and turn right along the foot",
          compactPath("252 545 190 95 899 87"),
        ),
        simpleStroke(
          "lift, then draw the top, turn down the right side and hook left",
          compactPath("255 537 786 537 768 140 739 -10 700 -45 583 -55"),
        ),
        simpleStroke(
          "lift, then draw the upper dot",
          compactPath("432 479 580 372"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("66 313 936 313"),
        ),
        simpleStroke(
          "lift, then draw the lower dot",
          compactPath("405 259 573 145"),
        ),
      ],
      source: chineseCharacterSource("每"),
    },
  ],
  // 半: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 半. Medians 1–2 draw two short strokes leaning toward each other;
  // medians 3–4 draw two horizontals, the lower one longer; median 5 draws the
  // long vertical last, from the top down through both. The order, the
  // directions and all four lifts are the source's.
  [
    "chinese:半",
    {
      script: "chinese",
      glyph: "半",
      strokes: [
        simpleStroke(
          "draw the left short stroke",
          compactPath("208 714 269 664 309 597"),
        ),
        simpleStroke(
          "lift, then draw the right short stroke",
          compactPath("810 789 690 590"),
        ),
        simpleStroke(
          "lift, then draw the upper horizontal",
          compactPath("138 480 868 480"),
        ),
        simpleStroke(
          "lift, then draw the long lower horizontal",
          compactPath("74 250 918 250"),
        ),
        simpleStroke(
          "lift, then draw the long vertical through both",
          compactPath("496 822 496 -54"),
        ),
      ],
      source: chineseCharacterSource("半"),
    },
  ],
  // 后: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 后. Medians 1–3 draw the frame: a short left-falling stroke across
  // the top, a long left-falling stroke down the left side, then the
  // horizontal. Medians 4–6 draw 口 inside it in the order the standalone
  // character uses. The order, the directions, the turn and all five lifts are
  // the source's.
  [
    "chinese:后",
    {
      script: "chinese",
      glyph: "后",
      strokes: [
        simpleStroke(
          "draw the short top stroke",
          compactPath("799 766 651 778 564 744 369 728 227 702"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("189 720 188 420 172 240 145 120 84 -40"),
        ),
        simpleStroke(
          "lift, then draw the horizontal",
          compactPath("214 536 400 517 679 520 828 537 939 512"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("349 320 349 -68"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("349 308 841 300 841 30"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("349 4 841 4"),
        ),
      ],
      source: chineseCharacterSource("后"),
    },
  ],
  // 月: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 月. Median 1 draws the left side, curving to the lower left; median
  // 2 draws a horizontal that turns down the right side and hooks to the left
  // without lifting; medians 3–4 draw the two horizontals inside. These are the
  // strokes of 月 in 明, 期 and 前, in the same order. The order, the directions,
  // the turn, the hook and all three lifts are the source's.
  [
    "chinese:月",
    {
      script: "chinese",
      glyph: "月",
      strokes: [
        simpleStroke(
          "draw 月's left side, curving to the lower left",
          compactPath("245 761 239 340 218 220 153 60 85 -40"),
        ),
        simpleStroke(
          "lift, then draw 月's top, turn down its right side and hook left",
          compactPath(
            "288 774 426 749 444 736 713 749 755 733 775 711 778 30 753 -16 732 -28 649 -36 548 -15",
          ),
        ),
        simpleStroke(
          "lift, then draw 月's upper horizontal",
          compactPath("288 489 583 516 793 510"),
        ),
        simpleStroke(
          "lift, then draw 月's lower horizontal",
          compactPath("331 244 487 275 793 272"),
        ),
      ],
      source: chineseCharacterSource("月"),
    },
  ],
  // 正: Hanzi Writer Data's five ordered medians, fitted to the vendored Noto
  // Sans SC 正. Median 1 draws the top horizontal; medians 2–5 draw 止 under it:
  // the middle vertical, a short horizontal to its right, a short vertical on
  // the left, then the long horizontal along the foot. The order, the
  // directions and all four lifts are the source's.
  [
    "chinese:正",
    {
      script: "chinese",
      glyph: "正",
      strokes: [
        simpleStroke(
          "draw the top horizontal",
          compactPath("173 751 345 719 422 726 527 716 892 746"),
        ),
        simpleStroke(
          "lift, then draw the middle vertical",
          compactPath("485 752 524 576 520 104 490 -22"),
        ),
        simpleStroke(
          "lift, then draw the short horizontal to its right",
          compactPath("500 400 552 396 563 381 612 391 862 389"),
        ),
        simpleStroke(
          "lift, then draw the short left vertical",
          compactPath("198 463 230 392 233 78 251 -22"),
        ),
        simpleStroke(
          "lift, then draw the long bottom horizontal",
          compactPath("62 16 130 -1 528 13 567 0 803 13 935 -15"),
        ),
      ],
      source: chineseCharacterSource("正"),
    },
  ],
  // 过: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 过. in two pieces, the walking piece written last. Medians 1–3 draw
  // 寸 as 对 ends: a horizontal, a vertical that hooks to the left at its foot
  // without lifting, then a dot. Medians 4–6 draw 辶 as 这 closes: a dot, a
  // stroke that runs right, turns down, turns again and falls to the left
  // without lifting, then the long flat right-falling stroke along the foot.
  // The order, the directions, the turns, the hook and all five lifts are the
  // source's.
  [
    "chinese:过",
    {
      script: "chinese",
      glyph: "过",
      strokes: [
        simpleStroke(
          "draw 寸's horizontal",
          compactPath("345 626 441 612 623 623 732 612 805 626 919 613"),
        ),
        simpleStroke(
          "lift, then draw 寸's vertical and hook left",
          compactPath("730 813 757 734 747 178 730 142 559 140"),
        ),
        simpleStroke(
          "lift, then draw 寸's dot",
          compactPath("407 475 499 389 551 310"),
        ),
        simpleStroke(
          "lift, then draw 辶's dot",
          compactPath("108 775 213 698 256 639"),
        ),
        simpleStroke(
          "lift, then draw 辶's short horizontal, turn down, turn again and fall left",
          compactPath("63 429 230 432 232 376 216 328 237 162 221 142 215 86"),
        ),
        simpleStroke(
          "lift, then draw 辶's long sweep along the foot",
          compactPath(
            "76 34 218 103 267 91 315 52 383 26 519 -8 836 -9 941 26",
          ),
        ),
      ],
      source: chineseCharacterSource("过"),
    },
  ],
  // 到: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 到. Medians 1–6 draw 至 on the left: a horizontal, a stroke that
  // falls to the left and turns right without lifting, a dot, then 土 (a
  // horizontal, the vertical through it, and a bottom bar that rises to the
  // right). Medians 7–8 draw the knife piece 刂 on the right as 前 ends: a short
  // vertical, then a long one that hooks to the left at its foot without
  // lifting. The order, the directions, the turn, the hook and all seven lifts
  // are the source's.
  [
    "chinese:到",
    {
      script: "chinese",
      glyph: "到",
      strokes: [
        simpleStroke(
          "draw the top horizontal",
          compactPath("85 750 270 728 340 742 570 743"),
        ),
        simpleStroke(
          "lift, then draw the stroke down to the left and turn right",
          compactPath(
            "222 759 242 724 276 699 178 547 168 523 174 490 255 488 533 520",
          ),
        ),
        simpleStroke(
          "lift, then draw the dot",
          compactPath("410 643 527 507 545 448"),
        ),
        simpleStroke(
          "lift, then draw 土's horizontal",
          compactPath("111 290 242 280 551 288"),
        ),
        simpleStroke(
          "lift, then draw 土's vertical",
          compactPath("304 417 336 395 332 165 301 24"),
        ),
        simpleStroke(
          "lift, then draw 土's rising bottom bar",
          compactPath("80 23 175 14 362 69 399 66 564 104"),
        ),
        simpleStroke(
          "lift, then draw 刂's short vertical",
          compactPath("654 673 674 546 668 281 685 160"),
        ),
        simpleStroke(
          "lift, then draw 刂's long vertical and hook left",
          compactPath("859 810 872 773 864 365 874 41 850 -17 751 -20 709 0"),
        ),
      ],
      source: chineseCharacterSource("到"),
    },
  ],
  // 得: Hanzi Writer Data's eleven ordered medians, fitted to the vendored Noto
  // Sans SC 得. Medians 1–3 draw 彳 on the left as 很 opens: two left-falling
  // strokes, the upper one short, then a vertical. Medians 4–11 draw the right
  // side: 日 as 日 is written alone, a horizontal under it, then 寸 as 对 ends (a
  // horizontal, a vertical that hooks to the left at its foot without lifting,
  // and a dot). The order, the directions, the turn, the hook and all ten lifts
  // are the source's.
  [
    "chinese:得",
    {
      script: "chinese",
      glyph: "得",
      strokes: [
        simpleStroke(
          "draw 彳's short upper left-falling stroke",
          compactPath("262 819 258 808 276 798 253 765 78 601"),
        ),
        simpleStroke(
          "lift, then draw 彳's longer lower left-falling stroke",
          compactPath("280 584 201 460 100 360 56 338"),
        ),
        simpleStroke(
          "lift, then draw 彳's vertical",
          compactPath("215 470 215 -65"),
        ),
        simpleStroke(
          "lift, then draw 日's left side",
          compactPath("445 784 445 501"),
        ),
        simpleStroke(
          "lift, then draw 日's top and turn down its right side",
          compactPath("445 780 850 780 850 498"),
        ),
        simpleStroke(
          "lift, then draw 日's middle horizontal",
          compactPath("445 640 850 640"),
        ),
        simpleStroke(
          "lift, then close 日 with a horizontal",
          compactPath("445 500 850 500"),
        ),
        simpleStroke(
          "lift, then draw the horizontal under 日",
          compactPath("368 372 906 372"),
        ),
        simpleStroke(
          "lift, then draw 寸's horizontal",
          compactPath("344 228 932 228"),
        ),
        simpleStroke(
          "lift, then draw 寸's vertical and hook left",
          compactPath(
            "671 394 751 360 754 -15 736 -52 695 -41 645 -48 611 -35",
          ),
        ),
        simpleStroke(
          "lift, then draw 寸's dot",
          compactPath("452 151 528 78 554 30"),
        ),
      ],
      source: chineseCharacterSource("得"),
    },
  ],
  // 比: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 比. Medians 1–2 draw the left 匕: a short horizontal, then a vertical
  // that turns up to the right at its foot without lifting. Medians 3–4 draw
  // the right 匕 as 能 draws it: a left-falling stroke, then a vertical that
  // bends right along the foot and hooks up without lifting. The order, the
  // directions, the turns, the hook and all three lifts are the source's.
  [
    "chinese:比",
    {
      script: "chinese",
      glyph: "比",
      strokes: [
        simpleStroke(
          "draw the left 匕's short horizontal",
          compactPath("143 447 202 481 364 501 441 496"),
        ),
        simpleStroke(
          "lift, then draw the left 匕's vertical and turn up to the right",
          compactPath("158 807 158 60 135 -35 250 30 432 89"),
        ),
        simpleStroke(
          "lift, then draw the right 匕's left-falling stroke",
          compactPath("838 614 806 545 575 393 546 361"),
        ),
        simpleStroke(
          "lift, then draw the right 匕's vertical, bend right and hook up",
          compactPath(
            "556 821 569 781 563 742 577 719 558 192 576 8 600 -10 680 -30 791 -30 863 -5 892 23 896 217",
          ),
        ),
      ],
      source: chineseCharacterSource("比"),
    },
  ],
  // 还: Hanzi Writer Data's seven ordered medians, fitted to the vendored Noto
  // Sans SC 还. in two pieces, the walking piece written last. Medians 1–4 draw
  // 不 as 不 is written alone: a horizontal, a left-falling stroke, a vertical,
  // then a dot. Medians 5–7 draw 辶 as 这 closes: a dot, a stroke that runs
  // right, turns down, turns again and falls to the left without lifting, then
  // the long flat right-falling stroke along the foot. The order, the
  // directions, the turns and all six lifts are the source's.
  [
    "chinese:还",
    {
      script: "chinese",
      glyph: "还",
      strokes: [
        simpleStroke(
          "draw 不's horizontal",
          compactPath("340 742 459 720 600 731 671 720 777 741 913 726"),
        ),
        simpleStroke(
          "lift, then draw 不's left-falling stroke",
          compactPath(
            "602 627 602 607 624 585 585 499 543 490 502 436 326 285",
          ),
        ),
        simpleStroke(
          "lift, then draw 不's vertical",
          compactPath("558 556 607 451 605 76"),
        ),
        simpleStroke(
          "lift, then draw 不's dot",
          compactPath("698 490 862 359 902 287"),
        ),
        simpleStroke(
          "lift, then draw 辶's dot",
          compactPath("110 788 218 715 263 653"),
        ),
        simpleStroke(
          "lift, then draw 辶's short horizontal, turn down, turn again and fall left",
          compactPath(
            "54 461 143 458 218 473 217 371 202 338 227 197 216 145 199 133 156 34",
          ),
        ),
        simpleStroke(
          "lift, then draw 辶's long sweep along the foot",
          compactPath("58 11 213 81 310 26 416 -13 538 -28 817 -39 947 0"),
        ),
      ],
      source: chineseCharacterSource("还"),
    },
  ],
  // 从: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 从. Medians 1–2 draw the left 人: a left-falling stroke, then a short
  // right-falling stroke that stops at its middle. Medians 3–4 draw the right 人
  // as 人 is written alone: a left-falling stroke, then the right-falling
  // stroke. The order, the directions and all three lifts are the source's.
  [
    "chinese:从",
    {
      script: "chinese",
      glyph: "从",
      strokes: [
        simpleStroke(
          "draw the left 人's left-falling stroke",
          compactPath("271 729 305 656 277 401 235 224 164 51 70 -26"),
        ),
        simpleStroke(
          "lift, then draw the left 人's short right-falling stroke",
          compactPath("277 498 359 373 438 280 468 195"),
        ),
        simpleStroke(
          "lift, then draw the right 人's left-falling stroke",
          compactPath("669 801 661 603 596 273 520 88 458 10 400 -26"),
        ),
        simpleStroke(
          "lift, then draw the right 人's right-falling stroke",
          compactPath("650 497 693 320 780 125 842 31 936 -7"),
        ),
      ],
      source: chineseCharacterSource("从"),
    },
  ],
  // 元: Hanzi Writer Data's four ordered medians, fitted to the vendored Noto
  // Sans SC 元. Medians 1–2 draw 二 as 二 is written alone, the lower horizontal
  // longer. Medians 3–4 draw 儿 under it as 儿 is written alone: a left-falling
  // stroke, then a vertical that bends right along the foot and hooks up
  // without lifting. The order, the directions, the hook and all three lifts
  // are the source's.
  [
    "chinese:元",
    {
      script: "chinese",
      glyph: "元",
      strokes: [
        simpleStroke(
          "draw the short upper horizontal",
          compactPath("183 725 487 716 770 747"),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("72 451 329 432 551 450 888 439"),
        ),
        simpleStroke(
          "lift, then draw 儿's left-falling stroke",
          compactPath("327 398 360 341 335 252 268 118 211 46 90 -45"),
        ),
        simpleStroke(
          "lift, then draw 儿's vertical, bend right along the foot and hook up",
          compactPath(
            "555 465 610 421 625 389 607 235 612 34 628 -20 731 -35 840 -25 909 1 919 52 906 162",
          ),
        ),
      ],
      source: chineseCharacterSource("元"),
    },
  ],
  // 房: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 房. Medians 1–4 draw 户 on top: a dot, a horizontal that turns down
  // at its right end without lifting, the horizontal that closes the box, then
  // a long left-falling stroke down the left side. Medians 5–8 draw 方 under it:
  // a dot, a long horizontal, a horizontal under it that turns down and hooks
  // to the left without lifting, then a left-falling stroke through the middle.
  // The order, the directions, the turns, the hook and all seven lifts are the
  // source's.
  [
    "chinese:房",
    {
      script: "chinese",
      glyph: "房",
      strokes: [
        simpleStroke(
          "draw 户's dot",
          compactPath("468 811 504 790 565 688"),
        ),
        simpleStroke(
          "lift, then draw 户's horizontal and turn down",
          compactPath("158 707 847 707 847 532"),
        ),
        simpleStroke(
          "lift, then close 户 with a horizontal",
          compactPath("158 535 862 535"),
        ),
        simpleStroke(
          "lift, then draw 户's long left-falling stroke",
          compactPath("173 715 160 250 139 130 82 -48"),
        ),
        simpleStroke(
          "lift, then draw 方's dot",
          compactPath("528 475 572 439 596 398"),
        ),
        simpleStroke(
          "lift, then draw 方's horizontal",
          compactPath("255 338 479 330 589 343 809 344 905 333"),
        ),
        simpleStroke(
          "lift, then draw 方's short horizontal, turn down and hook left",
          compactPath("436 183 630 191 827 178 780 -15 708 -38 616 -43"),
        ),
        simpleStroke(
          "lift, then draw 方's left-falling stroke",
          compactPath("468 295 429 130 393 70 264 -42"),
        ),
      ],
      source: chineseCharacterSource("房"),
    },
  ],
  // 米: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 米. Medians 1–2 draw the two short strokes above, the left one
  // first; medians 3–4 draw the horizontal and the vertical down through it;
  // medians 5–6 draw a left-falling and then a right-falling stroke from the
  // crossing. The order, the directions and all five lifts are the source's.
  [
    "chinese:米",
    {
      script: "chinese",
      glyph: "米",
      strokes: [
        simpleStroke(
          "draw the upper-left dot",
          compactPath("179 687 241 639 297 553"),
        ),
        simpleStroke(
          "lift, then draw the upper-right short stroke",
          compactPath("837 765 784 650 698 540"),
        ),
        simpleStroke(
          "lift, then draw the horizontal",
          compactPath("73 433 182 410 489 432 568 420 726 427 926 393"),
        ),
        simpleStroke(
          "lift, then draw the vertical",
          compactPath("474 824 496 744 486 -66"),
        ),
        simpleStroke(
          "lift, then draw the left-falling stroke",
          compactPath("524 484 300 190 178 81 67 17"),
        ),
        simpleStroke(
          "lift, then draw the right-falling stroke",
          compactPath("470 493 522 397 722 165 934 26"),
        ),
      ],
      source: chineseCharacterSource("米"),
    },
  ],
  // 衣: Hanzi Writer Data's six ordered medians, fitted to the vendored Noto
  // Sans SC 衣. Medians 1–2 draw a dot and a horizontal on top; medians 3–6 draw
  // the body: a long left-falling stroke, a short vertical inside it that
  // flicks up to the right without lifting, a short left-falling stroke on the
  // right, then a long right-falling stroke. The order, the directions, the
  // turn and all five lifts are the source's.
  [
    "chinese:衣",
    {
      script: "chinese",
      glyph: "衣",
      strokes: [
        simpleStroke(
          "draw the top dot",
          compactPath("449 814 520 754 546 704"),
        ),
        simpleStroke(
          "lift, then draw the horizontal",
          compactPath("76 654 242 630 408 632 483 608 591 635 896 646"),
        ),
        simpleStroke(
          "lift, then draw the long left-falling stroke",
          compactPath("495 615 257 380 149 320 93 273"),
        ),
        simpleStroke(
          "lift, then draw the short vertical and flick up to the right",
          compactPath("297 380 296 60 255 -35 590 92"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke",
          compactPath("819 462 782 410 785 398 702 322 635 234"),
        ),
        simpleStroke(
          "lift, then draw the long right-falling stroke",
          compactPath("540 448 567 426 614 301 683 188 809 57 841 51 933 -1"),
        ),
      ],
      source: chineseCharacterSource("衣"),
    },
  ],
  // 茶: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 茶. in three pieces. Medians 1–3 draw 艹 on top as 菜 opens: a
  // horizontal, then two short verticals through it. Medians 4–5 draw 人 under
  // it: a left-falling, then a right-falling stroke. Medians 6–9 draw the foot:
  // a horizontal, a vertical that hooks to the left at its foot without
  // lifting, then a short left-falling stroke and a dot on either side of it.
  // The order, the directions, the hook and all eight lifts are the source's.
  [
    "chinese:茶",
    {
      script: "chinese",
      glyph: "茶",
      strokes: [
        simpleStroke(
          "draw 艹's horizontal",
          compactPath("82 700 920 700"),
        ),
        simpleStroke(
          "lift, then draw 艹's left vertical",
          compactPath("311 822 311 596"),
        ),
        simpleStroke(
          "lift, then draw 艹's right vertical",
          compactPath("684 822 684 596"),
        ),
        simpleStroke(
          "lift, then draw 人's left-falling stroke",
          compactPath("498 621 492 568 438 541 291 430 100 335"),
        ),
        simpleStroke(
          "lift, then draw 人's right-falling stroke",
          compactPath("500 576 728 430 920 340"),
        ),
        simpleStroke(
          "lift, then draw the horizontal under it",
          compactPath("188 287 319 269 447 271 482 254 586 276 777 269"),
        ),
        simpleStroke(
          "lift, then draw the vertical and hook left",
          compactPath("476 424 496 356 491 -9 472 -49 396 -45 359 -26"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke",
          compactPath("288 177 228 64 143 -23"),
        ),
        simpleStroke(
          "lift, then draw the dot",
          compactPath("672 170 795 68 835 -10"),
        ),
      ],
      source: chineseCharacterSource("茶"),
    },
  ],
  // 事: Hanzi Writer Data's eight ordered medians, fitted to the vendored Noto
  // Sans SC 事. Median 1 draws a short horizontal on top; medians 2–4 draw a
  // small 口 under it in the order the standalone character uses; medians 5–7
  // draw a horizontal that turns down at its right end without lifting, a long
  // horizontal and a shorter one; median 8 draws a long vertical from the top
  // down through them all, hooking to the left without lifting. The order, the
  // directions, the turns, the hook and all seven lifts are the source's.
  [
    "chinese:事",
    {
      script: "chinese",
      glyph: "事",
      strokes: [
        simpleStroke(
          "draw the top horizontal",
          compactPath("79 732 159 716 306 730 530 722 793 735 919 718"),
        ),
        simpleStroke(
          "lift, then draw 口's left side",
          compactPath("207 618 207 490"),
        ),
        simpleStroke(
          "lift, then draw 口's top and turn down its right side",
          compactPath("207 613 760 613 797 605 797 505"),
        ),
        simpleStroke(
          "lift, then close 口 with a horizontal",
          compactPath("207 484 797 484"),
        ),
        simpleStroke(
          "lift, then draw the horizontal and turn down",
          compactPath(
            "156 376 286 357 501 369 760 365 799 340 779 239 792 218 800 78",
          ),
        ),
        simpleStroke(
          "lift, then draw the long horizontal",
          compactPath("58 237 151 223 493 243 718 232 881 239 941 227"),
        ),
        simpleStroke(
          "lift, then draw the shorter horizontal",
          compactPath("148 104 280 93 389 107 642 108 835 94"),
        ),
        simpleStroke(
          "lift, then draw the long vertical and hook left",
          compactPath("475 825 498 758 491 24 466 -41 394 -55 318 -36"),
        ),
      ],
      source: chineseCharacterSource("事"),
    },
  ],
  // 觉: Hanzi Writer Data's nine ordered medians, fitted to the vendored Noto
  // Sans SC 觉. in three pieces. Medians 1–3 draw the three short strokes on
  // top, left to right: two dots, then a short left-falling stroke. Medians 4–5
  // draw the cover: a short stroke on the left, then a horizontal that hooks
  // down to the left without lifting. Medians 6–9 draw 见 at the foot: its left
  // side, a horizontal that turns down the right side without lifting, a left-
  // falling stroke, then a vertical that bends right along the foot and hooks
  // up without lifting. Noto's cover turns straight down at its right end,
  // where the source's hooks to the left. The order, the directions, the turns,
  // the hooks and all eight lifts are the source's.
  [
    "chinese:觉",
    {
      script: "chinese",
      glyph: "觉",
      strokes: [
        simpleStroke(
          "draw the left dot",
          compactPath("208 787 277 688"),
        ),
        simpleStroke(
          "lift, then draw the middle dot",
          compactPath("441 809 532 696"),
        ),
        simpleStroke(
          "lift, then draw the short left-falling stroke",
          compactPath("805 789 700 657"),
        ),
        simpleStroke(
          "lift, then draw the cover's left stroke",
          compactPath("121 632 121 483"),
        ),
        simpleStroke(
          "lift, then draw the cover's horizontal and turn down",
          compactPath("108 630 882 630 880 483"),
        ),
        simpleStroke(
          "lift, then draw 见's left side",
          compactPath("245 472 245 158"),
        ),
        simpleStroke(
          "lift, then draw 见's top and turn down its right side",
          compactPath("245 465 754 465 754 148"),
        ),
        simpleStroke(
          "lift, then draw 见's left-falling stroke",
          compactPath("483 374 497 345 469 177 397 72 306 10 215 -24 109 -42"),
        ),
        simpleStroke(
          "lift, then draw 见's last stroke down, bend right and hook up",
          compactPath(
            "559 187 579 137 568 56 582 -11 610 -27 794 -33 898 -11 908 39 900 80",
          ),
        ),
      ],
      source: chineseCharacterSource("觉"),
    },
  ],
];
