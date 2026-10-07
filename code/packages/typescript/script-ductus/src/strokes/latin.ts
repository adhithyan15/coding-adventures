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
// THE OUTLINE. The paths are fitted to LatinPrint-Subset.ttf, a renamed
// subset of SIL Global's literacy typeface Andika 7.000 (OFL-1.1; see the
// _fonts README and subset-latin.sh). It is used because its DEFAULT a is the
// one-storey a, the letter every source teaches. Noto Sans, the other Latin
// outline bundled, prints a two-storey a, which no source draws:
//
//     one-storey a (Andika; the sources)    two-storey a (Noto Sans)
//            .--.|                                .--.
//           /    |                                    \
//          |     |                               .---.|
//           \    |                              (     |
//            '--'|                               '---'|
//
// The books' own text is set in another face; the strip shows the handwriting
// model's letter. Every point below is in that font's units (1000 to the em,
// y up), and tests/strokes/latin.test.ts holds the paths to its ink at the
// default tolerances, with no override.
//
// PRINT, NOT JOINED. These are print letters: each stands apart, so a word is
// drawn letter by letter (human-language-data's SEPARATE_LETTER_SCRIPTS).
// Grundschrift ends some strokes with a small exit hook that the outline does
// not print; the path ends where the printed letter ends. Where the outline
// joins two parts that the source draws in one stroke (b's and p's bowls to
// their stems; the bowls of a, d, g and q to their stems; the arch of h, n and
// r), the path runs back along the ink it has just drawn instead of lifting,
// exactly as the one-stroke source does.
//
// MARKS. A precomposed letter with a mark (ñ, á é í ó ú, ü) is its own entry:
// the base letter's cited path, a lift, then the mark, drawn after the letter
// as most of UJIpenchars2's writers draw it. The acute goes up to the right
// (about six in ten of those writers; three in ten go down to the left, which
// each record's variation says); ü's dots go left, then right.
//
// LEFT OUT. The grave, the circumflex, the cedilla, the macron, æ and œ have
// no source; ä, ö, ë, ï and ÿ would follow ü only by analogy.
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
                { x: 115, y: 736 },
                { x: 115, y: 691 },
                { x: 115, y: 647 },
                { x: 115, y: 602 },
                { x: 115, y: 557 },
                { x: 115, y: 513 },
                { x: 115, y: 468 },
                { x: 115, y: 424 },
                { x: 121, y: 379 },
                { x: 123, y: 335 },
                { x: 117, y: 291 },
                { x: 115, y: 246 },
                { x: 115, y: 202 },
                { x: 115, y: 157 },
                { x: 115, y: 112 },
                { x: 119, y: 68 },
              ],
            },
            {
              label: "back up and round the bowl",
              path: [
                { x: 119, y: 68 },
                { x: 115, y: 113 },
                { x: 115, y: 158 },
                { x: 115, y: 203 },
                { x: 115, y: 248 },
                { x: 117, y: 293 },
                { x: 125, y: 337 },
                { x: 157, y: 367 },
                { x: 193, y: 394 },
                { x: 227, y: 423 },
                { x: 265, y: 447 },
                { x: 307, y: 462 },
                { x: 352, y: 462 },
                { x: 393, y: 445 },
                { x: 425, y: 413 },
                { x: 451, y: 377 },
                { x: 465, y: 334 },
                { x: 471, y: 289 },
                { x: 472, y: 244 },
                { x: 470, y: 199 },
                { x: 458, y: 155 },
                { x: 439, y: 115 },
                { x: 409, y: 82 },
                { x: 375, y: 51 },
                { x: 334, y: 33 },
                { x: 289, y: 28 },
                { x: 244, y: 28 },
                { x: 200, y: 35 },
                { x: 157, y: 48 },
                { x: 119, y: 72 },
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
                { x: 394, y: 448 },
                { x: 350, y: 461 },
                { x: 304, y: 464 },
                { x: 259, y: 463 },
                { x: 215, y: 451 },
                { x: 175, y: 427 },
                { x: 143, y: 395 },
                { x: 118, y: 356 },
                { x: 102, y: 313 },
                { x: 98, y: 268 },
                { x: 98, y: 222 },
                { x: 100, y: 176 },
                { x: 116, y: 133 },
                { x: 145, y: 97 },
                { x: 177, y: 65 },
                { x: 216, y: 41 },
                { x: 261, y: 32 },
                { x: 306, y: 29 },
                { x: 352, y: 34 },
                { x: 394, y: 52 },
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
                { x: 102, y: 276 },
                { x: 150, y: 272 },
                { x: 198, y: 272 },
                { x: 246, y: 272 },
                { x: 294, y: 272 },
                { x: 342, y: 272 },
                { x: 390, y: 272 },
                { x: 434, y: 288 },
              ],
            },
            {
              label: "curve up, round and down",
              path: [
                { x: 434, y: 288 },
                { x: 434, y: 333 },
                { x: 425, y: 377 },
                { x: 403, y: 417 },
                { x: 369, y: 447 },
                { x: 327, y: 464 },
                { x: 282, y: 468 },
                { x: 237, y: 467 },
                { x: 194, y: 454 },
                { x: 158, y: 427 },
                { x: 129, y: 393 },
                { x: 108, y: 352 },
                { x: 102, y: 308 },
                { x: 98, y: 263 },
                { x: 94, y: 218 },
                { x: 99, y: 173 },
                { x: 116, y: 131 },
                { x: 145, y: 97 },
                { x: 178, y: 65 },
                { x: 216, y: 41 },
                { x: 260, y: 32 },
                { x: 305, y: 32 },
                { x: 351, y: 33 },
                { x: 395, y: 42 },
                { x: 438, y: 56 },
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
                { x: 431, y: 435 },
                { x: 390, y: 452 },
                { x: 348, y: 462 },
                { x: 304, y: 463 },
                { x: 260, y: 463 },
                { x: 217, y: 451 },
                { x: 179, y: 429 },
                { x: 149, y: 397 },
                { x: 126, y: 359 },
                { x: 110, y: 319 },
                { x: 102, y: 275 },
                { x: 99, y: 231 },
                { x: 99, y: 187 },
                { x: 105, y: 144 },
                { x: 121, y: 103 },
                { x: 149, y: 70 },
                { x: 187, y: 47 },
                { x: 230, y: 39 },
                { x: 273, y: 48 },
                { x: 313, y: 67 },
                { x: 348, y: 93 },
                { x: 381, y: 122 },
                { x: 423, y: 135 },
              ],
            },
            {
              label: "back up, then down and hook left",
              path: [
                { x: 423, y: 135 },
                { x: 434, y: 179 },
                { x: 435, y: 225 },
                { x: 435, y: 270 },
                { x: 439, y: 316 },
                { x: 439, y: 361 },
                { x: 437, y: 407 },
                { x: 435, y: 400 },
                { x: 438, y: 355 },
                { x: 439, y: 309 },
                { x: 439, y: 264 },
                { x: 439, y: 218 },
                { x: 439, y: 173 },
                { x: 439, y: 127 },
                { x: 439, y: 82 },
                { x: 439, y: 36 },
                { x: 439, y: -10 },
                { x: 433, y: -55 },
                { x: 416, y: -97 },
                { x: 386, y: -130 },
                { x: 353, y: -162 },
                { x: 312, y: -181 },
                { x: 267, y: -185 },
                { x: 222, y: -184 },
                { x: 178, y: -173 },
                { x: 135, y: -157 },
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
                { x: 115, y: 736 },
                { x: 115, y: 690 },
                { x: 115, y: 644 },
                { x: 115, y: 597 },
                { x: 115, y: 551 },
                { x: 115, y: 505 },
                { x: 115, y: 459 },
                { x: 116, y: 413 },
                { x: 122, y: 367 },
                { x: 121, y: 321 },
                { x: 115, y: 275 },
                { x: 115, y: 229 },
                { x: 115, y: 183 },
                { x: 115, y: 136 },
                { x: 115, y: 90 },
                { x: 115, y: 44 },
              ],
            },
            {
              label: "back up, over and down",
              path: [
                { x: 115, y: 44 },
                { x: 115, y: 89 },
                { x: 115, y: 134 },
                { x: 115, y: 180 },
                { x: 115, y: 225 },
                { x: 115, y: 270 },
                { x: 120, y: 315 },
                { x: 136, y: 356 },
                { x: 176, y: 378 },
                { x: 208, y: 409 },
                { x: 244, y: 437 },
                { x: 285, y: 456 },
                { x: 329, y: 464 },
                { x: 374, y: 459 },
                { x: 413, y: 437 },
                { x: 442, y: 403 },
                { x: 455, y: 360 },
                { x: 462, y: 315 },
                { x: 463, y: 270 },
                { x: 463, y: 225 },
                { x: 463, y: 180 },
                { x: 463, y: 134 },
                { x: 463, y: 89 },
                { x: 463, y: 44 },
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
                { x: 136, y: 452 },
                { x: 136, y: 407 },
                { x: 136, y: 361 },
                { x: 136, y: 316 },
                { x: 136, y: 271 },
                { x: 140, y: 225 },
                { x: 140, y: 180 },
                { x: 140, y: 135 },
                { x: 140, y: 89 },
                { x: 140, y: 44 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the dot on top",
              path: [
                { x: 136, y: 692 },
                { x: 136, y: 658 },
                { x: 136, y: 624 },
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
                { x: 135, y: 736 },
                { x: 135, y: 690 },
                { x: 135, y: 644 },
                { x: 135, y: 598 },
                { x: 135, y: 551 },
                { x: 137, y: 505 },
                { x: 139, y: 459 },
                { x: 139, y: 413 },
                { x: 139, y: 367 },
                { x: 139, y: 321 },
                { x: 139, y: 275 },
                { x: 139, y: 229 },
                { x: 139, y: 182 },
                { x: 139, y: 136 },
                { x: 139, y: 90 },
                { x: 139, y: 44 },
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
                { x: 107, y: 456 },
                { x: 113, y: 410 },
                { x: 122, y: 364 },
                { x: 123, y: 318 },
                { x: 123, y: 272 },
                { x: 123, y: 225 },
                { x: 123, y: 179 },
                { x: 123, y: 132 },
                { x: 123, y: 86 },
                { x: 119, y: 40 },
              ],
            },
            {
              label: "back up, over and down",
              path: [
                { x: 119, y: 40 },
                { x: 119, y: 85 },
                { x: 119, y: 131 },
                { x: 119, y: 176 },
                { x: 119, y: 221 },
                { x: 119, y: 266 },
                { x: 126, y: 311 },
                { x: 142, y: 352 },
                { x: 179, y: 377 },
                { x: 212, y: 409 },
                { x: 247, y: 436 },
                { x: 288, y: 456 },
                { x: 333, y: 464 },
                { x: 378, y: 459 },
                { x: 417, y: 438 },
                { x: 446, y: 403 },
                { x: 460, y: 360 },
                { x: 467, y: 316 },
                { x: 467, y: 270 },
                { x: 467, y: 225 },
                { x: 467, y: 180 },
                { x: 467, y: 135 },
                { x: 467, y: 89 },
                { x: 467, y: 44 },
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
                { x: 278, y: 468 },
                { x: 234, y: 463 },
                { x: 193, y: 445 },
                { x: 161, y: 415 },
                { x: 130, y: 383 },
                { x: 108, y: 344 },
                { x: 96, y: 301 },
                { x: 94, y: 257 },
                { x: 94, y: 212 },
                { x: 101, y: 168 },
                { x: 116, y: 126 },
                { x: 141, y: 90 },
                { x: 173, y: 59 },
                { x: 212, y: 38 },
                { x: 256, y: 29 },
                { x: 300, y: 29 },
                { x: 343, y: 40 },
                { x: 379, y: 65 },
                { x: 411, y: 97 },
                { x: 440, y: 131 },
                { x: 456, y: 172 },
                { x: 462, y: 216 },
                { x: 462, y: 261 },
                { x: 460, y: 305 },
                { x: 449, y: 348 },
                { x: 429, y: 388 },
                { x: 398, y: 420 },
                { x: 364, y: 449 },
                { x: 322, y: 463 },
                { x: 278, y: 468 },
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
                { x: 107, y: 456 },
                { x: 113, y: 410 },
                { x: 122, y: 364 },
                { x: 123, y: 318 },
                { x: 123, y: 272 },
                { x: 123, y: 225 },
                { x: 123, y: 179 },
                { x: 123, y: 132 },
                { x: 123, y: 86 },
                { x: 119, y: 40 },
              ],
            },
            {
              label: "back up and over to the right",
              path: [
                { x: 119, y: 40 },
                { x: 119, y: 84 },
                { x: 119, y: 128 },
                { x: 119, y: 172 },
                { x: 119, y: 215 },
                { x: 119, y: 259 },
                { x: 124, y: 303 },
                { x: 135, y: 345 },
                { x: 170, y: 371 },
                { x: 201, y: 402 },
                { x: 233, y: 431 },
                { x: 272, y: 453 },
                { x: 315, y: 456 },
                { x: 354, y: 440 },
                { x: 379, y: 404 },
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
                { x: 366, y: 448 },
                { x: 323, y: 458 },
                { x: 279, y: 467 },
                { x: 234, y: 468 },
                { x: 190, y: 461 },
                { x: 152, y: 438 },
                { x: 122, y: 405 },
                { x: 114, y: 362 },
                { x: 124, y: 320 },
                { x: 156, y: 288 },
                { x: 196, y: 269 },
                { x: 239, y: 257 },
                { x: 282, y: 245 },
                { x: 323, y: 227 },
                { x: 356, y: 198 },
                { x: 381, y: 161 },
                { x: 381, y: 117 },
                { x: 358, y: 80 },
                { x: 325, y: 49 },
                { x: 284, y: 33 },
                { x: 239, y: 32 },
                { x: 195, y: 33 },
                { x: 151, y: 42 },
                { x: 110, y: 60 },
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
                { x: 118, y: 452 },
                { x: 118, y: 408 },
                { x: 118, y: 363 },
                { x: 118, y: 319 },
                { x: 118, y: 274 },
                { x: 120, y: 230 },
                { x: 122, y: 185 },
                { x: 126, y: 141 },
                { x: 139, y: 99 },
                { x: 169, y: 65 },
                { x: 203, y: 38 },
                { x: 248, y: 36 },
                { x: 292, y: 39 },
                { x: 331, y: 60 },
                { x: 365, y: 88 },
                { x: 397, y: 119 },
                { x: 433, y: 146 },
                { x: 451, y: 185 },
                { x: 454, y: 230 },
                { x: 454, y: 274 },
                { x: 454, y: 319 },
                { x: 454, y: 363 },
                { x: 454, y: 408 },
                { x: 454, y: 452 },
              ],
            },
            {
              label: "draw the right side down",
              path: [
                { x: 454, y: 452 },
                { x: 454, y: 407 },
                { x: 454, y: 363 },
                { x: 454, y: 318 },
                { x: 454, y: 273 },
                { x: 454, y: 229 },
                { x: 454, y: 184 },
                { x: 455, y: 139 },
                { x: 465, y: 96 },
                { x: 474, y: 52 },
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
                { x: 78, y: 456 },
                { x: 91, y: 413 },
                { x: 103, y: 369 },
                { x: 115, y: 326 },
                { x: 127, y: 282 },
                { x: 138, y: 238 },
                { x: 150, y: 195 },
                { x: 162, y: 151 },
                { x: 174, y: 108 },
                { x: 197, y: 69 },
                { x: 234, y: 85 },
                { x: 258, y: 123 },
                { x: 271, y: 166 },
                { x: 285, y: 209 },
                { x: 299, y: 252 },
                { x: 313, y: 295 },
                { x: 327, y: 338 },
                { x: 341, y: 381 },
                { x: 363, y: 420 },
                { x: 398, y: 415 },
                { x: 417, y: 374 },
                { x: 431, y: 331 },
                { x: 445, y: 288 },
                { x: 459, y: 245 },
                { x: 473, y: 202 },
                { x: 486, y: 159 },
                { x: 501, y: 116 },
                { x: 520, y: 76 },
                { x: 556, y: 71 },
                { x: 582, y: 107 },
                { x: 594, y: 151 },
                { x: 605, y: 195 },
                { x: 618, y: 238 },
                { x: 630, y: 282 },
                { x: 642, y: 325 },
                { x: 653, y: 369 },
                { x: 666, y: 413 },
                { x: 678, y: 456 },
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
                { x: 119, y: 48 },
                { x: 121, y: 95 },
                { x: 123, y: 142 },
                { x: 123, y: 189 },
                { x: 123, y: 236 },
                { x: 123, y: 282 },
                { x: 123, y: 329 },
                { x: 123, y: 376 },
                { x: 123, y: 423 },
                { x: 123, y: 470 },
                { x: 123, y: 517 },
                { x: 123, y: 564 },
              ],
            },
            {
              label: "over the top and round the bowls",
              path: [
                { x: 123, y: 564 },
                { x: 129, y: 609 },
                { x: 143, y: 652 },
                { x: 171, y: 688 },
                { x: 204, y: 720 },
                { x: 245, y: 740 },
                { x: 289, y: 749 },
                { x: 335, y: 751 },
                { x: 379, y: 741 },
                { x: 420, y: 722 },
                { x: 453, y: 690 },
                { x: 478, y: 652 },
                { x: 483, y: 607 },
                { x: 468, y: 564 },
                { x: 437, y: 531 },
                { x: 400, y: 505 },
                { x: 364, y: 477 },
                { x: 335, y: 442 },
                { x: 343, y: 399 },
                { x: 355, y: 355 },
                { x: 386, y: 322 },
                { x: 426, y: 304 },
                { x: 463, y: 288 },
                { x: 488, y: 251 },
                { x: 518, y: 216 },
                { x: 527, y: 172 },
                { x: 527, y: 126 },
                { x: 508, y: 86 },
                { x: 476, y: 54 },
                { x: 435, y: 34 },
                { x: 390, y: 32 },
                { x: 344, y: 34 },
                { x: 299, y: 40 },
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
                { x: 107, y: 456 },
                { x: 113, y: 410 },
                { x: 122, y: 364 },
                { x: 123, y: 318 },
                { x: 123, y: 272 },
                { x: 123, y: 225 },
                { x: 123, y: 179 },
                { x: 123, y: 132 },
                { x: 123, y: 86 },
                { x: 119, y: 40 },
              ],
            },
            {
              label: "back up, over and down",
              path: [
                { x: 119, y: 40 },
                { x: 119, y: 85 },
                { x: 119, y: 131 },
                { x: 119, y: 176 },
                { x: 119, y: 221 },
                { x: 119, y: 266 },
                { x: 126, y: 311 },
                { x: 142, y: 352 },
                { x: 179, y: 377 },
                { x: 212, y: 409 },
                { x: 247, y: 436 },
                { x: 288, y: 456 },
                { x: 333, y: 464 },
                { x: 378, y: 459 },
                { x: 417, y: 438 },
                { x: 446, y: 403 },
                { x: 460, y: 360 },
                { x: 467, y: 316 },
                { x: 467, y: 270 },
                { x: 467, y: 225 },
                { x: 467, y: 180 },
                { x: 467, y: 135 },
                { x: 467, y: 89 },
                { x: 467, y: 44 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the tilde, left to right",
              path: [
                { x: 167, y: 644 },
                { x: 204, y: 666 },
                { x: 247, y: 664 },
                { x: 287, y: 648 },
                { x: 328, y: 632 },
                { x: 370, y: 624 },
                { x: 411, y: 636 },
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
                { x: 558, y: 640 },
                { x: 517, y: 660 },
                { x: 474, y: 673 },
                { x: 429, y: 680 },
                { x: 384, y: 684 },
                { x: 338, y: 680 },
                { x: 294, y: 671 },
                { x: 252, y: 654 },
                { x: 213, y: 630 },
                { x: 181, y: 599 },
                { x: 151, y: 564 },
                { x: 129, y: 525 },
                { x: 112, y: 482 },
                { x: 101, y: 438 },
                { x: 98, y: 393 },
                { x: 94, y: 348 },
              ],
            },
            {
              label: "round the bottom, up and in",
              path: [
                { x: 94, y: 348 },
                { x: 95, y: 303 },
                { x: 103, y: 260 },
                { x: 114, y: 216 },
                { x: 131, y: 175 },
                { x: 154, y: 137 },
                { x: 185, y: 105 },
                { x: 218, y: 75 },
                { x: 256, y: 53 },
                { x: 299, y: 39 },
                { x: 343, y: 32 },
                { x: 387, y: 32 },
                { x: 432, y: 36 },
                { x: 475, y: 47 },
                { x: 516, y: 62 },
                { x: 554, y: 86 },
                { x: 574, y: 124 },
                { x: 574, y: 169 },
                { x: 578, y: 213 },
                { x: 578, y: 258 },
                { x: 570, y: 301 },
                { x: 531, y: 319 },
                { x: 487, y: 322 },
                { x: 443, y: 324 },
                { x: 398, y: 324 },
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
                { x: 256, y: 296 },
                { x: 254, y: 252 },
                { x: 250, y: 209 },
                { x: 241, y: 166 },
                { x: 223, y: 126 },
                { x: 198, y: 91 },
                { x: 169, y: 57 },
                { x: 144, y: 22 },
                { x: 128, y: -19 },
                { x: 128, y: -62 },
                { x: 136, y: -105 },
                { x: 167, y: -135 },
                { x: 208, y: -151 },
                { x: 251, y: -156 },
                { x: 294, y: -154 },
                { x: 336, y: -140 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the dot on top",
              path: [
                { x: 256, y: 508 },
                { x: 256, y: 476 },
                { x: 256, y: 444 },
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
                { x: 175, y: 276 },
                { x: 175, y: 229 },
                { x: 175, y: 183 },
                { x: 175, y: 136 },
                { x: 175, y: 89 },
                { x: 175, y: 43 },
                { x: 175, y: -4 },
                { x: 175, y: -51 },
                { x: 175, y: -97 },
                { x: 175, y: -144 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the dot on top",
              path: [
                { x: 175, y: 508 },
                { x: 175, y: 476 },
                { x: 175, y: 444 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("¡"),
    },
  ],
  // a. 1 stroke: the bowl anticlockwise, back up the stem, then down it.
  [
    "latin:a",
    {
      script: "latin",
      glyph: "a",
      strokes: [
        {
          segments: [
            {
              label: "round the bowl from the top right",
              path: [
                { x: 431, y: 448 },
                { x: 388, y: 462 },
                { x: 344, y: 468 },
                { x: 299, y: 468 },
                { x: 254, y: 463 },
                { x: 212, y: 447 },
                { x: 178, y: 418 },
                { x: 147, y: 385 },
                { x: 125, y: 346 },
                { x: 111, y: 304 },
                { x: 101, y: 260 },
                { x: 99, y: 215 },
                { x: 99, y: 170 },
                { x: 106, y: 126 },
                { x: 123, y: 84 },
                { x: 152, y: 50 },
                { x: 193, y: 33 },
                { x: 237, y: 36 },
                { x: 276, y: 58 },
                { x: 308, y: 90 },
                { x: 335, y: 126 },
                { x: 357, y: 165 },
                { x: 380, y: 204 },
                { x: 411, y: 236 },
                { x: 427, y: 276 },
              ],
            },
            {
              label: "back up, then down to the foot",
              path: [
                { x: 427, y: 276 },
                { x: 434, y: 322 },
                { x: 435, y: 368 },
                { x: 435, y: 414 },
                { x: 435, y: 418 },
                { x: 435, y: 371 },
                { x: 435, y: 325 },
                { x: 429, y: 279 },
                { x: 436, y: 234 },
                { x: 447, y: 190 },
                { x: 447, y: 144 },
                { x: 451, y: 98 },
                { x: 459, y: 52 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("a"),
    },
  ],
  // d. 1 stroke: the bowl anticlockwise, up the stem to the top, then down it.
  [
    "latin:d",
    {
      script: "latin",
      glyph: "d",
      strokes: [
        {
          segments: [
            {
              label: "round the bowl from the top right",
              path: [
                { x: 423, y: 428 },
                { x: 380, y: 443 },
                { x: 337, y: 458 },
                { x: 291, y: 464 },
                { x: 246, y: 462 },
                { x: 203, y: 446 },
                { x: 168, y: 417 },
                { x: 138, y: 383 },
                { x: 117, y: 342 },
                { x: 104, y: 298 },
                { x: 99, y: 253 },
                { x: 99, y: 207 },
                { x: 103, y: 162 },
                { x: 117, y: 118 },
                { x: 145, y: 82 },
                { x: 178, y: 51 },
                { x: 219, y: 32 },
                { x: 264, y: 28 },
                { x: 309, y: 38 },
                { x: 349, y: 60 },
                { x: 382, y: 91 },
                { x: 415, y: 124 },
                { x: 446, y: 157 },
                { x: 459, y: 200 },
              ],
            },
            {
              label: "up the stem to the top, then down to the foot",
              path: [
                { x: 459, y: 200 },
                { x: 459, y: 245 },
                { x: 459, y: 289 },
                { x: 459, y: 334 },
                { x: 458, y: 379 },
                { x: 451, y: 423 },
                { x: 455, y: 468 },
                { x: 455, y: 512 },
                { x: 455, y: 557 },
                { x: 455, y: 602 },
                { x: 455, y: 646 },
                { x: 455, y: 691 },
                { x: 455, y: 718 },
                { x: 455, y: 673 },
                { x: 455, y: 628 },
                { x: 455, y: 584 },
                { x: 455, y: 539 },
                { x: 455, y: 494 },
                { x: 453, y: 449 },
                { x: 453, y: 405 },
                { x: 455, y: 360 },
                { x: 455, y: 316 },
                { x: 455, y: 271 },
                { x: 455, y: 226 },
                { x: 455, y: 182 },
                { x: 452, y: 137 },
                { x: 468, y: 95 },
                { x: 479, y: 52 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("d"),
    },
  ],
  // p. 1 stroke: the stem, back up it, the bowl clockwise.
  [
    "latin:p",
    {
      script: "latin",
      glyph: "p",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 107, y: 457 },
                { x: 113, y: 414 },
                { x: 121, y: 371 },
                { x: 123, y: 328 },
                { x: 123, y: 284 },
                { x: 123, y: 240 },
                { x: 123, y: 197 },
                { x: 123, y: 153 },
                { x: 123, y: 110 },
                { x: 123, y: 66 },
                { x: 123, y: 23 },
                { x: 119, y: -21 },
                { x: 119, y: -64 },
                { x: 119, y: -108 },
                { x: 123, y: -151 },
                { x: 123, y: -195 },
              ],
            },
            {
              label: "back up and round the bowl",
              path: [
                { x: 123, y: -195 },
                { x: 123, y: -150 },
                { x: 123, y: -105 },
                { x: 123, y: -61 },
                { x: 123, y: -16 },
                { x: 123, y: 29 },
                { x: 127, y: 73 },
                { x: 124, y: 118 },
                { x: 123, y: 163 },
                { x: 123, y: 207 },
                { x: 123, y: 252 },
                { x: 124, y: 297 },
                { x: 132, y: 341 },
                { x: 166, y: 369 },
                { x: 198, y: 400 },
                { x: 232, y: 429 },
                { x: 271, y: 452 },
                { x: 314, y: 461 },
                { x: 359, y: 461 },
                { x: 400, y: 445 },
                { x: 433, y: 414 },
                { x: 456, y: 376 },
                { x: 471, y: 334 },
                { x: 478, y: 290 },
                { x: 479, y: 245 },
                { x: 477, y: 200 },
                { x: 465, y: 157 },
                { x: 446, y: 116 },
                { x: 420, y: 80 },
                { x: 385, y: 52 },
                { x: 344, y: 35 },
                { x: 299, y: 33 },
                { x: 255, y: 36 },
                { x: 214, y: 54 },
                { x: 175, y: 76 },
                { x: 131, y: 85 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("p"),
    },
  ],
  // q. 1 stroke: the bowl anticlockwise, back up the stem, then down below the line.
  [
    "latin:q",
    {
      script: "latin",
      glyph: "q",
      strokes: [
        {
          segments: [
            {
              label: "round the bowl from the top right",
              path: [
                { x: 419, y: 441 },
                { x: 377, y: 454 },
                { x: 334, y: 464 },
                { x: 290, y: 465 },
                { x: 246, y: 461 },
                { x: 205, y: 445 },
                { x: 170, y: 420 },
                { x: 141, y: 386 },
                { x: 121, y: 347 },
                { x: 107, y: 306 },
                { x: 100, y: 262 },
                { x: 99, y: 218 },
                { x: 100, y: 174 },
                { x: 112, y: 132 },
                { x: 137, y: 96 },
                { x: 168, y: 64 },
                { x: 203, y: 39 },
                { x: 246, y: 30 },
                { x: 289, y: 37 },
                { x: 328, y: 58 },
                { x: 361, y: 86 },
                { x: 395, y: 114 },
                { x: 427, y: 145 },
              ],
            },
            {
              label: "back up, then down the stem",
              path: [
                { x: 427, y: 145 },
                { x: 435, y: 189 },
                { x: 435, y: 234 },
                { x: 435, y: 279 },
                { x: 435, y: 324 },
                { x: 435, y: 369 },
                { x: 432, y: 414 },
                { x: 431, y: 389 },
                { x: 432, y: 344 },
                { x: 435, y: 300 },
                { x: 435, y: 255 },
                { x: 435, y: 210 },
                { x: 435, y: 165 },
                { x: 435, y: 120 },
                { x: 435, y: 75 },
                { x: 435, y: 30 },
                { x: 435, y: -15 },
                { x: 435, y: -60 },
                { x: 435, y: -105 },
                { x: 435, y: -150 },
                { x: 435, y: -195 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("q"),
    },
  ],
  // t. 2 strokes: the stem, then the crossbar left to right.
  [
    "latin:t",
    {
      script: "latin",
      glyph: "t",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down and round to the right",
              path: [
                { x: 155, y: 604 },
                { x: 155, y: 560 },
                { x: 155, y: 516 },
                { x: 155, y: 472 },
                { x: 155, y: 429 },
                { x: 155, y: 385 },
                { x: 155, y: 341 },
                { x: 155, y: 297 },
                { x: 157, y: 253 },
                { x: 159, y: 209 },
                { x: 159, y: 166 },
                { x: 159, y: 122 },
                { x: 170, y: 79 },
                { x: 199, y: 47 },
                { x: 240, y: 36 },
                { x: 284, y: 36 },
                { x: 327, y: 44 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the crossbar, left to right",
              path: [
                { x: 47, y: 448 },
                { x: 94, y: 448 },
                { x: 140, y: 449 },
                { x: 187, y: 452 },
                { x: 234, y: 452 },
                { x: 280, y: 452 },
                { x: 327, y: 448 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("t"),
    },
  ],
  // y. 2 strokes: the short line, then the long line with its tail.
  [
    "latin:y",
    {
      script: "latin",
      glyph: "y",
      strokes: [
        {
          segments: [
            {
              label: "draw the short line down to the right",
              path: [
                { x: 86, y: 447 },
                { x: 103, y: 405 },
                { x: 121, y: 364 },
                { x: 138, y: 322 },
                { x: 154, y: 280 },
                { x: 171, y: 239 },
                { x: 188, y: 197 },
                { x: 204, y: 155 },
                { x: 221, y: 113 },
                { x: 240, y: 72 },
                { x: 270, y: 39 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the long line down to the left",
              path: [
                { x: 438, y: 447 },
                { x: 422, y: 405 },
                { x: 407, y: 362 },
                { x: 392, y: 320 },
                { x: 376, y: 278 },
                { x: 361, y: 235 },
                { x: 347, y: 193 },
                { x: 331, y: 150 },
                { x: 316, y: 108 },
                { x: 296, y: 67 },
                { x: 272, y: 30 },
                { x: 269, y: -15 },
                { x: 254, y: -57 },
                { x: 234, y: -97 },
                { x: 206, y: -133 },
                { x: 174, y: -164 },
                { x: 134, y: -183 },
                { x: 89, y: -185 },
                { x: 46, y: -173 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("y"),
    },
  ],
  // H. 3 strokes: the left stem, the crossbar, the right stem.
  [
    "latin:H",
    {
      script: "latin",
      glyph: "H",
      strokes: [
        {
          segments: [
            {
              label: "draw the left stem down",
              path: [
                { x: 130, y: 668 },
                { x: 130, y: 623 },
                { x: 130, y: 579 },
                { x: 130, y: 534 },
                { x: 130, y: 489 },
                { x: 130, y: 445 },
                { x: 132, y: 400 },
                { x: 135, y: 356 },
                { x: 130, y: 312 },
                { x: 130, y: 267 },
                { x: 130, y: 223 },
                { x: 130, y: 178 },
                { x: 130, y: 133 },
                { x: 130, y: 89 },
                { x: 130, y: 44 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the crossbar, left to right",
              path: [
                { x: 138, y: 368 },
                { x: 181, y: 368 },
                { x: 224, y: 368 },
                { x: 266, y: 368 },
                { x: 309, y: 368 },
                { x: 352, y: 368 },
                { x: 395, y: 368 },
                { x: 438, y: 368 },
                { x: 480, y: 368 },
                { x: 523, y: 368 },
                { x: 566, y: 368 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the right stem down",
              path: [
                { x: 578, y: 668 },
                { x: 578, y: 623 },
                { x: 578, y: 579 },
                { x: 578, y: 534 },
                { x: 578, y: 489 },
                { x: 578, y: 445 },
                { x: 575, y: 400 },
                { x: 570, y: 356 },
                { x: 574, y: 312 },
                { x: 574, y: 267 },
                { x: 574, y: 222 },
                { x: 574, y: 178 },
                { x: 574, y: 133 },
                { x: 574, y: 88 },
                { x: 578, y: 44 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("H"),
    },
  ],
  // á. 2 strokes: the a as above, then the acute up to the right (UJIpenchars2).
  [
    "latin:á",
    {
      script: "latin",
      glyph: "á",
      strokes: [
        {
          segments: [
            {
              label: "round the bowl from the top right",
              path: [
                { x: 431, y: 448 },
                { x: 388, y: 462 },
                { x: 344, y: 468 },
                { x: 299, y: 468 },
                { x: 254, y: 463 },
                { x: 212, y: 447 },
                { x: 178, y: 418 },
                { x: 147, y: 385 },
                { x: 125, y: 346 },
                { x: 111, y: 304 },
                { x: 101, y: 260 },
                { x: 99, y: 215 },
                { x: 99, y: 170 },
                { x: 106, y: 126 },
                { x: 123, y: 84 },
                { x: 152, y: 50 },
                { x: 193, y: 33 },
                { x: 237, y: 36 },
                { x: 276, y: 58 },
                { x: 308, y: 90 },
                { x: 335, y: 126 },
                { x: 357, y: 165 },
                { x: 380, y: 204 },
                { x: 411, y: 236 },
                { x: 427, y: 276 },
              ],
            },
            {
              label: "back up, then down to the foot",
              path: [
                { x: 427, y: 276 },
                { x: 434, y: 322 },
                { x: 435, y: 368 },
                { x: 435, y: 414 },
                { x: 435, y: 418 },
                { x: 435, y: 371 },
                { x: 435, y: 325 },
                { x: 429, y: 279 },
                { x: 436, y: 234 },
                { x: 447, y: 190 },
                { x: 447, y: 144 },
                { x: 451, y: 98 },
                { x: 459, y: 52 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the acute, up to the right",
              path: [
                { x: 267, y: 604 },
                { x: 296, y: 633 },
                { x: 323, y: 663 },
                { x: 350, y: 694 },
                { x: 379, y: 724 },
                { x: 411, y: 748 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("á"),
    },
  ],
  // é. 2 strokes: the e as above, then the acute up to the right (UJIpenchars2).
  [
    "latin:é",
    {
      script: "latin",
      glyph: "é",
      strokes: [
        {
          segments: [
            {
              label: "draw the bar to the right",
              path: [
                { x: 102, y: 276 },
                { x: 150, y: 272 },
                { x: 198, y: 272 },
                { x: 246, y: 272 },
                { x: 294, y: 272 },
                { x: 342, y: 272 },
                { x: 390, y: 272 },
                { x: 434, y: 288 },
              ],
            },
            {
              label: "curve up, round and down",
              path: [
                { x: 434, y: 288 },
                { x: 434, y: 333 },
                { x: 425, y: 377 },
                { x: 403, y: 417 },
                { x: 369, y: 447 },
                { x: 327, y: 464 },
                { x: 282, y: 468 },
                { x: 237, y: 467 },
                { x: 194, y: 454 },
                { x: 158, y: 427 },
                { x: 129, y: 393 },
                { x: 108, y: 352 },
                { x: 102, y: 308 },
                { x: 98, y: 263 },
                { x: 94, y: 218 },
                { x: 99, y: 173 },
                { x: 116, y: 131 },
                { x: 145, y: 97 },
                { x: 178, y: 65 },
                { x: 216, y: 41 },
                { x: 260, y: 32 },
                { x: 305, y: 32 },
                { x: 351, y: 33 },
                { x: 395, y: 42 },
                { x: 438, y: 56 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the acute, up to the right",
              path: [
                { x: 254, y: 604 },
                { x: 283, y: 633 },
                { x: 310, y: 663 },
                { x: 337, y: 694 },
                { x: 366, y: 724 },
                { x: 398, y: 748 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("é"),
    },
  ],
  // í. 2 strokes: the stem, then the acute in the dot's place (UJIpenchars2).
  [
    "latin:í",
    {
      script: "latin",
      glyph: "í",
      strokes: [
        {
          segments: [
            {
              label: "draw the stem down",
              path: [
                { x: 136, y: 452 },
                { x: 136, y: 407 },
                { x: 136, y: 361 },
                { x: 136, y: 316 },
                { x: 136, y: 271 },
                { x: 140, y: 225 },
                { x: 140, y: 180 },
                { x: 140, y: 135 },
                { x: 140, y: 89 },
                { x: 140, y: 44 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the acute, up to the right",
              path: [
                { x: 115, y: 604 },
                { x: 144, y: 633 },
                { x: 171, y: 663 },
                { x: 198, y: 694 },
                { x: 227, y: 724 },
                { x: 259, y: 748 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("í"),
    },
  ],
  // ó. 2 strokes: the o as above, then the acute up to the right (UJIpenchars2).
  [
    "latin:ó",
    {
      script: "latin",
      glyph: "ó",
      strokes: [
        {
          segments: [
            {
              label: "round to the left, back to the top",
              path: [
                { x: 278, y: 468 },
                { x: 234, y: 463 },
                { x: 193, y: 445 },
                { x: 161, y: 415 },
                { x: 130, y: 383 },
                { x: 108, y: 344 },
                { x: 96, y: 301 },
                { x: 94, y: 257 },
                { x: 94, y: 212 },
                { x: 101, y: 168 },
                { x: 116, y: 126 },
                { x: 141, y: 90 },
                { x: 173, y: 59 },
                { x: 212, y: 38 },
                { x: 256, y: 29 },
                { x: 300, y: 29 },
                { x: 343, y: 40 },
                { x: 379, y: 65 },
                { x: 411, y: 97 },
                { x: 440, y: 131 },
                { x: 456, y: 172 },
                { x: 462, y: 216 },
                { x: 462, y: 261 },
                { x: 460, y: 305 },
                { x: 449, y: 348 },
                { x: 429, y: 388 },
                { x: 398, y: 420 },
                { x: 364, y: 449 },
                { x: 322, y: 463 },
                { x: 278, y: 468 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the acute, up to the right",
              path: [
                { x: 254, y: 604 },
                { x: 283, y: 633 },
                { x: 310, y: 663 },
                { x: 337, y: 694 },
                { x: 366, y: 724 },
                { x: 398, y: 748 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("ó"),
    },
  ],
  // ú. 2 strokes: the u as above, then the acute up to the right (UJIpenchars2).
  [
    "latin:ú",
    {
      script: "latin",
      glyph: "ú",
      strokes: [
        {
          segments: [
            {
              label: "down, round and up",
              path: [
                { x: 118, y: 452 },
                { x: 118, y: 408 },
                { x: 118, y: 363 },
                { x: 118, y: 319 },
                { x: 118, y: 274 },
                { x: 120, y: 230 },
                { x: 122, y: 185 },
                { x: 126, y: 141 },
                { x: 139, y: 99 },
                { x: 169, y: 65 },
                { x: 203, y: 38 },
                { x: 248, y: 36 },
                { x: 292, y: 39 },
                { x: 331, y: 60 },
                { x: 365, y: 88 },
                { x: 397, y: 119 },
                { x: 433, y: 146 },
                { x: 451, y: 185 },
                { x: 454, y: 230 },
                { x: 454, y: 274 },
                { x: 454, y: 319 },
                { x: 454, y: 363 },
                { x: 454, y: 408 },
                { x: 454, y: 452 },
              ],
            },
            {
              label: "draw the right side down",
              path: [
                { x: 454, y: 452 },
                { x: 454, y: 407 },
                { x: 454, y: 363 },
                { x: 454, y: 318 },
                { x: 454, y: 273 },
                { x: 454, y: 229 },
                { x: 454, y: 184 },
                { x: 455, y: 139 },
                { x: 465, y: 96 },
                { x: 474, y: 52 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the acute, up to the right",
              path: [
                { x: 274, y: 612 },
                { x: 305, y: 648 },
                { x: 337, y: 683 },
                { x: 371, y: 717 },
                { x: 406, y: 748 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("ú"),
    },
  ],
  // ü. 3 strokes: the u as above, then the left dot, then the right dot (UJIpenchars2).
  [
    "latin:ü",
    {
      script: "latin",
      glyph: "ü",
      strokes: [
        {
          segments: [
            {
              label: "down, round and up",
              path: [
                { x: 118, y: 452 },
                { x: 118, y: 408 },
                { x: 118, y: 363 },
                { x: 118, y: 319 },
                { x: 118, y: 274 },
                { x: 120, y: 230 },
                { x: 122, y: 185 },
                { x: 126, y: 141 },
                { x: 139, y: 99 },
                { x: 169, y: 65 },
                { x: 203, y: 38 },
                { x: 248, y: 36 },
                { x: 292, y: 39 },
                { x: 331, y: 60 },
                { x: 365, y: 88 },
                { x: 397, y: 119 },
                { x: 433, y: 146 },
                { x: 451, y: 185 },
                { x: 454, y: 230 },
                { x: 454, y: 274 },
                { x: 454, y: 319 },
                { x: 454, y: 363 },
                { x: 454, y: 408 },
                { x: 454, y: 452 },
              ],
            },
            {
              label: "draw the right side down",
              path: [
                { x: 454, y: 452 },
                { x: 454, y: 407 },
                { x: 454, y: 363 },
                { x: 454, y: 318 },
                { x: 454, y: 273 },
                { x: 454, y: 229 },
                { x: 454, y: 184 },
                { x: 455, y: 139 },
                { x: 465, y: 96 },
                { x: 474, y: 52 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the left dot",
              path: [
                { x: 186, y: 688 },
                { x: 186, y: 656 },
                { x: 186, y: 624 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then the right dot",
              path: [
                { x: 390, y: 688 },
                { x: 390, y: 656 },
                { x: 390, y: 624 },
              ],
            },
          ],
        },
      ],
      source: latinLetterSource("ü"),
    },
  ],
];
