// Authored telugu ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import telugu from "../../../../../learning/human-languages/data/scripts/telugu.json";

const teluguLetterSource = (glyph: string): StrokeSource => {
  const letter = [...telugu.letters, ...telugu.independentVowels].find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Telugu letter ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const teluguIndependentVowelSource = teluguLetterSource;

export const entries: DuctusEntry[] = [
  [
    "telugu:త",
    {
      script: "telugu",
      glyph: "త",
      strokes: [
        {
          segments: [
            {
              label: "curl upward along the inner-left shoulder",
              path: [
                { x: 88, y: 212 }, { x: 131, y: 211 },
                { x: 174, y: 204 }, { x: 217, y: 204 },
                { x: 258, y: 216 }, { x: 291, y: 244 },
                { x: 308, y: 283 }, { x: 307, y: 326 },
                { x: 288, y: 364 },
              ],
            },
            {
              label: "turn downward around the outer-left bowl",
              path: [
                { x: 288, y: 364 }, { x: 250, y: 384 },
                { x: 208, y: 388 }, { x: 165, y: 382 },
                { x: 128, y: 361 }, { x: 99, y: 330 },
                { x: 80, y: 291 }, { x: 76, y: 249 },
                { x: 85, y: 206 }, { x: 89, y: 164 },
                { x: 110, y: 127 }, { x: 140, y: 96 },
              ],
            },
            {
              label: "sweep right around the broad lower bowl",
              path: [
                { x: 140, y: 96 }, { x: 177, y: 73 },
                { x: 216, y: 55 }, { x: 258, y: 42 },
                { x: 300, y: 34 }, { x: 343, y: 29 },
                { x: 387, y: 28 }, { x: 430, y: 28 },
                { x: 473, y: 32 }, { x: 516, y: 40 },
                { x: 558, y: 51 }, { x: 597, y: 69 },
                { x: 633, y: 94 }, { x: 664, y: 124 },
              ],
            },
            {
              label: "curve upward around the outer-right bowl",
              path: [
                { x: 664, y: 124 }, { x: 688, y: 164 },
                { x: 696, y: 209 }, { x: 695, y: 255 },
                { x: 699, y: 301 }, { x: 680, y: 343 },
                { x: 647, y: 376 }, { x: 608, y: 400 },
              ],
            },
            {
              label: "turn down the inner-right shoulder",
              path: [
                { x: 608, y: 400 }, { x: 565, y: 384 },
                { x: 520, y: 371 }, { x: 485, y: 341 },
                { x: 472, y: 297 }, { x: 482, y: 253 },
                { x: 514, y: 220 }, { x: 558, y: 208 },
                { x: 604, y: 209 }, { x: 647, y: 225 },
                { x: 692, y: 236 },
              ],
            },
            {
              label: "curve up and left over the upper shoulder",
              path: [
                { x: 692, y: 236 }, { x: 699, y: 279 },
                { x: 692, y: 323 }, { x: 666, y: 358 },
                { x: 633, y: 387 }, { x: 597, y: 412 },
                { x: 563, y: 440 }, { x: 521, y: 453 },
                { x: 484, y: 476 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 484, y: 476 }, { x: 438, y: 468 },
                { x: 390, y: 469 }, { x: 351, y: 493 },
                { x: 324, y: 532 },
              ],
            },
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 324, y: 532 }, { x: 346, y: 495 },
                { x: 382, y: 470 }, { x: 426, y: 468 },
                { x: 470, y: 471 }, { x: 499, y: 503 },
                { x: 527, y: 537 }, { x: 554, y: 571 },
                { x: 582, y: 605 }, { x: 613, y: 637 },
                { x: 647, y: 664 }, { x: 688, y: 680 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("త"),
    },
  ],
  [
    "telugu:థ",
    {
      script: "telugu",
      glyph: "థ",
      strokes: [
        {
          segments: [
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 287, y: 431 }, { x: 246, y: 439 },
                { x: 204, y: 430 }, { x: 168, y: 406 },
                { x: 137, y: 376 }, { x: 112, y: 341 },
                { x: 94, y: 303 }, { x: 83, y: 261 },
                { x: 79, y: 218 }, { x: 79, y: 175 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 79, y: 175 }, { x: 84, y: 131 },
                { x: 99, y: 90 }, { x: 127, y: 56 },
                { x: 166, y: 34 }, { x: 210, y: 31 },
                { x: 254, y: 35 }, { x: 292, y: 57 },
                { x: 324, y: 88 }, { x: 350, y: 123 },
                { x: 351, y: 167 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 351, y: 167 }, { x: 359, y: 122 },
                { x: 388, y: 86 }, { x: 420, y: 52 },
                { x: 463, y: 33 }, { x: 509, y: 31 },
                { x: 554, y: 39 }, { x: 591, y: 68 },
                { x: 615, y: 108 }, { x: 625, y: 153 },
                { x: 631, y: 199 },
              ],
            },
            {
              label: "curve left round the upper-right shoulder",
              path: [
                { x: 631, y: 199 }, { x: 628, y: 243 },
                { x: 619, y: 286 }, { x: 604, y: 327 },
                { x: 581, y: 365 }, { x: 551, y: 397 },
                { x: 516, y: 423 }, { x: 477, y: 443 },
                { x: 435, y: 457 }, { x: 394, y: 472 },
                { x: 351, y: 463 }, { x: 307, y: 463 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 307, y: 463 }, { x: 263, y: 484 },
                { x: 237, y: 529 }, { x: 207, y: 571 },
              ],
            },
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 207, y: 571 }, { x: 232, y: 536 },
                { x: 255, y: 498 }, { x: 282, y: 467 },
                { x: 326, y: 463 }, { x: 369, y: 466 },
                { x: 406, y: 486 }, { x: 430, y: 523 },
                { x: 457, y: 557 }, { x: 484, y: 591 },
                { x: 512, y: 624 }, { x: 545, y: 653 },
                { x: 583, y: 675 }, { x: 623, y: 691 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate lower stem downward",
              path: [
                { x: 355, y: 11 }, { x: 355, y: -33 },
                { x: 353, y: -77 }, { x: 355, y: -121 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "place the separate inner dot",
              path: [
                { x: 325, y: 265 }, { x: 380, y: 265 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("థ"),
    },
  ],
  [
    "telugu:ద",
    {
      script: "telugu",
      glyph: "ద",
      strokes: [
        {
          segments: [
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 287, y: 431 }, { x: 246, y: 439 },
                { x: 204, y: 430 }, { x: 168, y: 406 },
                { x: 137, y: 376 }, { x: 112, y: 341 },
                { x: 94, y: 303 }, { x: 83, y: 261 },
                { x: 79, y: 218 }, { x: 79, y: 175 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 79, y: 175 }, { x: 84, y: 131 },
                { x: 99, y: 90 }, { x: 127, y: 56 },
                { x: 165, y: 34 }, { x: 208, y: 31 },
                { x: 252, y: 35 }, { x: 290, y: 56 },
                { x: 319, y: 89 }, { x: 347, y: 124 },
                { x: 351, y: 167 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 351, y: 167 }, { x: 359, y: 122 },
                { x: 388, y: 86 }, { x: 421, y: 53 },
                { x: 463, y: 33 }, { x: 509, y: 31 },
                { x: 555, y: 39 }, { x: 591, y: 68 },
                { x: 615, y: 108 }, { x: 625, y: 153 },
                { x: 631, y: 199 },
              ],
            },
            {
              label: "curve left round the upper-right shoulder",
              path: [
                { x: 631, y: 199 }, { x: 628, y: 243 },
                { x: 619, y: 286 }, { x: 604, y: 327 },
                { x: 581, y: 365 }, { x: 551, y: 397 },
                { x: 516, y: 423 }, { x: 477, y: 443 },
                { x: 435, y: 457 }, { x: 394, y: 472 },
                { x: 351, y: 463 }, { x: 307, y: 463 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 307, y: 463 }, { x: 263, y: 484 },
                { x: 237, y: 529 }, { x: 207, y: 571 },
              ],
            },
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 207, y: 571 }, { x: 232, y: 536 },
                { x: 255, y: 498 }, { x: 282, y: 467 },
                { x: 326, y: 463 }, { x: 369, y: 466 },
                { x: 406, y: 486 }, { x: 430, y: 523 },
                { x: 457, y: 557 }, { x: 484, y: 591 },
                { x: 512, y: 624 }, { x: 545, y: 653 },
                { x: 583, y: 675 }, { x: 623, y: 691 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ద"),
    },
  ],
  [
    "telugu:ధ",
    {
      script: "telugu",
      glyph: "ధ",
      strokes: [
        {
          segments: [
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 287, y: 431 }, { x: 246, y: 439 },
                { x: 204, y: 430 }, { x: 168, y: 406 },
                { x: 137, y: 376 }, { x: 112, y: 341 },
                { x: 94, y: 303 }, { x: 83, y: 261 },
                { x: 79, y: 218 }, { x: 79, y: 175 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 79, y: 175 }, { x: 84, y: 131 },
                { x: 99, y: 90 }, { x: 127, y: 56 },
                { x: 165, y: 34 }, { x: 208, y: 31 },
                { x: 252, y: 35 }, { x: 290, y: 56 },
                { x: 319, y: 89 }, { x: 347, y: 124 },
                { x: 351, y: 167 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 351, y: 167 }, { x: 359, y: 122 },
                { x: 388, y: 86 }, { x: 421, y: 53 },
                { x: 463, y: 33 }, { x: 509, y: 31 },
                { x: 555, y: 39 }, { x: 591, y: 68 },
                { x: 615, y: 108 }, { x: 625, y: 153 },
                { x: 631, y: 199 },
              ],
            },
            {
              label: "curve left round the upper-right shoulder",
              path: [
                { x: 631, y: 199 }, { x: 628, y: 243 },
                { x: 619, y: 286 }, { x: 604, y: 327 },
                { x: 581, y: 365 }, { x: 551, y: 397 },
                { x: 516, y: 423 }, { x: 477, y: 443 },
                { x: 435, y: 457 }, { x: 394, y: 472 },
                { x: 351, y: 463 }, { x: 307, y: 463 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 307, y: 463 }, { x: 263, y: 484 },
                { x: 237, y: 529 }, { x: 207, y: 571 },
              ],
            },
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 207, y: 571 }, { x: 232, y: 536 },
                { x: 255, y: 498 }, { x: 282, y: 467 },
                { x: 326, y: 463 }, { x: 369, y: 466 },
                { x: 406, y: 486 }, { x: 430, y: 523 },
                { x: 457, y: 557 }, { x: 484, y: 591 },
                { x: 512, y: 624 }, { x: 545, y: 653 },
                { x: 583, y: 675 }, { x: 623, y: 691 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate lower stem downward",
              path: [
                { x: 355, y: 11 }, { x: 355, y: -33 },
                { x: 353, y: -77 }, { x: 355, y: -121 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ధ"),
    },
  ],
  [
    "telugu:న",
    {
      script: "telugu",
      glyph: "న",
      strokes: [
        {
          segments: [
            {
              label: "sweep up the left bowl into the middle",
              path: [
                { x: 105, y: 7 }, { x: 90, y: 51 },
                { x: 76, y: 95 }, { x: 73, y: 141 },
                { x: 78, y: 187 }, { x: 106, y: 223 },
                { x: 147, y: 245 }, { x: 193, y: 244 },
                { x: 233, y: 222 }, { x: 265, y: 188 },
                { x: 293, y: 152 }, { x: 322, y: 115 },
                { x: 354, y: 81 }, { x: 389, y: 51 },
              ],
            },
            {
              label: "sweep right and up the broad lower bowl",
              path: [
                { x: 389, y: 51 }, { x: 430, y: 34 },
                { x: 474, y: 31 }, { x: 518, y: 33 },
                { x: 559, y: 49 }, { x: 592, y: 79 },
                { x: 613, y: 118 }, { x: 623, y: 161 },
                { x: 625, y: 206 }, { x: 620, y: 250 },
                { x: 607, y: 292 }, { x: 586, y: 331 },
                { x: 558, y: 365 }, { x: 524, y: 394 },
                { x: 486, y: 417 }, { x: 446, y: 436 },
                { x: 403, y: 449 }, { x: 360, y: 458 },
                { x: 321, y: 479 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 321, y: 479 }, { x: 282, y: 468 },
                { x: 241, y: 467 }, { x: 202, y: 474 },
                { x: 172, y: 501 }, { x: 149, y: 535 },
              ],
            },
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 149, y: 535 }, { x: 174, y: 498 },
                { x: 210, y: 470 }, { x: 255, y: 467 },
                { x: 300, y: 470 }, { x: 330, y: 502 },
                { x: 356, y: 539 }, { x: 384, y: 575 },
                { x: 413, y: 610 }, { x: 445, y: 643 },
                { x: 482, y: 668 }, { x: 525, y: 683 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("న"),
    },
  ],
  [
    "telugu:ప",
    {
      script: "telugu",
      glyph: "ప",
      strokes: [
        {
          segments: [
            {
              label: "sweep left across the upper-left curve",
              path: [
                { x: 361, y: 88 }, { x: 323, y: 118 },
                { x: 300, y: 160 }, { x: 268, y: 197 },
                { x: 232, y: 228 }, { x: 186, y: 243 },
                { x: 139, y: 240 }, { x: 97, y: 216 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 97, y: 216 }, { x: 76, y: 179 },
                { x: 69, y: 136 }, { x: 76, y: 94 },
                { x: 101, y: 60 }, { x: 135, y: 34 },
                { x: 177, y: 28 }, { x: 220, y: 32 },
                { x: 256, y: 55 }, { x: 287, y: 86 },
                { x: 317, y: 116 },
              ],
            },
            {
              label: "sweep upward around the broad right bowl",
              path: [
                { x: 317, y: 116 }, { x: 353, y: 92 },
                { x: 386, y: 63 }, { x: 423, y: 40 },
                { x: 466, y: 32 }, { x: 510, y: 32 },
                { x: 553, y: 40 }, { x: 589, y: 65 },
                { x: 614, y: 100 }, { x: 628, y: 142 },
                { x: 633, y: 185 }, { x: 632, y: 229 },
                { x: 624, y: 272 }, { x: 607, y: 313 },
                { x: 584, y: 350 }, { x: 554, y: 383 },
                { x: 521, y: 411 }, { x: 485, y: 436 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up through the separate top flourish",
              path: [
                { x: 85, y: 570 }, { x: 105, y: 520 },
                { x: 140, y: 470 }, { x: 185, y: 440 },
                { x: 230, y: 445 }, { x: 275, y: 480 },
                { x: 320, y: 535 }, { x: 370, y: 595 },
                { x: 425, y: 650 }, { x: 480, y: 690 },
                { x: 525, y: 700 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ప"),
    },
  ],
  [
    "telugu:ఫ",
    {
      script: "telugu",
      glyph: "ఫ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left across the upper-left curve",
              path: [
                { x: 329, y: 115 }, { x: 305, y: 150 },
                { x: 279, y: 184 }, { x: 249, y: 214 },
                { x: 212, y: 237 }, { x: 170, y: 243 },
                { x: 128, y: 235 }, { x: 93, y: 211 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 93, y: 211 }, { x: 74, y: 173 },
                { x: 69, y: 131 }, { x: 78, y: 89 },
                { x: 102, y: 54 }, { x: 138, y: 32 },
                { x: 181, y: 27 }, { x: 223, y: 33 },
                { x: 258, y: 57 }, { x: 288, y: 88 },
                { x: 317, y: 119 },
              ],
            },
            {
              label: "sweep upward around the broad right bowl",
              path: [
                { x: 317, y: 119 }, { x: 353, y: 95 },
                { x: 385, y: 64 }, { x: 422, y: 41 },
                { x: 465, y: 31 }, { x: 509, y: 31 },
                { x: 551, y: 40 }, { x: 585, y: 67 },
                { x: 614, y: 101 }, { x: 628, y: 142 },
                { x: 633, y: 185 }, { x: 632, y: 229 },
                { x: 624, y: 273 }, { x: 607, y: 313 },
                { x: 583, y: 350 }, { x: 555, y: 384 },
                { x: 521, y: 412 }, { x: 484, y: 436 },
                { x: 445, y: 455 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up through the separate top flourish",
              path: [
                { x: 105, y: 555 }, { x: 121, y: 513 },
                { x: 150, y: 480 }, { x: 192, y: 464 },
                { x: 236, y: 469 }, { x: 274, y: 493 },
                { x: 306, y: 525 }, { x: 334, y: 560 },
                { x: 362, y: 595 }, { x: 391, y: 629 },
                { x: 426, y: 658 }, { x: 466, y: 678 },
                { x: 509, y: 691 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the lower stem downward",
              path: [
                { x: 325, y: 12 }, { x: 325, y: -32 },
                { x: 325, y: -78 }, { x: 325, y: -120 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఫ"),
    },
  ],
  [
    "telugu:బ",
    {
      script: "telugu",
      glyph: "బ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right around the upper-left curve",
              path: [
                { x: 84, y: 328 }, { x: 84, y: 373 },
                { x: 92, y: 417 }, { x: 126, y: 447 },
                { x: 169, y: 460 }, { x: 215, y: 463 },
                { x: 259, y: 455 }, { x: 296, y: 430 },
                { x: 315, y: 389 }, { x: 316, y: 344 },
              ],
            },
            {
              label: "curve down and left through the shoulder",
              path: [
                { x: 316, y: 344 }, { x: 289, y: 309 },
                { x: 256, y: 280 }, { x: 217, y: 261 },
                { x: 176, y: 245 }, { x: 137, y: 224 },
                { x: 104, y: 196 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 104, y: 196 }, { x: 86, y: 156 },
                { x: 85, y: 112 }, { x: 102, y: 72 },
                { x: 136, y: 43 }, { x: 178, y: 30 },
                { x: 222, y: 28 }, { x: 266, y: 35 },
                { x: 305, y: 55 }, { x: 337, y: 85 },
                { x: 372, y: 112 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 372, y: 112 }, { x: 409, y: 87 },
                { x: 442, y: 56 }, { x: 482, y: 35 },
                { x: 528, y: 32 }, { x: 573, y: 34 },
                { x: 614, y: 53 }, { x: 645, y: 86 },
                { x: 664, y: 127 }, { x: 672, y: 172 },
              ],
            },
            {
              label: "curve left round the upper-right shoulder",
              path: [
                { x: 672, y: 172 }, { x: 672, y: 217 },
                { x: 666, y: 261 }, { x: 651, y: 303 },
                { x: 628, y: 341 }, { x: 600, y: 375 },
                { x: 568, y: 406 }, { x: 531, y: 432 },
                { x: 492, y: 452 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("బ"),
    },
  ],
  [
    "telugu:భ",
    {
      script: "telugu",
      glyph: "భ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right around the upper-left curve",
              path: [
                { x: 84, y: 311 }, { x: 80, y: 357 },
                { x: 85, y: 404 }, { x: 115, y: 439 },
                { x: 157, y: 458 }, { x: 204, y: 463 },
                { x: 251, y: 458 }, { x: 290, y: 433 },
                { x: 314, y: 394 }, { x: 316, y: 347 },
              ],
            },
            {
              label: "curve down and left through the shoulder",
              path: [
                { x: 316, y: 347 }, { x: 297, y: 309 },
                { x: 265, y: 281 }, { x: 226, y: 261 },
                { x: 185, y: 247 }, { x: 146, y: 229 },
                { x: 112, y: 203 }, { x: 88, y: 167 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 88, y: 167 }, { x: 88, y: 121 },
                { x: 97, y: 78 }, { x: 130, y: 46 },
                { x: 172, y: 31 }, { x: 218, y: 31 },
                { x: 263, y: 35 }, { x: 304, y: 54 },
                { x: 337, y: 85 }, { x: 362, y: 123 },
                { x: 356, y: 167 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 356, y: 167 }, { x: 378, y: 128 },
                { x: 405, y: 92 }, { x: 437, y: 59 },
                { x: 475, y: 37 }, { x: 520, y: 30 },
                { x: 565, y: 31 }, { x: 605, y: 49 },
                { x: 638, y: 81 }, { x: 662, y: 119 },
                { x: 671, y: 163 }, { x: 672, y: 208 },
                { x: 668, y: 253 }, { x: 654, y: 296 },
                { x: 633, y: 335 }, { x: 604, y: 370 },
                { x: 572, y: 402 }, { x: 539, y: 432 },
                { x: 503, y: 459 }, { x: 458, y: 464 },
                { x: 416, y: 479 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 416, y: 479 }, { x: 390, y: 513 },
                { x: 368, y: 551 },
              ],
            },
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 368, y: 551 }, { x: 389, y: 513 },
                { x: 417, y: 480 }, { x: 457, y: 464 },
                { x: 501, y: 460 }, { x: 535, y: 485 },
                { x: 566, y: 517 }, { x: 594, y: 550 },
                { x: 621, y: 584 }, { x: 649, y: 618 },
                { x: 680, y: 648 }, { x: 717, y: 672 },
                { x: 756, y: 691 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the lower stem downward",
              path: [
                { x: 376, y: 11 }, { x: 376, y: -33 },
                { x: 374, y: -77 }, { x: 376, y: -121 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("భ"),
    },
  ],
  [
    "telugu:మ",
    {
      script: "telugu",
      glyph: "మ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left around the upper-left arch",
              path: [
                { x: 285, y: 228 }, { x: 241, y: 224 },
                { x: 199, y: 241 }, { x: 154, y: 243 },
                { x: 112, y: 225 }, { x: 81, y: 192 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 81, y: 192 }, { x: 69, y: 150 },
                { x: 72, y: 105 }, { x: 91, y: 66 },
                { x: 125, y: 38 }, { x: 168, y: 28 },
                { x: 212, y: 30 }, { x: 252, y: 49 },
                { x: 283, y: 81 }, { x: 311, y: 115 },
                { x: 293, y: 152 },
              ],
            },
            {
              label: "turn right around the lower-middle bowl",
              path: [
                { x: 293, y: 152 }, { x: 325, y: 117 },
                { x: 361, y: 87 }, { x: 396, y: 55 },
                { x: 438, y: 34 }, { x: 485, y: 32 },
                { x: 532, y: 34 }, { x: 575, y: 53 },
                { x: 607, y: 87 }, { x: 637, y: 124 },
              ],
            },
            {
              label: "arc up and left over the central shoulder",
              path: [
                { x: 637, y: 124 }, { x: 633, y: 168 },
                { x: 633, y: 213 }, { x: 627, y: 257 },
                { x: 613, y: 298 }, { x: 591, y: 337 },
                { x: 562, y: 371 }, { x: 527, y: 398 },
                { x: 489, y: 421 }, { x: 448, y: 438 },
                { x: 405, y: 450 }, { x: 361, y: 460 },
                { x: 320, y: 474 }, { x: 276, y: 468 },
                { x: 231, y: 468 }, { x: 189, y: 480 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 189, y: 480 }, { x: 162, y: 515 },
                { x: 137, y: 552 },
              ],
            },
            {
              label: "sweep right and up into the top flourish",
              path: [
                { x: 137, y: 552 }, { x: 163, y: 513 },
                { x: 193, y: 479 }, { x: 238, y: 468 },
                { x: 284, y: 472 }, { x: 319, y: 502 },
                { x: 350, y: 537 }, { x: 381, y: 572 },
                { x: 411, y: 606 }, { x: 445, y: 639 },
                { x: 482, y: 667 }, { x: 525, y: 684 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 657, y: 116 }, { x: 691, y: 87 },
                { x: 725, y: 60 }, { x: 765, y: 40 },
                { x: 807, y: 29 }, { x: 851, y: 29 },
                { x: 894, y: 38 }, { x: 932, y: 61 },
                { x: 959, y: 95 }, { x: 974, y: 136 },
                { x: 981, y: 180 },
              ],
            },
            {
              label: "curve up and left over the right shoulder",
              path: [
                { x: 981, y: 180 }, { x: 981, y: 224 },
                { x: 972, y: 267 }, { x: 956, y: 308 },
                { x: 933, y: 345 }, { x: 905, y: 379 },
                { x: 872, y: 408 }, { x: 840, y: 438 },
                { x: 813, y: 472 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("మ"),
    },
  ],
  [
    "telugu:య",
    {
      script: "telugu",
      glyph: "య",
      strokes: [
        {
          segments: [
            {
              label: "loop the left bowl counterclockwise",
              path: [
                { x: 543, y: 216 }, { x: 517, y: 251 },
                { x: 511, y: 295 }, { x: 498, y: 337 },
                { x: 475, y: 375 }, { x: 444, y: 407 },
                { x: 409, y: 434 }, { x: 369, y: 451 },
                { x: 325, y: 456 }, { x: 281, y: 456 },
                { x: 236, y: 454 }, { x: 195, y: 441 },
                { x: 157, y: 417 }, { x: 126, y: 386 },
                { x: 102, y: 348 }, { x: 86, y: 307 },
                { x: 79, y: 264 }, { x: 79, y: 219 },
                { x: 85, y: 176 }, { x: 100, y: 134 },
                { x: 127, y: 100 }, { x: 159, y: 69 },
                { x: 197, y: 46 }, { x: 239, y: 33 },
                { x: 283, y: 28 }, { x: 327, y: 29 },
                { x: 371, y: 37 }, { x: 412, y: 53 },
                { x: 448, y: 78 }, { x: 479, y: 110 },
                { x: 507, y: 144 },
              ],
            },
            {
              label: "loop the centre bowl counterclockwise",
              path: [
                { x: 507, y: 144 }, { x: 543, y: 115 },
                { x: 577, y: 83 }, { x: 614, y: 56 },
                { x: 655, y: 37 }, { x: 701, y: 32 },
                { x: 747, y: 32 }, { x: 791, y: 46 },
                { x: 826, y: 76 }, { x: 853, y: 114 },
                { x: 863, y: 157 }, { x: 863, y: 204 },
                { x: 857, y: 249 }, { x: 843, y: 293 },
                { x: 821, y: 333 }, { x: 791, y: 368 },
                { x: 757, y: 400 }, { x: 720, y: 428 },
                { x: 687, y: 460 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw down and up the lower angled join",
              path: [
                { x: 520, y: 550 }, { x: 560, y: 520 },
                { x: 600, y: 470 }, { x: 640, y: 460 },
                { x: 680, y: 480 }, { x: 730, y: 530 },
                { x: 780, y: 590 }, { x: 840, y: 650 },
                { x: 900, y: 685 }, { x: 950, y: 680 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "loop the right bowl counterclockwise",
              path: [
                { x: 895, y: 113 }, { x: 973, y: 23 },
                { x: 1083, y: -10 }, { x: 1185, y: 47 },
                { x: 1228, y: 155 }, { x: 1218, y: 273 },
                { x: 1169, y: 382 }, { x: 1083, y: 463 },
                { x: 1047, y: 480 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("య"),
    },
  ],
  [
    "telugu:ర",
    {
      script: "telugu",
      glyph: "ర",
      strokes: [
        {
          segments: [
            {
              label: "loop the main bowl counterclockwise",
              path: [
                { x: 295, y: 464 }, { x: 250, y: 464 },
                { x: 206, y: 457 }, { x: 171, y: 429 },
                { x: 136, y: 400 }, { x: 109, y: 364 },
                { x: 90, y: 323 }, { x: 80, y: 279 },
                { x: 79, y: 234 }, { x: 82, y: 189 },
                { x: 95, y: 146 }, { x: 118, y: 107 },
                { x: 150, y: 75 }, { x: 188, y: 51 },
                { x: 230, y: 35 }, { x: 275, y: 28 },
                { x: 320, y: 28 }, { x: 365, y: 35 },
                { x: 408, y: 50 }, { x: 445, y: 75 },
                { x: 476, y: 108 }, { x: 498, y: 148 },
                { x: 511, y: 191 }, { x: 515, y: 236 },
                { x: 514, y: 281 }, { x: 503, y: 325 },
                { x: 484, y: 366 }, { x: 455, y: 400 },
                { x: 421, y: 429 }, { x: 380, y: 450 },
                { x: 340, y: 469 }, { x: 295, y: 464 },
              ],
            },
            {
              label: "climb up the chevron's left arm",
              path: [
                { x: 295, y: 464 }, { x: 243, y: 464 },
                { x: 203, y: 490 }, { x: 179, y: 536 },
              ],
            },
            {
              label: "draw the upper chevron down and up",
              path: [
                { x: 179, y: 536 }, { x: 198, y: 497 },
                { x: 227, y: 466 }, { x: 270, y: 464 },
                { x: 314, y: 467 }, { x: 349, y: 489 },
                { x: 373, y: 525 }, { x: 401, y: 559 },
                { x: 428, y: 593 }, { x: 457, y: 626 },
                { x: 489, y: 655 }, { x: 527, y: 676 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ర"),
    },
  ],
  [
    "telugu:ల",
    {
      script: "telugu",
      glyph: "ల",
      strokes: [
        {
          segments: [
            {
              label: "loop the small upper bowl counterclockwise",
              path: [
                { x: 119, y: 392 }, { x: 89, y: 358 },
                { x: 80, y: 314 }, { x: 114, y: 290 },
                { x: 157, y: 276 }, { x: 202, y: 272 },
                { x: 247, y: 276 }, { x: 287, y: 298 },
                { x: 311, y: 336 }, { x: 315, y: 381 },
                { x: 301, y: 424 }, { x: 267, y: 454 },
                { x: 223, y: 460 }, { x: 177, y: 457 },
                { x: 138, y: 434 }, { x: 108, y: 401 },
                { x: 87, y: 360 },
              ],
            },
            {
              label: "sweep down round the broad lower bowl",
              path: [
                { x: 87, y: 360 }, { x: 80, y: 316 },
                { x: 79, y: 272 }, { x: 79, y: 228 },
                { x: 88, y: 184 }, { x: 107, y: 144 },
                { x: 134, y: 109 }, { x: 167, y: 79 },
                { x: 206, y: 58 }, { x: 247, y: 41 },
                { x: 291, y: 32 }, { x: 335, y: 28 },
                { x: 379, y: 28 }, { x: 424, y: 32 },
                { x: 467, y: 41 }, { x: 509, y: 55 },
                { x: 548, y: 77 }, { x: 581, y: 106 },
                { x: 607, y: 142 }, { x: 623, y: 184 },
                { x: 631, y: 227 }, { x: 631, y: 272 },
                { x: 622, y: 315 }, { x: 604, y: 356 },
                { x: 578, y: 392 }, { x: 545, y: 421 },
                { x: 507, y: 444 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ల"),
    },
  ],
  [
    "telugu:ళ",
    {
      script: "telugu",
      glyph: "ళ",
      strokes: [
        {
          segments: [
            {
              label: "loop the small inner bowl counterclockwise",
              path: [
                { x: 125, y: 396 }, { x: 92, y: 366 },
                { x: 73, y: 327 }, { x: 76, y: 283 },
                { x: 101, y: 246 }, { x: 138, y: 223 },
                { x: 181, y: 212 }, { x: 224, y: 221 },
                { x: 254, y: 253 }, { x: 283, y: 287 },
                { x: 289, y: 331 }, { x: 272, y: 371 },
                { x: 236, y: 397 }, { x: 193, y: 404 },
                { x: 148, y: 403 }, { x: 109, y: 383 },
                { x: 77, y: 352 },
              ],
            },
            {
              label: "sweep down the body, loop the lower bowl",
              path: [
                { x: 77, y: 352 }, { x: 73, y: 307 },
                { x: 87, y: 264 }, { x: 120, y: 233 },
                { x: 161, y: 214 }, { x: 206, y: 210 },
                { x: 247, y: 195 }, { x: 288, y: 180 },
                { x: 334, y: 180 }, { x: 379, y: 179 },
                { x: 416, y: 154 }, { x: 436, y: 113 },
                { x: 434, y: 68 }, { x: 403, y: 36 },
                { x: 359, y: 24 }, { x: 314, y: 24 },
                { x: 269, y: 31 }, { x: 233, y: 58 },
                { x: 222, y: 101 }, { x: 240, y: 142 },
                { x: 265, y: 180 },
              ],
            },
            {
              label: "sweep right and up the broad outer body",
              path: [
                { x: 265, y: 180 }, { x: 310, y: 180 },
                { x: 356, y: 180 }, { x: 401, y: 180 },
                { x: 444, y: 194 }, { x: 486, y: 209 },
                { x: 526, y: 231 }, { x: 556, y: 264 },
                { x: 570, y: 307 }, { x: 569, y: 352 },
                { x: 548, y: 392 }, { x: 514, y: 421 },
                { x: 474, y: 442 }, { x: 437, y: 468 },
              ],
            },
            {
              label: "climb up the chevron's left arm",
              path: [
                { x: 437, y: 468 }, { x: 391, y: 468 },
                { x: 345, y: 469 }, { x: 307, y: 494 },
                { x: 281, y: 532 },
              ],
            },
            {
              label: "draw the upper chevron down and up",
              path: [
                { x: 281, y: 532 }, { x: 304, y: 494 },
                { x: 340, y: 470 }, { x: 384, y: 468 },
                { x: 427, y: 470 }, { x: 455, y: 503 },
                { x: 484, y: 536 }, { x: 511, y: 571 },
                { x: 539, y: 605 }, { x: 570, y: 637 },
                { x: 604, y: 664 }, { x: 645, y: 680 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ళ"),
    },
  ],
  [
    "telugu:వ",
    {
      script: "telugu",
      glyph: "వ",
      strokes: [
        {
          segments: [
            {
              label: "loop the lower-left bowl counterclockwise",
              path: [
                { x: 289, y: 140 }, { x: 280, y: 184 },
                { x: 247, y: 216 }, { x: 208, y: 239 },
                { x: 162, y: 244 }, { x: 119, y: 230 },
                { x: 86, y: 198 }, { x: 70, y: 156 },
                { x: 71, y: 110 }, { x: 92, y: 70 },
                { x: 125, y: 39 }, { x: 169, y: 28 },
                { x: 215, y: 31 }, { x: 254, y: 53 },
                { x: 286, y: 86 }, { x: 317, y: 120 },
              ],
            },
            {
              label: "sweep round the lower and right body",
              path: [
                { x: 317, y: 120 }, { x: 354, y: 94 },
                { x: 387, y: 63 }, { x: 426, y: 39 },
                { x: 470, y: 32 }, { x: 516, y: 32 },
                { x: 560, y: 44 }, { x: 595, y: 72 },
                { x: 619, y: 110 }, { x: 631, y: 154 },
                { x: 633, y: 200 }, { x: 630, y: 245 },
                { x: 617, y: 289 }, { x: 596, y: 329 },
                { x: 568, y: 364 }, { x: 533, y: 394 },
                { x: 495, y: 418 }, { x: 453, y: 436 },
                { x: 409, y: 449 }, { x: 365, y: 459 },
                { x: 325, y: 480 },
              ],
            },
            {
              label: "climb up the chevron's left arm",
              path: [
                { x: 325, y: 480 }, { x: 285, y: 469 },
                { x: 244, y: 468 }, { x: 203, y: 473 },
                { x: 172, y: 499 }, { x: 153, y: 536 },
              ],
            },
            {
              label: "draw the upper chevron down and up",
              path: [
                { x: 153, y: 536 }, { x: 173, y: 498 },
                { x: 206, y: 472 }, { x: 249, y: 468 },
                { x: 292, y: 469 }, { x: 327, y: 490 },
                { x: 349, y: 527 }, { x: 376, y: 561 },
                { x: 403, y: 594 }, { x: 431, y: 626 },
                { x: 464, y: 655 }, { x: 501, y: 676 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("వ"),
    },
  ],
  [
    "telugu:శ",
    {
      script: "telugu",
      glyph: "శ",
      strokes: [
        {
          segments: [
            {
              label: "loop around the broad lower-left bowl",
              path: [
                { x: 85, y: 192 }, { x: 74, y: 233 },
                { x: 92, y: 274 }, { x: 126, y: 302 },
                { x: 168, y: 316 }, { x: 212, y: 320 },
                { x: 256, y: 320 }, { x: 300, y: 313 },
                { x: 341, y: 296 }, { x: 382, y: 279 },
                { x: 418, y: 255 }, { x: 448, y: 222 },
                { x: 469, y: 183 }, { x: 477, y: 139 },
                { x: 474, y: 95 }, { x: 450, y: 58 },
                { x: 413, y: 34 }, { x: 370, y: 28 },
                { x: 325, y: 29 }, { x: 285, y: 47 },
                { x: 258, y: 82 }, { x: 250, y: 125 },
                { x: 261, y: 168 },
              ],
            },
            {
              label: "sweep round the tall lower and right body",
              path: [
                { x: 261, y: 168 }, { x: 288, y: 202 },
                { x: 320, y: 231 }, { x: 350, y: 263 },
                { x: 387, y: 282 }, { x: 417, y: 313 },
                { x: 440, y: 349 }, { x: 444, y: 392 },
                { x: 421, y: 428 }, { x: 388, y: 457 },
                { x: 350, y: 477 }, { x: 308, y: 469 },
                { x: 264, y: 468 }, { x: 224, y: 482 },
                { x: 197, y: 516 },
              ],
            },
            {
              label: "climb up the chevron's left arm",
              path: [
                { x: 197, y: 516 }, { x: 189, y: 536 },
              ],
            },
            {
              label: "draw the upper chevron down and up",
              path: [
                { x: 189, y: 536 }, { x: 211, y: 495 },
                { x: 250, y: 470 }, { x: 297, y: 468 },
                { x: 343, y: 477 }, { x: 374, y: 511 },
                { x: 402, y: 548 }, { x: 431, y: 585 },
                { x: 462, y: 621 }, { x: 496, y: 653 },
                { x: 537, y: 676 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("శ"),
    },
  ],
  [
    "telugu:ష",
    {
      script: "telugu",
      glyph: "ష",
      strokes: [
        {
          segments: [
            {
              label: "loop around the lower-left bowl",
              path: [
                { x: 333, y: 167 }, { x: 291, y: 175 },
                { x: 258, y: 205 }, { x: 224, y: 232 },
                { x: 181, y: 243 }, { x: 138, y: 239 },
                { x: 100, y: 217 }, { x: 76, y: 180 },
                { x: 69, y: 137 }, { x: 76, y: 94 },
                { x: 100, y: 57 }, { x: 136, y: 33 },
                { x: 179, y: 27 }, { x: 222, y: 33 },
                { x: 260, y: 56 }, { x: 289, y: 89 },
                { x: 317, y: 123 },
              ],
            },
            {
              label: "sweep round the lower and right body",
              path: [
                { x: 317, y: 123 }, { x: 353, y: 95 },
                { x: 386, y: 64 }, { x: 424, y: 40 },
                { x: 468, y: 30 }, { x: 513, y: 27 },
                { x: 557, y: 33 }, { x: 589, y: 64 },
                { x: 615, y: 102 }, { x: 629, y: 144 },
                { x: 633, y: 189 }, { x: 632, y: 235 },
                { x: 622, y: 279 }, { x: 603, y: 320 },
                { x: 578, y: 357 }, { x: 546, y: 390 },
                { x: 511, y: 419 }, { x: 473, y: 443 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "add the short lower-right tail",
              path: [
                { x: 615, y: 61 }, { x: 585, y: 5 },
                { x: 600, y: -65 }, { x: 660, y: -105 },
                { x: 736, y: -63 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate chevron down and up",
              path: [
                { x: 99, y: 579 }, { x: 126, y: 548 },
                { x: 152, y: 516 }, { x: 178, y: 484 },
                { x: 205, y: 453 }, { x: 231, y: 433 },
                { x: 258, y: 464 }, { x: 283, y: 497 },
                { x: 309, y: 529 }, { x: 336, y: 560 },
                { x: 362, y: 592 }, { x: 389, y: 623 },
                { x: 416, y: 654 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ష"),
    },
  ],
  [
    "telugu:స",
    {
      script: "telugu",
      glyph: "స",
      strokes: [
        {
          segments: [
            {
              label: "loop the left bowl, sweep the right body",
              path: [
                { x: 70, y: 59 }, { x: 55, y: 73 },
                { x: 47, y: 82 }, { x: 40, y: 99 },
                { x: 31, y: 129 }, { x: 33, y: 170 },
                { x: 45, y: 209 }, { x: 68, y: 244 },
                { x: 99, y: 270 }, { x: 139, y: 283 },
                { x: 180, y: 281 }, { x: 218, y: 265 },
                { x: 250, y: 239 }, { x: 275, y: 206 },
                { x: 296, y: 171 }, { x: 318, y: 136 },
                { x: 339, y: 101 }, { x: 363, y: 67 },
                { x: 391, y: 37 }, { x: 425, y: 14 },
                { x: 465, y: 1 }, { x: 506, y: 0 },
                { x: 547, y: 9 }, { x: 584, y: 28 },
                { x: 614, y: 56 }, { x: 636, y: 91 },
                { x: 650, y: 129 }, { x: 658, y: 169 },
                { x: 658, y: 210 }, { x: 652, y: 251 },
                { x: 643, y: 290 }, { x: 629, y: 329 },
                { x: 610, y: 366 }, { x: 590, y: 380 },
                { x: 570, y: 390 }, { x: 550, y: 400 },
                { x: 525, y: 410 }, { x: 495, y: 420 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate chevron down and up",
              path: [
                { x: 103, y: 569 }, { x: 130, y: 538 },
                { x: 158, y: 507 }, { x: 186, y: 477 },
                { x: 214, y: 447 }, { x: 243, y: 441 },
                { x: 274, y: 469 }, { x: 302, y: 499 },
                { x: 328, y: 531 }, { x: 355, y: 562 },
                { x: 384, y: 592 }, { x: 412, y: 622 },
                { x: 440, y: 645 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("స"),
    },
  ],
  [
    "telugu:హ",
    {
      script: "telugu",
      glyph: "హ",
      strokes: [
        {
          segments: [
            {
              label: "loop around the lower-left bowl",
              path: [
                { x: 285, y: 152 }, { x: 271, y: 193 },
                { x: 237, y: 224 }, { x: 196, y: 241 },
                { x: 151, y: 242 }, { x: 110, y: 225 },
                { x: 81, y: 191 }, { x: 69, y: 147 },
                { x: 73, y: 103 }, { x: 97, y: 65 },
                { x: 131, y: 36 }, { x: 175, y: 28 },
                { x: 220, y: 32 }, { x: 257, y: 57 },
                { x: 290, y: 89 }, { x: 317, y: 124 },
              ],
            },
            {
              label: "sweep round the lower and right body",
              path: [
                { x: 317, y: 124 }, { x: 352, y: 96 },
                { x: 384, y: 66 }, { x: 422, y: 41 },
                { x: 465, y: 32 }, { x: 509, y: 32 },
                { x: 553, y: 40 }, { x: 589, y: 65 },
                { x: 616, y: 101 }, { x: 629, y: 143 },
                { x: 633, y: 188 }, { x: 632, y: 232 },
                { x: 620, y: 275 }, { x: 601, y: 316 },
                { x: 575, y: 352 }, { x: 544, y: 385 },
                { x: 514, y: 418 }, { x: 481, y: 444 },
              ],
            },
            {
              label: "draw the middle bar right and curl its end",
              path: [
                { x: 481, y: 444 }, { x: 525, y: 454 },
                { x: 569, y: 460 }, { x: 614, y: 461 },
                { x: 659, y: 464 }, { x: 704, y: 464 },
                { x: 749, y: 464 }, { x: 794, y: 456 },
                { x: 838, y: 460 }, { x: 883, y: 459 },
                { x: 926, y: 444 }, { x: 959, y: 414 },
                { x: 978, y: 374 }, { x: 979, y: 329 },
                { x: 961, y: 289 }, { x: 925, y: 262 },
                { x: 880, y: 256 }, { x: 836, y: 260 },
                { x: 800, y: 287 }, { x: 781, y: 327 },
                { x: 779, y: 372 }, { x: 793, y: 415 },
                { x: 809, y: 456 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate chevron down and up",
              path: [
                { x: 77, y: 516 }, { x: 120, y: 508 },
                { x: 153, y: 477 }, { x: 196, y: 464 },
                { x: 241, y: 471 }, { x: 278, y: 497 },
                { x: 309, y: 529 }, { x: 337, y: 565 },
                { x: 366, y: 600 }, { x: 397, y: 632 },
                { x: 432, y: 662 }, { x: 473, y: 680 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("హ"),
    },
  ],
  [
    "telugu:ణ",
    {
      script: "telugu",
      glyph: "ణ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left and up the lower-left bowl",
              path: [
                { x: 329, y: 88 }, { x: 300, y: 56 },
                { x: 261, y: 37 }, { x: 218, y: 28 },
                { x: 175, y: 28 }, { x: 132, y: 38 },
                { x: 98, y: 65 }, { x: 81, y: 104 },
                { x: 81, y: 148 }, { x: 100, y: 186 },
                { x: 132, y: 216 }, { x: 165, y: 244 },
              ],
            },
            {
              label: "curve right across the upper-left bowl",
              path: [
                { x: 165, y: 244 }, { x: 132, y: 277 },
                { x: 100, y: 311 }, { x: 85, y: 354 },
                { x: 93, y: 399 }, { x: 124, y: 433 },
                { x: 165, y: 454 }, { x: 211, y: 460 },
                { x: 258, y: 460 }, { x: 303, y: 452 },
                { x: 344, y: 431 }, { x: 378, y: 399 },
                { x: 413, y: 368 },
              ],
            },
            {
              label: "arch right and down the upper-right bowl",
              path: [
                { x: 413, y: 368 }, { x: 448, y: 395 },
                { x: 481, y: 427 }, { x: 520, y: 450 },
                { x: 564, y: 456 }, { x: 610, y: 456 },
                { x: 652, y: 442 }, { x: 687, y: 414 },
                { x: 715, y: 378 }, { x: 734, y: 337 },
                { x: 744, y: 293 }, { x: 749, y: 248 },
              ],
            },
            {
              label: "turn left around the lower-right bowl",
              path: [
                { x: 749, y: 248 }, { x: 745, y: 202 },
                { x: 733, y: 158 }, { x: 729, y: 113 },
                { x: 705, y: 74 }, { x: 670, y: 45 },
                { x: 627, y: 29 }, { x: 581, y: 28 },
                { x: 536, y: 34 }, { x: 500, y: 63 },
                { x: 481, y: 104 },
              ],
            },
            {
              label: "sweep upward along the inner curve",
              path: [
                { x: 481, y: 104 }, { x: 482, y: 149 },
                { x: 501, y: 189 }, { x: 536, y: 216 },
                { x: 580, y: 224 }, { x: 625, y: 224 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ణ"),
    },
  ],
  [
    "telugu:ఢ",
    {
      script: "telugu",
      glyph: "ఢ",
      strokes: [
        {
          segments: [
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 627, y: 399 }, { x: 584, y: 413 },
                { x: 543, y: 430 }, { x: 500, y: 446 },
                { x: 457, y: 458 }, { x: 415, y: 472 },
                { x: 370, y: 467 }, { x: 325, y: 467 },
                { x: 280, y: 461 }, { x: 240, y: 440 },
                { x: 199, y: 422 }, { x: 162, y: 396 },
                { x: 131, y: 364 }, { x: 106, y: 326 },
                { x: 89, y: 284 }, { x: 80, y: 240 },
                { x: 75, y: 195 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 75, y: 195 }, { x: 79, y: 150 },
                { x: 91, y: 106 }, { x: 115, y: 67 },
                { x: 151, y: 39 }, { x: 195, y: 31 },
                { x: 241, y: 32 }, { x: 282, y: 50 },
                { x: 314, y: 82 }, { x: 344, y: 117 },
                { x: 359, y: 159 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 359, y: 159 }, { x: 363, y: 116 },
                { x: 391, y: 83 }, { x: 423, y: 54 },
                { x: 465, y: 45 }, { x: 506, y: 30 },
                { x: 549, y: 27 }, { x: 592, y: 30 },
                { x: 631, y: 49 }, { x: 659, y: 82 },
                { x: 670, y: 124 }, { x: 671, y: 167 },
              ],
            },
            {
              label: "curve left round the upper-right shoulder",
              path: [
                { x: 671, y: 167 }, { x: 641, y: 204 },
                { x: 598, y: 222 }, { x: 551, y: 223 },
                { x: 506, y: 210 }, { x: 474, y: 176 },
                { x: 467, y: 129 }, { x: 471, y: 82 },
                { x: 488, y: 44 }, { x: 535, y: 47 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up through the separate top flourish",
              path: [
                { x: 255, y: 567 }, { x: 263, y: 524 },
                { x: 283, y: 486 }, { x: 321, y: 467 },
                { x: 365, y: 467 }, { x: 407, y: 476 },
                { x: 438, y: 507 }, { x: 465, y: 541 },
                { x: 493, y: 575 }, { x: 521, y: 609 },
                { x: 552, y: 640 }, { x: 586, y: 667 },
                { x: 626, y: 684 }, { x: 667, y: 699 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate lower stem downward",
              path: [
                { x: 355, y: 11 }, { x: 355, y: -33 },
                { x: 353, y: -77 }, { x: 355, y: -121 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఢ"),
    },
  ],
  [
    "telugu:డ",
    {
      script: "telugu",
      glyph: "డ",
      strokes: [
        {
          segments: [
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 627, y: 399 }, { x: 584, y: 413 },
                { x: 543, y: 430 }, { x: 500, y: 446 },
                { x: 457, y: 458 }, { x: 415, y: 472 },
                { x: 370, y: 467 }, { x: 325, y: 467 },
                { x: 280, y: 461 }, { x: 240, y: 440 },
                { x: 199, y: 422 }, { x: 162, y: 396 },
                { x: 131, y: 364 }, { x: 106, y: 326 },
                { x: 89, y: 284 }, { x: 80, y: 240 },
                { x: 75, y: 195 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 75, y: 195 }, { x: 79, y: 150 },
                { x: 91, y: 106 }, { x: 115, y: 67 },
                { x: 151, y: 39 }, { x: 195, y: 31 },
                { x: 241, y: 32 }, { x: 282, y: 50 },
                { x: 314, y: 82 }, { x: 344, y: 117 },
                { x: 359, y: 159 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 359, y: 159 }, { x: 363, y: 116 },
                { x: 391, y: 83 }, { x: 423, y: 54 },
                { x: 465, y: 45 }, { x: 506, y: 30 },
                { x: 549, y: 27 }, { x: 592, y: 30 },
                { x: 631, y: 49 }, { x: 659, y: 82 },
                { x: 670, y: 124 }, { x: 671, y: 167 },
              ],
            },
            {
              label: "curve left round the upper-right shoulder",
              path: [
                { x: 671, y: 167 }, { x: 641, y: 204 },
                { x: 598, y: 222 }, { x: 551, y: 223 },
                { x: 506, y: 210 }, { x: 474, y: 176 },
                { x: 467, y: 129 }, { x: 471, y: 82 },
                { x: 488, y: 44 }, { x: 535, y: 47 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up through the separate top flourish",
              path: [
                { x: 255, y: 567 }, { x: 263, y: 524 },
                { x: 283, y: 486 }, { x: 321, y: 467 },
                { x: 365, y: 467 }, { x: 407, y: 476 },
                { x: 438, y: 507 }, { x: 465, y: 541 },
                { x: 493, y: 575 }, { x: 521, y: 609 },
                { x: 552, y: 640 }, { x: 586, y: 667 },
                { x: 626, y: 684 }, { x: 667, y: 699 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("డ"),
    },
  ],
  [
    "telugu:ఠ",
    {
      script: "telugu",
      glyph: "ఠ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left round the broad circular body",
              path: [
                { x: 295, y: 464 }, { x: 250, y: 464 },
                { x: 206, y: 457 }, { x: 170, y: 429 },
                { x: 136, y: 400 }, { x: 109, y: 364 },
                { x: 90, y: 323 }, { x: 80, y: 279 },
                { x: 79, y: 234 }, { x: 82, y: 189 },
                { x: 95, y: 145 }, { x: 118, y: 107 },
                { x: 150, y: 74 }, { x: 188, y: 50 },
                { x: 231, y: 35 }, { x: 275, y: 28 },
                { x: 320, y: 28 }, { x: 365, y: 35 },
                { x: 408, y: 50 }, { x: 445, y: 75 },
                { x: 476, y: 108 }, { x: 498, y: 148 },
                { x: 511, y: 191 }, { x: 515, y: 236 },
                { x: 514, y: 281 }, { x: 503, y: 325 },
                { x: 484, y: 366 }, { x: 455, y: 400 },
                { x: 421, y: 429 }, { x: 380, y: 450 },
                { x: 340, y: 469 }, { x: 295, y: 464 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 295, y: 464 }, { x: 243, y: 464 },
                { x: 203, y: 490 }, { x: 179, y: 536 },
              ],
            },
            {
              label: "curl upward through the top flourish",
              path: [
                { x: 179, y: 536 }, { x: 198, y: 497 },
                { x: 227, y: 466 }, { x: 270, y: 464 },
                { x: 314, y: 467 }, { x: 349, y: 489 },
                { x: 374, y: 525 }, { x: 401, y: 559 },
                { x: 428, y: 593 }, { x: 457, y: 626 },
                { x: 489, y: 655 }, { x: 527, y: 676 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "place the separate inner dot",
              path: [
                { x: 270, y: 245 }, { x: 325, y: 245 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఠ"),
    },
  ],
  [
    "telugu:ట",
    {
      script: "telugu",
      glyph: "ట",
      strokes: [
        {
          segments: [
            {
              label: "curl upward along the inner shoulder",
              path: [
                { x: 99, y: 324 }, { x: 132, y: 297 },
                { x: 171, y: 279 }, { x: 213, y: 272 },
                { x: 256, y: 273 }, { x: 297, y: 287 },
                { x: 327, y: 316 }, { x: 339, y: 357 },
                { x: 339, y: 400 },
              ],
            },
            {
              label: "sweep down around the upper-left curve",
              path: [
                { x: 339, y: 400 }, { x: 314, y: 437 },
                { x: 276, y: 460 }, { x: 231, y: 464 },
                { x: 187, y: 452 }, { x: 150, y: 427 },
                { x: 120, y: 394 }, { x: 99, y: 354 },
                { x: 91, y: 310 }, { x: 80, y: 266 },
                { x: 79, y: 221 }, { x: 83, y: 176 },
              ],
            },
            {
              label: "turn right around the lower-left bowl",
              path: [
                { x: 83, y: 176 }, { x: 94, y: 133 },
                { x: 113, y: 94 }, { x: 142, y: 62 },
                { x: 179, y: 38 }, { x: 222, y: 29 },
                { x: 266, y: 30 }, { x: 308, y: 40 },
                { x: 346, y: 63 }, { x: 375, y: 96 },
                { x: 399, y: 132 }, { x: 403, y: 176 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 403, y: 176 }, { x: 404, y: 129 },
                { x: 429, y: 90 }, { x: 460, y: 55 },
                { x: 503, y: 35 }, { x: 550, y: 32 },
                { x: 597, y: 34 }, { x: 639, y: 54 },
                { x: 670, y: 90 }, { x: 688, y: 133 },
                { x: 695, y: 180 },
              ],
            },
            {
              label: "curve up and left over the outer shoulder",
              path: [
                { x: 695, y: 180 }, { x: 695, y: 223 },
                { x: 687, y: 265 }, { x: 672, y: 304 },
                { x: 650, y: 341 }, { x: 622, y: 373 },
                { x: 592, y: 403 }, { x: 558, y: 429 },
                { x: 530, y: 460 }, { x: 515, y: 500 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate upper stem downward",
              path: [
                { x: 235, y: 620 }, { x: 235, y: 575 },
                { x: 235, y: 530 }, { x: 235, y: 485 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ట"),
    },
  ],
  [
    "telugu:ఞ",
    {
      script: "telugu",
      glyph: "ఞ",
      strokes: [
        {
          segments: [
            {
              label: "sweep up around the upper-left loop",
              path: [
                { x: 120, y: 312 }, { x: 83, y: 340 },
                { x: 74, y: 384 }, { x: 93, y: 426 },
                { x: 131, y: 452 }, { x: 177, y: 463 },
                { x: 223, y: 457 }, { x: 263, y: 433 },
                { x: 296, y: 400 }, { x: 329, y: 367 },
              ],
            },
            {
              label: "sweep right around the upper-right loop",
              path: [
                { x: 329, y: 367 }, { x: 361, y: 401 },
                { x: 393, y: 433 }, { x: 434, y: 455 },
                { x: 480, y: 459 }, { x: 525, y: 450 },
                { x: 563, y: 424 }, { x: 591, y: 387 },
                { x: 607, y: 344 }, { x: 613, y: 299 },
              ],
            },
            {
              label: "curve down and left around the lower bowl",
              path: [
                { x: 613, y: 299 }, { x: 611, y: 253 },
                { x: 602, y: 208 }, { x: 584, y: 166 },
                { x: 554, y: 131 }, { x: 528, y: 93 },
                { x: 488, y: 70 }, { x: 447, y: 51 },
                { x: 403, y: 36 }, { x: 357, y: 29 },
                { x: 311, y: 27 }, { x: 265, y: 27 },
                { x: 220, y: 34 }, { x: 177, y: 51 },
              ],
            },
            {
              label: "curl upward around the inner-left loop",
              path: [
                { x: 177, y: 51 }, { x: 148, y: 85 },
                { x: 137, y: 128 }, { x: 149, y: 170 },
                { x: 181, y: 202 }, { x: 220, y: 223 },
                { x: 264, y: 230 }, { x: 309, y: 231 },
              ],
            },
            {
              label: "curl down and right around the inner bowl",
              path: [
                { x: 309, y: 231 }, { x: 355, y: 226 },
                { x: 400, y: 214 }, { x: 443, y: 196 },
                { x: 483, y: 171 }, { x: 517, y: 139 },
                { x: 537, y: 99 },
              ],
            },
            {
              label: "draw the short downward tail",
              path: [
                { x: 537, y: 99 }, { x: 560, y: 59 },
                { x: 569, y: 14 }, { x: 569, y: -32 },
                { x: 577, y: -77 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the right horizontal bar",
              path: [
                { x: 700, y: 320 }, { x: 760, y: 320 },
                { x: 825, y: 320 }, { x: 900, y: 320 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate upper stem downward",
              path: [
                { x: 780, y: 475 }, { x: 780, y: 430 },
                { x: 780, y: 385 }, { x: 780, y: 350 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఞ"),
    },
  ],
  [
    "telugu:ఙ",
    {
      script: "telugu",
      glyph: "ఙ",
      strokes: [
        {
          segments: [
            {
              label: "turn around the compact upper-left lobe",
              path: [
                { x: 128, y: 323 },
                { x: 118, y: 365 },
                { x: 140, y: 410 },
                { x: 175, y: 452 },
                { x: 220, y: 465 },
                { x: 270, y: 445 },
                { x: 310, y: 405 },
                { x: 320, y: 360 },
                { x: 300, y: 305 },
                { x: 260, y: 270 },
              ],
            },
            {
              label: "continue down round the broad lower bowl",
              path: [
                { x: 260, y: 270 },
                { x: 190, y: 245 },
                { x: 125, y: 215 },
                { x: 85, y: 170 },
                { x: 82, y: 120 },
                { x: 105, y: 75 },
                { x: 150, y: 42 },
                { x: 205, y: 25 },
                { x: 260, y: 35 },
                { x: 315, y: 70 },
                { x: 365, y: 115 },
              ],
            },
            {
              label: "curl upward around the rounded right lobe",
              path: [
                { x: 365, y: 115 },
                { x: 410, y: 75 },
                { x: 465, y: 42 },
                { x: 525, y: 25 },
                { x: 585, y: 35 },
                { x: 635, y: 68 },
                { x: 660, y: 115 },
                { x: 660, y: 160 },
                { x: 640, y: 200 },
                { x: 600, y: 225 },
                { x: 565, y: 220 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift and draw the inner bar left to right",
              path: [
                { x: 310, y: 321 },
                { x: 380, y: 321 },
                { x: 450, y: 321 },
                { x: 520, y: 321 },
                { x: 590, y: 321 },
                { x: 660, y: 321 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, draw the upper headstroke down",
              path: [
                { x: 499, y: 480 },
                { x: 499, y: 440 },
                { x: 499, y: 400 },
                { x: 499, y: 360 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఙ"),
    },
  ],
  [
    "telugu:జ",
    {
      script: "telugu",
      glyph: "జ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right across the upper-left arch",
              path: [
                { x: 80, y: 360 }, { x: 85, y: 406 },
                { x: 116, y: 441 }, { x: 159, y: 458 },
                { x: 206, y: 460 }, { x: 252, y: 457 },
                { x: 292, y: 434 }, { x: 315, y: 394 },
                { x: 324, y: 348 },
              ],
            },
            {
              label: "curve down and right round the left bowl",
              path: [
                { x: 324, y: 348 }, { x: 294, y: 314 },
                { x: 261, y: 283 }, { x: 220, y: 263 },
                { x: 178, y: 245 }, { x: 138, y: 225 },
                { x: 104, y: 194 }, { x: 86, y: 153 },
                { x: 85, y: 108 }, { x: 105, y: 68 },
                { x: 141, y: 41 }, { x: 185, y: 32 },
                { x: 231, y: 32 }, { x: 275, y: 38 },
                { x: 312, y: 64 }, { x: 344, y: 96 },
                { x: 372, y: 132 },
              ],
            },
            {
              label: "sweep right and up the lower-right bowl",
              path: [
                { x: 372, y: 132 }, { x: 402, y: 98 },
                { x: 434, y: 66 }, { x: 470, y: 39 },
                { x: 514, y: 32 }, { x: 559, y: 32 },
                { x: 602, y: 43 }, { x: 637, y: 71 },
                { x: 656, y: 111 }, { x: 659, y: 156 },
                { x: 648, y: 200 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl the upper-right flourish",
              path: [
                { x: 325, y: 350 }, { x: 350, y: 325 },
                { x: 400, y: 305 }, { x: 455, y: 298 },
                { x: 515, y: 302 }, { x: 565, y: 315 },
                { x: 605, y: 340 }, { x: 625, y: 375 },
                { x: 615, y: 410 }, { x: 580, y: 435 },
                { x: 530, y: 450 }, { x: 485, y: 448 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("జ"),
    },
  ],
  [
    "telugu:ఝ",
    {
      script: "telugu",
      glyph: "ఝ",
      strokes: [
        {
          segments: [
            {
              label: "circle around the broad left bowl",
              path: [
                { x: 505, y: 245 },
                { x: 500, y: 315 },
                { x: 470, y: 375 },
                { x: 420, y: 425 },
                { x: 355, y: 455 },
                { x: 290, y: 465 },
                { x: 220, y: 455 },
                { x: 155, y: 425 },
                { x: 105, y: 375 },
                { x: 75, y: 315 },
                { x: 70, y: 245 },
                { x: 78, y: 175 },
                { x: 110, y: 115 },
                { x: 160, y: 70 },
                { x: 225, y: 42 },
                { x: 295, y: 35 },
                { x: 365, y: 48 },
                { x: 425, y: 80 },
                { x: 470, y: 130 },
                { x: 498, y: 190 },
                { x: 505, y: 245 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and circle around the middle bowl",
              path: [
                { x: 750, y: 403 },
                { x: 670, y: 430 },
                { x: 590, y: 403 },
                { x: 531, y: 330 },
                { x: 510, y: 230 },
                { x: 531, y: 130 },
                { x: 590, y: 57 },
                { x: 670, y: 30 },
                { x: 750, y: 57 },
                { x: 809, y: 130 },
                { x: 830, y: 230 },
                { x: 809, y: 330 },
                { x: 750, y: 403 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and circle around the right bowl",
              path: [
                { x: 1100, y: 383 },
                { x: 1020, y: 410 },
                { x: 940, y: 383 },
                { x: 881, y: 310 },
                { x: 860, y: 210 },
                { x: 881, y: 110 },
                { x: 940, y: 37 },
                { x: 1020, y: 10 },
                { x: 1100, y: 37 },
                { x: 1159, y: 110 },
                { x: 1180, y: 210 },
                { x: 1159, y: 310 },
                { x: 1100, y: 383 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and sweep the upper flourish",
              path: [
                { x: 160, y: 565 },
                { x: 175, y: 525 },
                { x: 205, y: 485 },
                { x: 245, y: 460 },
                { x: 285, y: 458 },
                { x: 325, y: 480 },
                { x: 360, y: 520 },
                { x: 395, y: 565 },
                { x: 435, y: 615 },
                { x: 480, y: 660 },
                { x: 530, y: 690 },
                { x: 580, y: 700 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and draw the separate downward stem",
              path: [
                { x: 882, y: 12 },
                { x: 882, y: -32 },
                { x: 882, y: -78 },
                { x: 882, y: -120 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఝ"),
    },
  ],
  [
    "telugu:చ",
    {
      script: "telugu",
      glyph: "చ",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper bar from left to right",
              path: [
                { x: 49, y: 296 }, { x: 88, y: 296 },
                { x: 128, y: 294 }, { x: 166, y: 283 },
                { x: 205, y: 288 },
              ],
            },
            {
              label: "continue down and around the left bowl",
              path: [
                { x: 205, y: 288 }, { x: 166, y: 271 },
                { x: 144, y: 232 }, { x: 118, y: 196 },
                { x: 109, y: 153 }, { x: 112, y: 108 },
                { x: 133, y: 69 }, { x: 168, y: 42 },
                { x: 211, y: 32 }, { x: 256, y: 32 },
                { x: 299, y: 42 }, { x: 334, y: 69 },
                { x: 363, y: 102 }, { x: 385, y: 140 },
              ],
            },
            {
              label: "sweep right and up around the outer bowl",
              path: [
                { x: 385, y: 140 }, { x: 407, y: 101 },
                { x: 438, y: 67 }, { x: 476, y: 40 },
                { x: 521, y: 32 }, { x: 567, y: 32 },
                { x: 611, y: 44 }, { x: 647, y: 72 },
                { x: 670, y: 112 }, { x: 680, y: 156 },
                { x: 681, y: 202 }, { x: 678, y: 248 },
                { x: 665, y: 292 }, { x: 644, y: 333 },
                { x: 615, y: 369 }, { x: 581, y: 400 },
                { x: 542, y: 424 }, { x: 500, y: 444 },
                { x: 461, y: 468 },
              ],
            },
            {
              label: "climb up the flourish's left arm",
              path: [
                { x: 461, y: 468 }, { x: 413, y: 468 },
                { x: 365, y: 469 }, { x: 324, y: 494 },
                { x: 301, y: 536 },
              ],
            },
            {
              label: "cup the upper flourish",
              path: [
                { x: 301, y: 536 }, { x: 322, y: 497 },
                { x: 357, y: 470 }, { x: 401, y: 464 },
                { x: 446, y: 467 }, { x: 475, y: 498 },
                { x: 502, y: 533 }, { x: 530, y: 568 },
                { x: 558, y: 603 }, { x: 589, y: 636 },
                { x: 624, y: 663 }, { x: 665, y: 680 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("చ"),
    },
  ],
  [
    "telugu:ఛ",
    {
      script: "telugu",
      glyph: "ఛ",
      strokes: [
        {
          segments: [
            {
              label: "draw the upper bar from left to right",
              path: [
                { x: 50, y: 295 },
                { x: 100, y: 295 },
                { x: 150, y: 295 },
                { x: 200, y: 295 },
                { x: 250, y: 295 },
              ],
            },
            {
              label: "continue down and around the left bowl",
              path: [
                { x: 250, y: 295 },
                { x: 220, y: 260 },
                { x: 175, y: 235 },
                { x: 130, y: 205 },
                { x: 105, y: 165 },
                { x: 105, y: 120 },
                { x: 125, y: 80 },
                { x: 165, y: 48 },
                { x: 215, y: 30 },
                { x: 265, y: 35 },
                { x: 310, y: 62 },
                { x: 345, y: 105 },
                { x: 370, y: 155 },
                { x: 385, y: 185 },
              ],
            },
            {
              label: "sweep right and up around the outer bowl",
              path: [
                { x: 385, y: 185 },
                { x: 410, y: 135 },
                { x: 450, y: 92 },
                { x: 505, y: 62 },
                { x: 565, y: 55 },
                { x: 620, y: 78 },
                { x: 660, y: 120 },
                { x: 680, y: 180 },
                { x: 675, y: 245 },
                { x: 650, y: 305 },
                { x: 610, y: 355 },
                { x: 560, y: 392 },
                { x: 505, y: 418 },
                { x: 450, y: 432 },
              ],
            },
            {
              label: "continue through the upper flourish",
              path: [
                { x: 450, y: 432 },
                { x: 410, y: 440 },
                { x: 370, y: 460 },
                { x: 335, y: 495 },
                { x: 305, y: 535 },
                { x: 288, y: 570 },
                { x: 325, y: 585 },
                { x: 375, y: 585 },
                { x: 425, y: 570 },
                { x: 470, y: 565 },
                { x: 515, y: 590 },
                { x: 560, y: 625 },
                { x: 610, y: 655 },
                { x: 660, y: 678 },
                { x: 710, y: 686 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and draw the separate downward stem",
              path: [
                { x: 385, y: 12 },
                { x: 385, y: -35 },
                { x: 385, y: -80 },
                { x: 385, y: -120 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఛ"),
    },
  ],
  [
    "telugu:ఘ",
    {
      script: "telugu",
      glyph: "ఘ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left around the upper-left shoulder",
              path: [
                { x: 300, y: 145 },
                { x: 280, y: 195 },
                { x: 245, y: 230 },
                { x: 200, y: 250 },
                { x: 150, y: 248 },
                { x: 110, y: 230 },
                { x: 80, y: 200 },
                { x: 65, y: 165 },
              ],
            },
            {
              label: "continue down round the lower-left bowl",
              path: [
                { x: 65, y: 165 },
                { x: 65, y: 125 },
                { x: 75, y: 85 },
                { x: 105, y: 50 },
                { x: 150, y: 28 },
                { x: 200, y: 25 },
                { x: 245, y: 45 },
                { x: 275, y: 80 },
                { x: 300, y: 125 },
                { x: 300, y: 145 },
              ],
            },
            {
              label: "turn upward around the broad middle arch",
              path: [
                { x: 300, y: 145 },
                { x: 330, y: 105 },
                { x: 370, y: 70 },
                { x: 420, y: 45 },
                { x: 480, y: 30 },
                { x: 535, y: 50 },
                { x: 575, y: 90 },
                { x: 600, y: 145 },
                { x: 607, y: 205 },
                { x: 595, y: 270 },
                { x: 565, y: 325 },
                { x: 520, y: 370 },
                { x: 470, y: 405 },
                { x: 420, y: 430 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep right and up around the outer arch",
              path: [
                { x: 650, y: 145 },
                { x: 690, y: 95 },
                { x: 745, y: 55 },
                { x: 810, y: 30 },
                { x: 870, y: 45 },
                { x: 915, y: 85 },
                { x: 945, y: 145 },
                { x: 950, y: 210 },
                { x: 935, y: 275 },
                { x: 900, y: 330 },
                { x: 855, y: 375 },
                { x: 805, y: 410 },
                { x: 765, y: 430 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and cup the upper flourish",
              path: [
                { x: 98, y: 570 },
                { x: 120, y: 520 },
                { x: 150, y: 480 },
                { x: 190, y: 462 },
                { x: 230, y: 465 },
                { x: 270, y: 490 },
                { x: 310, y: 535 },
                { x: 350, y: 585 },
                { x: 395, y: 630 },
                { x: 445, y: 665 },
                { x: 490, y: 683 },
                { x: 520, y: 686 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and draw the separate downward stem",
              path: [
                { x: 325, y: 15 },
                { x: 325, y: -48 },
                { x: 325, y: -118 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఘ"),
    },
  ],
  [
    "telugu:గ",
    {
      script: "telugu",
      glyph: "గ",
      strokes: [
        {
          segments: [
            {
              label: "sweep up and over the broad lower arch",
              path: [
                { x: 120, y: 12 },
                { x: 88, y: 70 },
                { x: 68, y: 145 },
                { x: 80, y: 230 },
                { x: 105, y: 320 },
                { x: 155, y: 405 },
                { x: 220, y: 455 },
                { x: 295, y: 465 },
                { x: 370, y: 445 },
                { x: 430, y: 398 },
                { x: 472, y: 330 },
                { x: 500, y: 250 },
                { x: 493, y: 170 },
                { x: 470, y: 95 },
                { x: 430, y: 25 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "cup through the separate upper flourish",
              path: [
                { x: 165, y: 560 },
                { x: 190, y: 505 },
                { x: 235, y: 465 },
                { x: 275, y: 468 },
                { x: 320, y: 495 },
                { x: 365, y: 540 },
                { x: 415, y: 595 },
                { x: 470, y: 645 },
                { x: 525, y: 680 },
                { x: 580, y: 685 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("గ"),
    },
  ],
  [
    "telugu:ఖ",
    {
      script: "telugu",
      glyph: "ఖ",
      strokes: [
        {
          segments: [
            {
              label: "circle up around the upper-left bowl",
              path: [
                { x: 285, y: 404 },
                { x: 252, y: 446 },
                { x: 198, y: 468 },
                { x: 135, y: 455 },
                { x: 82, y: 416 },
                { x: 65, y: 365 },
                { x: 86, y: 315 },
                { x: 132, y: 275 },
                { x: 188, y: 270 },
                { x: 240, y: 298 },
                { x: 276, y: 344 },
                { x: 285, y: 404 },
              ],
            },
            {
              label: "descend through the central curve",
              path: [
                { x: 285, y: 404 },
                { x: 325, y: 350 },
                { x: 350, y: 290 },
                { x: 360, y: 225 },
                { x: 350, y: 165 },
                { x: 320, y: 112 },
              ],
            },
            {
              label: "turn up around the left shoulder",
              path: [
                { x: 320, y: 112 },
                { x: 270, y: 78 },
                { x: 210, y: 58 },
                { x: 150, y: 58 },
                { x: 100, y: 78 },
                { x: 68, y: 108 },
                { x: 62, y: 140 },
                { x: 78, y: 170 },
                { x: 112, y: 188 },
                { x: 158, y: 190 },
              ],
            },
            {
              label: "sweep right and up the broad outer bowl",
              path: [
                { x: 158, y: 190 },
                { x: 220, y: 178 },
                { x: 285, y: 145 },
                { x: 350, y: 105 },
                { x: 420, y: 70 },
                { x: 500, y: 42 },
                { x: 570, y: 48 },
                { x: 625, y: 82 },
                { x: 665, y: 140 },
                { x: 675, y: 210 },
                { x: 658, y: 278 },
                { x: 620, y: 338 },
                { x: 565, y: 392 },
                { x: 500, y: 438 },
              ],
            },
            {
              label: "return left along the crown",
              path: [
                { x: 500, y: 438 },
                { x: 455, y: 460 },
                { x: 410, y: 468 },
                { x: 365, y: 456 },
                { x: 325, y: 432 },
                { x: 285, y: 404 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "draw the separate downward stem",
              path: [
                { x: 343, y: 12 },
                { x: 343, y: -48 },
                { x: 343, y: -118 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("ఖ"),
    },
  ],
  [
    "telugu:క",
    {
      script: "telugu",
      glyph: "క",
      strokes: [
        {
          segments: [
            {
              label: "turn down and left around the upper bowl",
              path: [
                { x: 442, y: 357 },
                { x: 410, y: 415 },
                { x: 350, y: 452 },
                { x: 270, y: 466 },
                { x: 190, y: 458 },
                { x: 125, y: 425 },
                { x: 87, y: 375 },
                { x: 78, y: 325 },
                { x: 100, y: 292 },
              ],
            },
            {
              label: "continue right over the middle shoulder",
              path: [
                { x: 100, y: 292 },
                { x: 165, y: 268 },
                { x: 235, y: 255 },
                { x: 310, y: 251 },
                { x: 385, y: 240 },
                { x: 442, y: 218 },
              ],
            },
            {
              label: "curve down and left around the lower bowl",
              path: [
                { x: 442, y: 218 },
                { x: 455, y: 165 },
                { x: 430, y: 112 },
                { x: 380, y: 72 },
                { x: 315, y: 45 },
                { x: 245, y: 31 },
                { x: 175, y: 44 },
                { x: 115, y: 78 },
                { x: 77, y: 125 },
              ],
            },
            {
              label: "finish upward along the left tail",
              path: [
                { x: 77, y: 125 },
                { x: 62, y: 155 },
                { x: 57, y: 190 },
                { x: 62, y: 225 },
                { x: 78, y: 250 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "sweep up through the separate headstroke",
              path: [
                { x: 132, y: 548 },
                { x: 165, y: 505 },
                { x: 215, y: 470 },
                { x: 260, y: 468 },
                { x: 315, y: 492 },
                { x: 365, y: 535 },
                { x: 410, y: 585 },
                { x: 455, y: 635 },
                { x: 510, y: 676 },
                { x: 548, y: 684 },
              ],
            },
          ],
        },
      ],
      source: teluguLetterSource("క"),
    },
  ],
  [
    "telugu:అ",
    {
      script: "telugu",
      glyph: "అ",
      strokes: [
        {
          segments: [
            {
              label: "turn around the left lobe",
              path: [
                { x: 144, y: 256 }, { x: 186, y: 272 },
                { x: 232, y: 273 }, { x: 275, y: 289 },
                { x: 306, y: 323 }, { x: 312, y: 369 },
                { x: 308, y: 414 }, { x: 277, y: 449 },
                { x: 233, y: 460 }, { x: 187, y: 459 },
                { x: 145, y: 440 }, { x: 113, y: 407 },
                { x: 90, y: 367 }, { x: 84, y: 321 },
                { x: 81, y: 275 }, { x: 80, y: 229 },
                { x: 92, y: 184 },
              ],
            },
            {
              label: "sweep around the broad lower bowl",
              path: [
                { x: 92, y: 184 }, { x: 113, y: 143 },
                { x: 143, y: 109 }, { x: 179, y: 80 },
                { x: 219, y: 58 }, { x: 262, y: 43 },
                { x: 307, y: 33 }, { x: 353, y: 28 },
                { x: 399, y: 28 }, { x: 444, y: 29 },
                { x: 490, y: 35 }, { x: 535, y: 43 },
                { x: 578, y: 58 }, { x: 620, y: 78 },
                { x: 655, y: 107 }, { x: 685, y: 142 },
                { x: 706, y: 182 }, { x: 718, y: 227 },
                { x: 724, y: 272 },
              ],
            },
            {
              label: "turn around the right lobe",
              path: [
                { x: 724, y: 272 }, { x: 722, y: 317 },
                { x: 712, y: 360 }, { x: 691, y: 399 },
                { x: 660, y: 431 }, { x: 623, y: 455 },
                { x: 579, y: 460 }, { x: 535, y: 454 },
                { x: 500, y: 427 }, { x: 481, y: 387 },
                { x: 482, y: 342 }, { x: 500, y: 302 },
                { x: 527, y: 266 }, { x: 557, y: 234 },
                { x: 600, y: 224 },
              ],
            },
            {
              label: "return left along the inner bar",
              path: [
                { x: 600, y: 224 }, { x: 557, y: 227 },
                { x: 514, y: 223 }, { x: 471, y: 220 },
                { x: 427, y: 220 }, { x: 384, y: 220 },
                { x: 340, y: 220 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("అ"),
    },
  ],
  [
    "telugu:ఆ",
    {
      script: "telugu",
      glyph: "ఆ",
      strokes: [
        {
          segments: [
            {
              label: "turn round the left lobe and lower bowl",
              path: [
                { x: 142, y: 258 },
                { x: 200, y: 270 },
                { x: 260, y: 310 },
                { x: 315, y: 370 },
                { x: 310, y: 425 },
                { x: 265, y: 468 },
                { x: 220, y: 468 },
                { x: 155, y: 450 },
                { x: 100, y: 395 },
                { x: 72, y: 325 },
                { x: 74, y: 245 },
                { x: 90, y: 180 },
                { x: 125, y: 105 },
                { x: 215, y: 48 },
                { x: 315, y: 24 },
                { x: 420, y: 24 },
                { x: 535, y: 50 },
                { x: 630, y: 105 },
                { x: 700, y: 185 },
                { x: 724, y: 270 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "turn the right lobe, then left along the bar",
              path: [
                { x: 610, y: 220 },
                { x: 610, y: 190 },
                { x: 650, y: 205 },
                { x: 665, y: 205 },
                { x: 675, y: 190 },
                { x: 680, y: 175 },
                { x: 690, y: 220 },
                { x: 700, y: 270 },
                { x: 710, y: 300 },
                { x: 715, y: 360 },
                { x: 680, y: 420 },
                { x: 620, y: 465 },
                { x: 575, y: 465 },
                { x: 530, y: 465 },
                { x: 500, y: 435 },
                { x: 478, y: 395 },
                { x: 478, y: 360 },
                { x: 500, y: 320 },
                { x: 550, y: 285 },
                { x: 575, y: 270 },
                { x: 610, y: 255 },
                { x: 610, y: 220 },
                { x: 520, y: 220 },
                { x: 420, y: 220 },
                { x: 305, y: 220 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఆ"),
    },
  ],
  [
    "telugu:ఇ",
    {
      script: "telugu",
      glyph: "ఇ",
      strokes: [
        {
          segments: [
            {
              label: "turn around the broad outer bowl",
              path: [
                { x: 460, y: 220 },
                { x: 430, y: 220 },
                { x: 330, y: 220 },
                { x: 230, y: 235 },
                { x: 155, y: 210 },
                { x: 112, y: 170 },
                { x: 100, y: 125 },
                { x: 118, y: 80 },
                { x: 175, y: 48 },
                { x: 250, y: 28 },
                { x: 340, y: 25 },
                { x: 430, y: 42 },
                { x: 505, y: 78 },
                { x: 555, y: 125 },
                { x: 585, y: 180 },
                { x: 600, y: 240 },
                { x: 610, y: 300 },
                { x: 615, y: 360 },
                { x: 620, y: 300 },
                { x: 620, y: 220 },
                { x: 590, y: 160 },
                { x: 560, y: 100 },
                { x: 560, y: 40 },
                { x: 570, y: -20 },
                { x: 555, y: -60 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "form the compact upper-left lobe",
              path: [
                { x: 320, y: 390 },
                { x: 275, y: 440 },
                { x: 220, y: 468 },
                { x: 155, y: 465 },
                { x: 100, y: 440 },
                { x: 70, y: 400 },
                { x: 70, y: 355 },
                { x: 92, y: 320 },
                { x: 120, y: 310 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "form the angled upper-right shoulder",
              path: [
                { x: 335, y: 390 },
                { x: 380, y: 438 },
                { x: 430, y: 468 },
                { x: 485, y: 468 },
                { x: 535, y: 445 },
                { x: 575, y: 410 },
                { x: 600, y: 365 },
                { x: 610, y: 315 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఇ"),
    },
  ],
  [
    "telugu:ఉ",
    {
      script: "telugu",
      glyph: "ఉ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left across the rounded upper arch",
              path: [
                { x: 610, y: 410 },
                { x: 550, y: 440 },
                { x: 470, y: 465 },
                { x: 380, y: 468 },
                { x: 285, y: 445 },
                { x: 205, y: 405 },
                { x: 145, y: 350 },
                { x: 110, y: 285 },
              ],
            },
            {
              label: "continue down round the broad lower bowl",
              path: [
                { x: 110, y: 285 },
                { x: 78, y: 215 },
                { x: 78, y: 145 },
                { x: 105, y: 85 },
                { x: 155, y: 45 },
                { x: 215, y: 25 },
                { x: 275, y: 35 },
                { x: 325, y: 70 },
                { x: 365, y: 115 },
                { x: 405, y: 75 },
                { x: 455, y: 42 },
              ],
            },
            {
              label: "curl up round the right lobe, no lift",
              path: [
                { x: 455, y: 42 },
                { x: 520, y: 20 },
                { x: 585, y: 25 },
                { x: 645, y: 55 },
                { x: 685, y: 105 },
                { x: 690, y: 155 },
                { x: 665, y: 205 },
                { x: 620, y: 225 },
                { x: 575, y: 220 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift and draw the inner bar left to right",
              path: [
                { x: 95, y: 282 },
                { x: 205, y: 282 },
                { x: 320, y: 282 },
                { x: 440, y: 282 },
                { x: 560, y: 282 },
                { x: 680, y: 282 },
                { x: 750, y: 282 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, draw the upper headstroke down",
              path: [
                { x: 378, y: 610 },
                { x: 378, y: 570 },
                { x: 378, y: 525 },
                { x: 378, y: 490 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఉ"),
    },
  ],
  [
    "telugu:ఎ",
    {
      script: "telugu",
      glyph: "ఎ",
      strokes: [
        {
          segments: [
            {
              label: "turn down and left round the lower loop",
              path: [
                { x: 273, y: 140 }, { x: 274, y: 183 },
                { x: 247, y: 217 }, { x: 209, y: 238 },
                { x: 166, y: 244 }, { x: 124, y: 233 },
                { x: 91, y: 205 }, { x: 72, y: 166 },
                { x: 69, y: 122 }, { x: 81, y: 80 },
              ],
            },
            {
              label: "round its base and back to the junction",
              path: [
                { x: 81, y: 80 }, { x: 115, y: 47 },
                { x: 158, y: 28 }, { x: 206, y: 29 },
                { x: 249, y: 49 }, { x: 283, y: 83 },
                { x: 313, y: 120 },
              ],
            },
            {
              label: "sweep up the broad outer arch",
              path: [
                { x: 313, y: 120 }, { x: 352, y: 96 },
                { x: 385, y: 65 }, { x: 423, y: 40 },
                { x: 468, y: 32 }, { x: 513, y: 32 },
                { x: 557, y: 42 }, { x: 593, y: 70 },
                { x: 618, y: 108 }, { x: 630, y: 152 },
                { x: 633, y: 198 }, { x: 632, y: 243 },
                { x: 624, y: 288 }, { x: 611, y: 332 },
                { x: 593, y: 374 }, { x: 572, y: 414 },
                { x: 546, y: 452 }, { x: 517, y: 487 },
                { x: 485, y: 520 }, { x: 453, y: 552 },
                { x: 418, y: 581 }, { x: 380, y: 607 },
                { x: 341, y: 631 }, { x: 301, y: 652 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఎ"),
    },
  ],
  [
    "telugu:ఏ",
    {
      script: "telugu",
      glyph: "ఏ",
      strokes: [
        {
          segments: [
            {
              label: "turn down and left round the lower loop",
              path: [
                { x: 275, y: 141 },
                { x: 255, y: 195 },
                { x: 215, y: 235 },
                { x: 170, y: 245 },
                { x: 120, y: 225 },
                { x: 80, y: 180 },
                { x: 68, y: 125 },
                { x: 78, y: 82 },
              ],
            },
            {
              label: "round its base and back to the junction",
              path: [
                { x: 78, y: 82 },
                { x: 105, y: 42 },
                { x: 155, y: 24 },
                { x: 205, y: 30 },
                { x: 245, y: 72 },
                { x: 275, y: 141 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart at the tail, sweep up the outer arch",
              path: [
                { x: 260, y: 655 },
                { x: 345, y: 610 },
                { x: 435, y: 545 },
                { x: 515, y: 465 },
                { x: 575, y: 370 },
                { x: 610, y: 270 },
                { x: 615, y: 180 },
                { x: 585, y: 100 },
                { x: 525, y: 45 },
                { x: 460, y: 28 },
                { x: 390, y: 52 },
                { x: 325, y: 95 },
                { x: 275, y: 141 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "restart and sweep up the upper-left hook",
              path: [
                { x: 210, y: 535 },
                { x: 175, y: 585 },
                { x: 155, y: 650 },
                { x: 155, y: 705 },
                { x: 175, y: 755 },
                { x: 215, y: 790 },
                { x: 260, y: 795 },
                { x: 295, y: 785 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఏ"),
    },
  ],
  [
    "telugu:ఒ",
    {
      script: "telugu",
      glyph: "ఒ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right across the upper arch",
              path: [
                { x: 84, y: 364 }, { x: 88, y: 409 },
                { x: 117, y: 441 }, { x: 159, y: 458 },
                { x: 204, y: 460 }, { x: 249, y: 458 },
                { x: 287, y: 436 }, { x: 313, y: 400 },
                { x: 316, y: 355 }, { x: 300, y: 314 },
                { x: 266, y: 285 }, { x: 226, y: 265 },
                { x: 184, y: 248 },
              ],
            },
            {
              label: "curve down around the left bowl",
              path: [
                { x: 184, y: 248 }, { x: 146, y: 228 },
                { x: 114, y: 198 }, { x: 90, y: 164 },
                { x: 88, y: 120 }, { x: 97, y: 79 },
                { x: 127, y: 48 }, { x: 167, y: 33 },
                { x: 210, y: 32 }, { x: 254, y: 33 },
                { x: 294, y: 48 }, { x: 325, y: 77 },
                { x: 356, y: 108 },
              ],
            },
            {
              label: "sweep right around the broad lower bowl",
              path: [
                { x: 356, y: 108 }, { x: 398, y: 97 },
                { x: 430, y: 66 }, { x: 467, y: 40 },
                { x: 510, y: 32 }, { x: 555, y: 32 },
                { x: 598, y: 41 }, { x: 634, y: 67 },
                { x: 654, y: 107 }, { x: 660, y: 151 },
                { x: 650, y: 195 }, { x: 629, y: 234 },
                { x: 600, y: 268 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఒ"),
    },
  ],
  [
    "telugu:ఐ",
    {
      script: "telugu",
      glyph: "ఐ",
      strokes: [
        {
          segments: [
            {
              label: "sweep left across the compact upper arch",
              path: [
                { x: 324, y: 112 }, { x: 314, y: 155 },
                { x: 286, y: 190 }, { x: 253, y: 220 },
                { x: 214, y: 240 }, { x: 169, y: 243 },
                { x: 127, y: 229 }, { x: 96, y: 198 },
                { x: 80, y: 156 },
              ],
            },
            {
              label: "curve down around the left bowl",
              path: [
                { x: 80, y: 156 }, { x: 82, y: 111 },
                { x: 101, y: 72 }, { x: 134, y: 41 },
                { x: 176, y: 28 }, { x: 220, y: 30 },
                { x: 260, y: 49 }, { x: 292, y: 80 },
                { x: 324, y: 112 },
              ],
            },
            {
              label: "sweep right around the broad lower bowl",
              path: [
                { x: 324, y: 112 }, { x: 363, y: 93 },
                { x: 396, y: 64 }, { x: 433, y: 41 },
                { x: 476, y: 32 }, { x: 521, y: 32 },
                { x: 563, y: 41 }, { x: 597, y: 69 },
                { x: 625, y: 103 }, { x: 640, y: 144 },
                { x: 648, y: 188 },
              ],
            },
            {
              label: "sweep left across the upper-right arch",
              path: [
                { x: 648, y: 188 }, { x: 648, y: 233 },
                { x: 647, y: 278 }, { x: 639, y: 322 },
                { x: 624, y: 365 }, { x: 600, y: 403 },
                { x: 568, y: 434 }, { x: 527, y: 454 },
                { x: 483, y: 460 }, { x: 438, y: 454 },
                { x: 399, y: 433 }, { x: 367, y: 401 },
                { x: 336, y: 368 },
              ],
            },
            {
              label: "sweep left across the upper-left arch",
              path: [
                { x: 336, y: 368 }, { x: 306, y: 399 },
                { x: 276, y: 431 }, { x: 239, y: 453 },
                { x: 196, y: 460 }, { x: 153, y: 458 },
                { x: 115, y: 438 }, { x: 87, y: 406 },
                { x: 80, y: 363 }, { x: 80, y: 320 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఐ"),
    },
  ],
  [
    "telugu:ఋ",
    {
      script: "telugu",
      glyph: "ఋ",
      strokes: [
        {
          segments: [
            {
              label: "sweep right across the upper shoulder",
              path: [
                { x: 84, y: 328 }, { x: 84, y: 373 },
                { x: 91, y: 416 }, { x: 124, y: 446 },
                { x: 167, y: 459 }, { x: 211, y: 463 },
                { x: 255, y: 456 }, { x: 293, y: 433 },
                { x: 314, y: 394 }, { x: 315, y: 350 },
                { x: 292, y: 312 }, { x: 259, y: 282 },
                { x: 219, y: 262 }, { x: 178, y: 245 },
                { x: 138, y: 225 }, { x: 104, y: 196 },
              ],
            },
            {
              label: "curve down around the left bowl",
              path: [
                { x: 104, y: 196 }, { x: 86, y: 156 },
                { x: 85, y: 112 }, { x: 102, y: 72 },
                { x: 136, y: 43 }, { x: 178, y: 30 },
                { x: 222, y: 28 }, { x: 266, y: 35 },
                { x: 305, y: 55 }, { x: 337, y: 85 },
                { x: 372, y: 112 },
              ],
            },
            {
              label: "sweep right around the lower bowl",
              path: [
                { x: 372, y: 112 }, { x: 410, y: 86 },
                { x: 445, y: 53 }, { x: 488, y: 34 },
                { x: 535, y: 32 }, { x: 582, y: 36 },
                { x: 620, y: 64 }, { x: 653, y: 98 },
                { x: 672, y: 140 },
              ],
            },
            {
              label: "curl up around the first right lobe",
              path: [
                { x: 672, y: 140 }, { x: 672, y: 183 },
                { x: 672, y: 226 }, { x: 664, y: 269 },
                { x: 648, y: 309 }, { x: 625, y: 345 },
                { x: 597, y: 378 }, { x: 565, y: 407 },
                { x: 530, y: 432 }, { x: 492, y: 452 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up around the middle lobe",
              path: [
                { x: 688, y: 124 }, { x: 721, y: 95 },
                { x: 755, y: 66 }, { x: 794, y: 44 },
                { x: 836, y: 32 }, { x: 880, y: 32 },
                { x: 924, y: 36 }, { x: 961, y: 61 },
                { x: 992, y: 92 }, { x: 1017, y: 128 },
                { x: 1020, y: 172 }, { x: 1019, y: 216 },
                { x: 1012, y: 260 }, { x: 998, y: 302 },
                { x: 975, y: 340 }, { x: 946, y: 374 },
                { x: 914, y: 404 }, { x: 879, y: 431 },
                { x: 840, y: 452 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "curl up around the final lobe",
              path: [
                { x: 1036, y: 124 }, { x: 1069, y: 94 },
                { x: 1102, y: 65 }, { x: 1140, y: 43 },
                { x: 1183, y: 32 }, { x: 1227, y: 32 },
                { x: 1271, y: 36 }, { x: 1308, y: 60 },
                { x: 1338, y: 92 }, { x: 1357, y: 131 },
                { x: 1364, y: 175 }, { x: 1364, y: 219 },
                { x: 1358, y: 263 }, { x: 1342, y: 304 },
                { x: 1320, y: 342 }, { x: 1292, y: 376 },
                { x: 1259, y: 406 }, { x: 1223, y: 432 },
                { x: 1184, y: 452 },
              ],
            },
          ],
        },
      ],
      source: teluguIndependentVowelSource("ఋ"),
    },
  ],
];
