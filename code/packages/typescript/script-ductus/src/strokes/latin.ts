// ---------------------------------------------------------------------------
// latin.ts — the Latin-script owner of cited pen paths (print letters)
// ---------------------------------------------------------------------------
//
// WHERE THE ORDER COMES FROM. The Grundschrift-App (github.com/Medien-Treibhaus/
// grundschrift-app-source, pinned commit f6dbd80) was made in a research
// project of the Laborschule at Bielefeld University with the
// Grundschulverband, on its Grundschrift school model, with a teacher advisory
// team. For every letter it stores an ordered list of paths, one per pen-down
// stroke, and the app makes a child trace them in order. The repository has no
// licence, so it is a source of FACTS only: the order, where each stroke
// starts, which way it goes and where the pen lifts. No point of it is copied.
//
// Native writers corroborate it: UJIpenchars2 (Prat et al., UCI Machine
// Learning Repository dataset 177, CC BY 4.0) holds 120 tablet pen traces of
// each Spanish character, two by each of 60 adult writers. Its counts and
// shares go into each record's `variation`. It is also the only source for
// the marks the school model lacks: the tilde of ñ (after the n, left to
// right), and the opening marks ¿ and ¡ (hook or bar first, dot last).
//
// THE OUTLINE. No bundled font is a Latin font, but Noto Sans Devanagari 2.006
// carries the Noto Sans Latin letters, including ñ, ß, ¿ and ¡. Every point
// below is fitted to that outline (font units, y up), and
// tests/strokes/latin.test.ts holds the paths to its ink at the default
// tolerances, with no override.
//
// PRINT, NOT JOINED. These are print letters: each stands apart, so a word is
// drawn letter by letter (human-language-data's SEPARATE_LETTER_SCRIPTS).
// Grundschrift ends some strokes with a small exit hook that Noto does not
// print; the path ends where the printed letter ends. Where Noto joins two
// parts that the source draws in one stroke (b's bowl to its stem, g's bowl to
// its stem, the arch of h, n and r), the path runs back along the ink it has
// just drawn instead of lifting, exactly as the one-stroke source does.
//
// LEFT OUT. a: Noto prints a two-storey a, and every source draws the
// one-storey a, so no path can follow a source and lie on the printed letter.
// The grave, the circumflex, the cedilla, æ and œ have no source; ä, ö, ë, ï
// and ÿ would be analogy only. The acute and ü are sourced but are not drawn
// yet, because every lesson that holds one also holds an a.
// ---------------------------------------------------------------------------

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import latin from "../../../../../learning/human-languages/data/scripts/latin.json";

/** The cited source a letter's own inventory row records — one source of truth. */
const latinLetterSource = (glyph: string): StrokeSource => {
  const letter = latin.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Latin ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  // b. 1 stroke: the stem, back up it, the bowl clockwise.
  [
    "latin:b",
    {
      script: "latin",
      glyph: "b",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 129, y: 718 },
                { x: 129, y: 673 },
                { x: 129, y: 628 },
                { x: 129, y: 583 },
                { x: 129, y: 538 },
                { x: 129, y: 492 },
                { x: 129, y: 447 },
                { x: 129, y: 402 },
                { x: 129, y: 357 },
                { x: 129, y: 312 },
                { x: 129, y: 267 },
                { x: 129, y: 222 },
                { x: 129, y: 177 },
                { x: 129, y: 132 },
                { x: 128, y: 87 },
                { x: 121, y: 42 },
              ],
            },
            {
              label: "back up and round the bowl",
              path: [
                { x: 121, y: 42 },
                { x: 128, y: 86 },
                { x: 133, y: 130 },
                { x: 133, y: 175 },
                { x: 129, y: 219 },
                { x: 129, y: 264 },
                { x: 129, y: 308 },
                { x: 130, y: 353 },
                { x: 136, y: 397 },
                { x: 165, y: 428 },
                { x: 200, y: 456 },
                { x: 235, y: 484 },
                { x: 275, y: 502 },
                { x: 320, y: 506 },
                { x: 364, y: 505 },
                { x: 407, y: 493 },
                { x: 444, y: 469 },
                { x: 474, y: 436 },
                { x: 494, y: 396 },
                { x: 506, y: 353 },
                { x: 513, y: 309 },
                { x: 513, y: 265 },
                { x: 512, y: 220 },
                { x: 505, y: 176 },
                { x: 492, y: 134 },
                { x: 471, y: 94 },
                { x: 440, y: 62 },
                { x: 401, y: 40 },
                { x: 358, y: 30 },
                { x: 313, y: 30 },
                { x: 269, y: 35 },
                { x: 229, y: 54 },
                { x: 195, y: 83 },
                { x: 153, y: 94 },
                { x: 141, y: 118 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("b"),
    },
  ],
  // c. 1 stroke from the top right, anticlockwise.
  [
    "latin:c",
    {
      script: "latin",
      glyph: "c",
      strokes: [
        {
          segments: [
            {
              label: "curve round to the left and down",
              path: [
                { x: 395, y: 494 },
                { x: 351, y: 504 },
                { x: 306, y: 506 },
                { x: 261, y: 504 },
                { x: 218, y: 492 },
                { x: 179, y: 469 },
                { x: 148, y: 436 },
                { x: 124, y: 398 },
                { x: 110, y: 355 },
                { x: 103, y: 310 },
                { x: 103, y: 265 },
                { x: 103, y: 220 },
                { x: 110, y: 176 },
                { x: 125, y: 133 },
                { x: 150, y: 96 },
                { x: 182, y: 64 },
                { x: 221, y: 42 },
                { x: 265, y: 31 },
                { x: 310, y: 30 },
                { x: 355, y: 32 },
                { x: 399, y: 42 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("c"),
    },
  ],
  // e. 1 stroke: the bar, then anticlockwise round.
  [
    "latin:e",
    {
      script: "latin",
      glyph: "e",
      strokes: [
        {
          segments: [
            {
              label: "draw the bar to the right",
              path: [
                { x: 107, y: 282 },
                { x: 152, y: 286 },
                { x: 198, y: 286 },
                { x: 243, y: 286 },
                { x: 289, y: 286 },
                { x: 335, y: 286 },
                { x: 380, y: 286 },
                { x: 426, y: 288 },
                { x: 471, y: 294 },
              ],
            },
            {
              label: "curve up, round and down",
              path: [
                { x: 471, y: 294 },
                { x: 467, y: 338 },
                { x: 459, y: 382 },
                { x: 442, y: 423 },
                { x: 413, y: 456 },
                { x: 380, y: 487 },
                { x: 339, y: 504 },
                { x: 295, y: 508 },
                { x: 251, y: 507 },
                { x: 209, y: 492 },
                { x: 174, y: 465 },
                { x: 144, y: 433 },
                { x: 122, y: 394 },
                { x: 109, y: 351 },
                { x: 107, y: 307 },
                { x: 104, y: 263 },
                { x: 103, y: 218 },
                { x: 112, y: 174 },
                { x: 128, y: 133 },
                { x: 153, y: 96 },
                { x: 185, y: 66 },
                { x: 224, y: 44 },
                { x: 267, y: 32 },
                { x: 311, y: 30 },
                { x: 356, y: 30 },
                { x: 400, y: 35 },
                { x: 443, y: 46 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("e"),
    },
  ],
  // g. 1 stroke: the bowl anticlockwise, back up the stem, the stem and tail.
  [
    "latin:g",
    {
      script: "latin",
      glyph: "g",
      strokes: [
        {
          segments: [
            {
              label: "round the bowl from the top right",
              path: [
                { x: 399, y: 492 },
                { x: 355, y: 496 },
                { x: 312, y: 506 },
                { x: 268, y: 508 },
                { x: 225, y: 500 },
                { x: 186, y: 478 },
                { x: 155, y: 447 },
                { x: 130, y: 411 },
                { x: 114, y: 370 },
                { x: 105, y: 326 },
                { x: 103, y: 282 },
                { x: 103, y: 238 },
                { x: 106, y: 194 },
                { x: 117, y: 151 },
                { x: 135, y: 110 },
                { x: 163, y: 76 },
                { x: 196, y: 48 },
                { x: 237, y: 32 },
                { x: 281, y: 28 },
                { x: 326, y: 30 },
                { x: 368, y: 43 },
                { x: 405, y: 67 },
                { x: 438, y: 96 },
                { x: 475, y: 120 },
              ],
            },
            {
              label: "back up, then down and hook left",
              path: [
                { x: 475, y: 120 },
                { x: 482, y: 164 },
                { x: 487, y: 209 },
                { x: 487, y: 254 },
                { x: 487, y: 298 },
                { x: 484, y: 343 },
                { x: 483, y: 388 },
                { x: 483, y: 433 },
                { x: 489, y: 477 },
                { x: 486, y: 453 },
                { x: 483, y: 408 },
                { x: 483, y: 363 },
                { x: 484, y: 318 },
                { x: 487, y: 274 },
                { x: 487, y: 229 },
                { x: 485, y: 184 },
                { x: 483, y: 140 },
                { x: 483, y: 95 },
                { x: 483, y: 50 },
                { x: 483, y: 5 },
                { x: 483, y: -40 },
                { x: 477, y: -84 },
                { x: 460, y: -125 },
                { x: 430, y: -159 },
                { x: 393, y: -183 },
                { x: 351, y: -197 },
                { x: 306, y: -200 },
                { x: 261, y: -200 },
                { x: 217, y: -199 },
                { x: 173, y: -192 },
                { x: 131, y: -176 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("g"),
    },
  ],
  // h. 1 stroke: the stem, back up it, the arch and the right side.
  [
    "latin:h",
    {
      script: "latin",
      glyph: "h",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 129, y: 716 },
                { x: 129, y: 671 },
                { x: 129, y: 626 },
                { x: 129, y: 581 },
                { x: 129, y: 536 },
                { x: 129, y: 491 },
                { x: 129, y: 446 },
                { x: 129, y: 401 },
                { x: 129, y: 355 },
                { x: 129, y: 310 },
                { x: 129, y: 265 },
                { x: 129, y: 220 },
                { x: 129, y: 175 },
                { x: 129, y: 130 },
                { x: 129, y: 85 },
                { x: 129, y: 40 },
              ],
            },
            {
              label: "back up, over and down",
              path: [
                { x: 129, y: 40 },
                { x: 129, y: 84 },
                { x: 129, y: 129 },
                { x: 129, y: 173 },
                { x: 129, y: 218 },
                { x: 129, y: 262 },
                { x: 129, y: 307 },
                { x: 130, y: 351 },
                { x: 137, y: 395 },
                { x: 166, y: 427 },
                { x: 200, y: 455 },
                { x: 235, y: 482 },
                { x: 276, y: 500 },
                { x: 320, y: 504 },
                { x: 364, y: 504 },
                { x: 408, y: 496 },
                { x: 443, y: 470 },
                { x: 473, y: 437 },
                { x: 488, y: 395 },
                { x: 493, y: 351 },
                { x: 493, y: 307 },
                { x: 493, y: 262 },
                { x: 493, y: 218 },
                { x: 493, y: 173 },
                { x: 493, y: 129 },
                { x: 493, y: 84 },
                { x: 493, y: 40 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("h"),
    },
  ],
  // i. 2 strokes: the stem, then the dot.
  [
    "latin:i",
    {
      script: "latin",
      glyph: "i",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 130, y: 492 },
                { x: 130, y: 447 },
                { x: 130, y: 402 },
                { x: 130, y: 356 },
                { x: 130, y: 311 },
                { x: 130, y: 266 },
                { x: 130, y: 221 },
                { x: 130, y: 176 },
                { x: 130, y: 130 },
                { x: 130, y: 85 },
                { x: 130, y: 40 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the dot on top",
              path: [
                { x: 130, y: 708 },
                { x: 130, y: 688 },
                { x: 130, y: 668 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("i"),
    },
  ],
  // l. 1 stroke, top down.
  [
    "latin:l",
    {
      script: "latin",
      glyph: "l",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 129, y: 716 },
                { x: 129, y: 671 },
                { x: 129, y: 626 },
                { x: 129, y: 581 },
                { x: 129, y: 536 },
                { x: 129, y: 491 },
                { x: 129, y: 446 },
                { x: 129, y: 401 },
                { x: 129, y: 355 },
                { x: 129, y: 310 },
                { x: 129, y: 265 },
                { x: 129, y: 220 },
                { x: 129, y: 175 },
                { x: 129, y: 130 },
                { x: 129, y: 85 },
                { x: 129, y: 40 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("l"),
    },
  ],
  // n. 1 stroke: the stem, back up it, the arch and the right side.
  [
    "latin:n",
    {
      script: "latin",
      glyph: "n",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 125, y: 496 },
                { x: 127, y: 451 },
                { x: 129, y: 405 },
                { x: 129, y: 359 },
                { x: 129, y: 314 },
                { x: 129, y: 268 },
                { x: 129, y: 222 },
                { x: 129, y: 177 },
                { x: 129, y: 131 },
                { x: 129, y: 86 },
                { x: 129, y: 40 },
              ],
            },
            {
              label: "back up, over and down",
              path: [
                { x: 129, y: 40 },
                { x: 129, y: 85 },
                { x: 129, y: 129 },
                { x: 129, y: 174 },
                { x: 129, y: 218 },
                { x: 129, y: 263 },
                { x: 129, y: 307 },
                { x: 130, y: 352 },
                { x: 137, y: 396 },
                { x: 167, y: 427 },
                { x: 201, y: 455 },
                { x: 236, y: 483 },
                { x: 276, y: 501 },
                { x: 321, y: 504 },
                { x: 365, y: 504 },
                { x: 409, y: 497 },
                { x: 445, y: 472 },
                { x: 474, y: 438 },
                { x: 488, y: 396 },
                { x: 493, y: 352 },
                { x: 493, y: 307 },
                { x: 493, y: 263 },
                { x: 493, y: 218 },
                { x: 493, y: 174 },
                { x: 493, y: 129 },
                { x: 493, y: 85 },
                { x: 493, y: 40 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("n"),
    },
  ],
  // o. 1 stroke from the top, anticlockwise.
  [
    "latin:o",
    {
      script: "latin",
      glyph: "o",
      strokes: [
        {
          segments: [
            {
              label: "round to the left, back to the top",
              path: [
                { x: 267, y: 506 },
                { x: 224, y: 496 },
                { x: 186, y: 473 },
                { x: 155, y: 442 },
                { x: 128, y: 406 },
                { x: 112, y: 365 },
                { x: 104, y: 321 },
                { x: 103, y: 277 },
                { x: 103, y: 232 },
                { x: 108, y: 189 },
                { x: 122, y: 146 },
                { x: 143, y: 108 },
                { x: 174, y: 75 },
                { x: 209, y: 48 },
                { x: 250, y: 33 },
                { x: 294, y: 30 },
                { x: 339, y: 30 },
                { x: 382, y: 40 },
                { x: 420, y: 63 },
                { x: 451, y: 94 },
                { x: 477, y: 131 },
                { x: 493, y: 172 },
                { x: 502, y: 215 },
                { x: 503, y: 260 },
                { x: 503, y: 304 },
                { x: 498, y: 348 },
                { x: 485, y: 390 },
                { x: 463, y: 429 },
                { x: 432, y: 461 },
                { x: 397, y: 488 },
                { x: 356, y: 503 },
                { x: 311, y: 506 },
                { x: 267, y: 506 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("o"),
    },
  ],
  // r. 1 stroke: the stem, back up it, the shoulder.
  [
    "latin:r",
    {
      script: "latin",
      glyph: "r",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 121, y: 496 },
                { x: 124, y: 451 },
                { x: 129, y: 405 },
                { x: 129, y: 360 },
                { x: 129, y: 314 },
                { x: 129, y: 268 },
                { x: 129, y: 223 },
                { x: 129, y: 177 },
                { x: 129, y: 131 },
                { x: 129, y: 86 },
                { x: 129, y: 40 },
              ],
            },
            {
              label: "back up and curve to the right",
              path: [
                { x: 129, y: 40 },
                { x: 129, y: 86 },
                { x: 129, y: 132 },
                { x: 129, y: 179 },
                { x: 129, y: 225 },
                { x: 129, y: 271 },
                { x: 129, y: 317 },
                { x: 134, y: 363 },
                { x: 158, y: 400 },
                { x: 194, y: 429 },
                { x: 226, y: 461 },
                { x: 264, y: 488 },
                { x: 307, y: 503 },
                { x: 353, y: 508 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("r"),
    },
  ],
  // s. 1 stroke from the top right: anticlockwise, then clockwise.
  [
    "latin:s",
    {
      script: "latin",
      glyph: "s",
      strokes: [
        {
          segments: [
            {
              label: "curve to the left, then the right",
              path: [
                { x: 379, y: 490 },
                { x: 335, y: 499 },
                { x: 291, y: 506 },
                { x: 247, y: 507 },
                { x: 202, y: 507 },
                { x: 160, y: 495 },
                { x: 125, y: 468 },
                { x: 100, y: 431 },
                { x: 99, y: 387 },
                { x: 121, y: 349 },
                { x: 153, y: 318 },
                { x: 193, y: 297 },
                { x: 234, y: 280 },
                { x: 275, y: 263 },
                { x: 316, y: 245 },
                { x: 351, y: 218 },
                { x: 381, y: 185 },
                { x: 391, y: 142 },
                { x: 380, y: 99 },
                { x: 351, y: 66 },
                { x: 314, y: 40 },
                { x: 271, y: 30 },
                { x: 227, y: 26 },
                { x: 182, y: 26 },
                { x: 138, y: 34 },
                { x: 95, y: 46 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("s"),
    },
  ],
  // u. 1 stroke: down, round and up, then the right side down.
  [
    "latin:u",
    {
      script: "latin",
      glyph: "u",
      strokes: [
        {
          segments: [
            {
              label: "down, round and up",
              path: [
                { x: 131, y: 498 },
                { x: 123, y: 455 },
                { x: 123, y: 410 },
                { x: 123, y: 365 },
                { x: 123, y: 320 },
                { x: 123, y: 275 },
                { x: 123, y: 230 },
                { x: 123, y: 185 },
                { x: 127, y: 140 },
                { x: 141, y: 98 },
                { x: 170, y: 63 },
                { x: 208, y: 39 },
                { x: 251, y: 30 },
                { x: 296, y: 30 },
                { x: 341, y: 34 },
                { x: 382, y: 51 },
                { x: 417, y: 80 },
                { x: 452, y: 107 },
                { x: 480, y: 140 },
                { x: 486, y: 184 },
                { x: 487, y: 229 },
                { x: 491, y: 274 },
                { x: 491, y: 319 },
                { x: 491, y: 364 },
                { x: 491, y: 409 },
                { x: 491, y: 454 },
                { x: 495, y: 498 },
              ],
            },
            {
              label: "draw the right side down",
              path: [
                { x: 495, y: 498 },
                { x: 491, y: 452 },
                { x: 491, y: 406 },
                { x: 491, y: 360 },
                { x: 491, y: 314 },
                { x: 491, y: 268 },
                { x: 490, y: 222 },
                { x: 487, y: 176 },
                { x: 487, y: 130 },
                { x: 489, y: 84 },
                { x: 495, y: 38 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("u"),
    },
  ],
  // w. 1 stroke: down, up, down, up.
  [
    "latin:w",
    {
      script: "latin",
      glyph: "w",
      strokes: [
        {
          segments: [
            {
              label: "down, up, down and up",
              path: [
                { x: 63, y: 497 },
                { x: 78, y: 455 },
                { x: 90, y: 412 },
                { x: 101, y: 369 },
                { x: 112, y: 326 },
                { x: 124, y: 283 },
                { x: 136, y: 240 },
                { x: 147, y: 197 },
                { x: 158, y: 154 },
                { x: 168, y: 110 },
                { x: 189, y: 72 },
                { x: 226, y: 58 },
                { x: 251, y: 94 },
                { x: 263, y: 137 },
                { x: 275, y: 180 },
                { x: 288, y: 223 },
                { x: 301, y: 266 },
                { x: 315, y: 308 },
                { x: 328, y: 351 },
                { x: 340, y: 393 },
                { x: 352, y: 436 },
                { x: 376, y: 473 },
                { x: 414, y: 473 },
                { x: 434, y: 434 },
                { x: 446, y: 391 },
                { x: 459, y: 348 },
                { x: 471, y: 306 },
                { x: 484, y: 263 },
                { x: 497, y: 220 },
                { x: 509, y: 178 },
                { x: 522, y: 135 },
                { x: 534, y: 92 },
                { x: 561, y: 58 },
                { x: 598, y: 73 },
                { x: 616, y: 113 },
                { x: 627, y: 156 },
                { x: 639, y: 199 },
                { x: 650, y: 242 },
                { x: 662, y: 285 },
                { x: 673, y: 328 },
                { x: 685, y: 371 },
                { x: 696, y: 414 },
                { x: 708, y: 457 },
                { x: 727, y: 497 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("w"),
    },
  ],
  // ß. 1 stroke: up the stem from the foot, over the top, round the bowls.
  [
    "latin:ß",
    {
      script: "latin",
      glyph: "ß",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem up from the foot",
              path: [
                { x: 129, y: 42 },
                { x: 129, y: 88 },
                { x: 129, y: 134 },
                { x: 129, y: 181 },
                { x: 129, y: 227 },
                { x: 129, y: 273 },
                { x: 129, y: 319 },
                { x: 129, y: 365 },
                { x: 129, y: 411 },
                { x: 129, y: 458 },
                { x: 129, y: 504 },
                { x: 129, y: 550 },
              ],
            },
            {
              label: "over the top and round the bowls",
              path: [
                { x: 129, y: 550 },
                { x: 129, y: 595 },
                { x: 138, y: 638 },
                { x: 165, y: 674 },
                { x: 198, y: 704 },
                { x: 239, y: 721 },
                { x: 283, y: 726 },
                { x: 328, y: 726 },
                { x: 372, y: 722 },
                { x: 415, y: 708 },
                { x: 450, y: 681 },
                { x: 472, y: 642 },
                { x: 473, y: 598 },
                { x: 458, y: 557 },
                { x: 427, y: 524 },
                { x: 395, y: 492 },
                { x: 364, y: 460 },
                { x: 342, y: 422 },
                { x: 359, y: 382 },
                { x: 383, y: 344 },
                { x: 415, y: 313 },
                { x: 457, y: 301 },
                { x: 482, y: 266 },
                { x: 513, y: 233 },
                { x: 535, y: 194 },
                { x: 537, y: 149 },
                { x: 535, y: 105 },
                { x: 513, y: 67 },
                { x: 477, y: 40 },
                { x: 434, y: 30 },
                { x: 389, y: 30 },
                { x: 344, y: 31 },
                { x: 301, y: 42 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("ß"),
    },
  ],
  // ñ. 2 strokes: the n as above, then the tilde left to right (UJIpenchars2).
  [
    "latin:ñ",
    {
      script: "latin",
      glyph: "ñ",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 125, y: 496 },
                { x: 127, y: 451 },
                { x: 129, y: 405 },
                { x: 129, y: 359 },
                { x: 129, y: 314 },
                { x: 129, y: 268 },
                { x: 129, y: 222 },
                { x: 129, y: 177 },
                { x: 129, y: 131 },
                { x: 129, y: 86 },
                { x: 129, y: 40 },
              ],
            },
            {
              label: "back up, over and down",
              path: [
                { x: 129, y: 40 },
                { x: 129, y: 85 },
                { x: 129, y: 129 },
                { x: 129, y: 174 },
                { x: 129, y: 218 },
                { x: 129, y: 263 },
                { x: 129, y: 307 },
                { x: 130, y: 352 },
                { x: 137, y: 396 },
                { x: 167, y: 427 },
                { x: 201, y: 455 },
                { x: 236, y: 483 },
                { x: 276, y: 501 },
                { x: 321, y: 504 },
                { x: 365, y: 504 },
                { x: 409, y: 497 },
                { x: 445, y: 472 },
                { x: 474, y: 438 },
                { x: 488, y: 396 },
                { x: 493, y: 352 },
                { x: 493, y: 307 },
                { x: 493, y: 263 },
                { x: 493, y: 218 },
                { x: 493, y: 174 },
                { x: 493, y: 129 },
                { x: 493, y: 85 },
                { x: 493, y: 40 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the tilde, left to right",
              path: [
                { x: 173, y: 668 },
                { x: 209, y: 694 },
                { x: 254, y: 696 },
                { x: 295, y: 680 },
                { x: 335, y: 658 },
                { x: 377, y: 644 },
                { x: 420, y: 653 },
                { x: 453, y: 684 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("ñ"),
    },
  ],
  // G. 1 stroke from the top right, anticlockwise, in along the bar.
  [
    "latin:G",
    {
      script: "latin",
      glyph: "G",
      strokes: [
        {
          segments: [
            {
              label: "curve round from the top right",
              path: [
                { x: 589, y: 658 },
                { x: 545, y: 670 },
                { x: 499, y: 678 },
                { x: 454, y: 682 },
                { x: 408, y: 686 },
                { x: 362, y: 680 },
                { x: 318, y: 670 },
                { x: 275, y: 654 },
                { x: 235, y: 631 },
                { x: 199, y: 603 },
                { x: 170, y: 567 },
                { x: 145, y: 528 },
                { x: 128, y: 486 },
                { x: 116, y: 441 },
                { x: 110, y: 396 },
                { x: 109, y: 350 },
              ],
            },
            {
              label: "round the bottom, up and in",
              path: [
                { x: 109, y: 350 },
                { x: 110, y: 305 },
                { x: 117, y: 261 },
                { x: 128, y: 218 },
                { x: 145, y: 177 },
                { x: 168, y: 139 },
                { x: 198, y: 106 },
                { x: 232, y: 77 },
                { x: 271, y: 56 },
                { x: 313, y: 42 },
                { x: 357, y: 34 },
                { x: 402, y: 30 },
                { x: 446, y: 30 },
                { x: 491, y: 32 },
                { x: 535, y: 38 },
                { x: 578, y: 49 },
                { x: 607, y: 80 },
                { x: 609, y: 125 },
                { x: 609, y: 170 },
                { x: 609, y: 214 },
                { x: 609, y: 259 },
                { x: 607, y: 303 },
                { x: 579, y: 332 },
                { x: 534, y: 334 },
                { x: 490, y: 338 },
                { x: 445, y: 338 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("G"),
    },
  ],
  // ¿. 2 strokes: the hook, then the dot (UJIpenchars2).
  [
    "latin:¿",
    {
      script: "latin",
      glyph: "¿",
      strokes: [
        {
          segments: [
            {
              label: "draw the hook down and round",
              path: [
                { x: 206, y: 296 },
                { x: 197, y: 251 },
                { x: 175, y: 211 },
                { x: 143, y: 179 },
                { x: 109, y: 147 },
                { x: 77, y: 115 },
                { x: 50, y: 78 },
                { x: 35, y: 35 },
                { x: 34, y: -11 },
                { x: 36, y: -56 },
                { x: 56, y: -97 },
                { x: 90, y: -128 },
                { x: 131, y: -146 },
                { x: 177, y: -152 },
                { x: 222, y: -150 },
                { x: 267, y: -140 },
                { x: 310, y: -124 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the dot on top",
              path: [
                { x: 198, y: 504 },
                { x: 198, y: 482 },
                { x: 198, y: 460 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("¿"),
    },
  ],
  // ¡. 2 strokes: the bar down, then the dot (UJIpenchars2).
  [
    "latin:¡",
    {
      script: "latin",
      glyph: "¡",
      strokes: [
        {
          segments: [
            {
              label: "draw the stroke down",
              path: [
                { x: 132, y: 298 },
                { x: 132, y: 254 },
                { x: 132, y: 210 },
                { x: 132, y: 166 },
                { x: 132, y: 121 },
                { x: 132, y: 77 },
                { x: 132, y: 33 },
                { x: 132, y: -11 },
                { x: 132, y: -55 },
                { x: 132, y: -99 },
                { x: 124, y: -142 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the dot on top",
              path: [
                { x: 124, y: 506 },
                { x: 124, y: 484 },
                { x: 124, y: 462 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("¡"),
    },
  ],
];
