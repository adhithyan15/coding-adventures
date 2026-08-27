// Authored devanagari ductus records. This is the stable source-ownership boundary.

import type { StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import devanagari from "../../../../../learning/human-languages/data/scripts/devanagari.json";

const devanagariAlphabetSource = (glyph: string): StrokeSource => {
  const letter = devanagari.letters.find((candidate) => candidate.glyph === glyph);
  if (!letter || !("strokeOrderSource" in letter) || !letter.strokeOrderSource) {
    throw new Error(`Devanagari ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

export const entries: DuctusEntry[] = [
  // The four-frame Commons sequence writes the complete left body in one
  // continuous run, lifts for the middle shoulder, descends the right stem,
  // then closes with the short shirorekha: four strokes and three lifts.
    ["devanagari:अ", {
    script: "devanagari",
    glyph: "अ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 570, y: 585 }, { x: 615, y: 585 },
        { x: 660, y: 585 }, { x: 705, y: 585 }, { x: 750, y: 585 },
        { x: 775, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("अ"),
  }],
  // The five Commons buildup frames preserve the joined left body of अ, then
  // add the shoulder, inner stem, trailing stem, and headline as four lifted
  // runs: five strokes and four lifts in all.
    ["devanagari:आ", {
    script: "devanagari",
    glyph: "आ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the inner stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then descend the trailing stem", path: [
        { x: 893, y: 590 }, { x: 893, y: 500 }, { x: 893, y: 410 },
        { x: 893, y: 320 }, { x: 893, y: 230 }, { x: 893, y: 140 },
        { x: 893, y: 50 }, { x: 893, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 610, y: 585 }, { x: 695, y: 585 },
        { x: 780, y: 585 }, { x: 865, y: 585 }, { x: 950, y: 585 },
        { x: 1030, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("आ"),
  }],
  // The Commons diagram writes the upright, both bowls, and tail as one
  // continuous body, then lifts once to draw the headline left-to-right.
    ["devanagari:इ", {
    script: "devanagari",
    glyph: "इ",
    strokes: [
      {
        segments: [
          { label: "descend the upright from the headline", path: [
            { x: 363, y: 590 }, { x: 363, y: 540 }, { x: 363, y: 490 },
            { x: 363, y: 440 },
          ] },
          { label: "turn left and curve around the upper bowl without lifting", path: [
            { x: 363, y: 440 }, { x: 320, y: 430 }, { x: 270, y: 430 },
            { x: 210, y: 430 }, { x: 160, y: 425 }, { x: 120, y: 405 },
            { x: 90, y: 375 }, { x: 85, y: 340 }, { x: 100, y: 305 },
            { x: 130, y: 275 }, { x: 170, y: 250 },
          ] },
          { label: "sweep right through the waist and around the lower bowl", path: [
            { x: 170, y: 250 }, { x: 220, y: 260 }, { x: 275, y: 260 },
            { x: 325, y: 245 }, { x: 370, y: 220 }, { x: 400, y: 185 },
            { x: 405, y: 150 }, { x: 390, y: 115 }, { x: 360, y: 85 },
            { x: 320, y: 60 }, { x: 275, y: 42 }, { x: 225, y: 35 },
            { x: 180, y: 40 }, { x: 140, y: 55 }, { x: 105, y: 75 },
            { x: 80, y: 85 },
          ] },
          { label: "finish down-right through the tail without lifting", path: [
            { x: 80, y: 85 }, { x: 120, y: 80 }, { x: 160, y: 60 },
            { x: 200, y: 35 }, { x: 230, y: 0 }, { x: 260, y: -35 },
            { x: 290, y: -75 }, { x: 320, y: -110 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 500, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("इ"),
  }],
  // The three Commons panels reuse इ's continuous body, then add the upper
  // curl and headline as two separately placed runs.
    ["devanagari:ई", {
    script: "devanagari",
    glyph: "ई",
    strokes: [
      {
        segments: [
          { label: "descend the upright from the headline", path: [
            { x: 363, y: 590 }, { x: 363, y: 540 }, { x: 363, y: 490 },
            { x: 363, y: 440 },
          ] },
          { label: "turn left and curve around the upper bowl without lifting", path: [
            { x: 363, y: 440 }, { x: 320, y: 430 }, { x: 270, y: 430 },
            { x: 210, y: 430 }, { x: 160, y: 425 }, { x: 120, y: 405 },
            { x: 90, y: 375 }, { x: 85, y: 340 }, { x: 100, y: 305 },
            { x: 130, y: 275 }, { x: 170, y: 250 },
          ] },
          { label: "sweep right through the waist and around the lower bowl", path: [
            { x: 170, y: 250 }, { x: 220, y: 260 }, { x: 275, y: 260 },
            { x: 325, y: 245 }, { x: 370, y: 220 }, { x: 400, y: 185 },
            { x: 405, y: 150 }, { x: 390, y: 115 }, { x: 360, y: 85 },
            { x: 320, y: 60 }, { x: 275, y: 42 }, { x: 225, y: 35 },
            { x: 180, y: 40 }, { x: 140, y: 55 }, { x: 105, y: 75 },
            { x: 80, y: 85 },
          ] },
          { label: "finish down-right through the tail without lifting", path: [
            { x: 80, y: 85 }, { x: 120, y: 80 }, { x: 160, y: 60 },
            { x: 200, y: 35 }, { x: 230, y: 0 }, { x: 260, y: -35 },
            { x: 290, y: -75 }, { x: 320, y: -110 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the upper curl upward and around to the right", path: [
        { x: 352, y: 620 }, { x: 330, y: 660 }, { x: 310, y: 710 },
        { x: 290, y: 760 }, { x: 300, y: 810 }, { x: 330, y: 850 },
        { x: 370, y: 865 }, { x: 410, y: 860 }, { x: 450, y: 850 },
        { x: 480, y: 835 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 500, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ई"),
  }],
  // The two Commons panels keep the upper bowl and lower loop in one
  // continuous body, then lift once to draw the headline left-to-right.
    ["devanagari:उ", {
    script: "devanagari",
    glyph: "उ",
    strokes: [
      {
        segments: [
          { label: "curve down and left around the upper bowl", path: [
            { x: 350, y: 555 }, { x: 390, y: 530 }, { x: 420, y: 500 },
            { x: 435, y: 460 }, { x: 420, y: 420 }, { x: 380, y: 380 },
            { x: 330, y: 345 }, { x: 275, y: 325 }, { x: 235, y: 320 },
          ] },
          { label: "sweep back through the waist and around the lower loop without lifting", path: [
            { x: 235, y: 320 }, { x: 280, y: 325 }, { x: 330, y: 335 },
            { x: 380, y: 325 }, { x: 420, y: 290 }, { x: 450, y: 240 },
            { x: 465, y: 180 }, { x: 450, y: 125 }, { x: 420, y: 85 },
            { x: 375, y: 55 }, { x: 325, y: 40 }, { x: 275, y: 45 },
            { x: 225, y: 65 }, { x: 180, y: 95 }, { x: 140, y: 135 },
            { x: 110, y: 180 }, { x: 85, y: 230 }, { x: 70, y: 280 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 558, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("उ"),
  }],
  // The three Commons panels reuse उ's continuous body, then add the
  // right-hand loop and headline as two separately placed runs.
    ["devanagari:ऊ", {
    script: "devanagari",
    glyph: "ऊ",
    strokes: [
      {
        segments: [
          { label: "curve down and left around the upper bowl", path: [
            { x: 350, y: 555 }, { x: 390, y: 530 }, { x: 420, y: 500 },
            { x: 435, y: 460 }, { x: 420, y: 420 }, { x: 380, y: 380 },
            { x: 330, y: 345 }, { x: 275, y: 325 }, { x: 235, y: 320 },
          ] },
          { label: "sweep back through the waist and around the lower loop without lifting", path: [
            { x: 235, y: 320 }, { x: 280, y: 325 }, { x: 330, y: 335 },
            { x: 380, y: 325 }, { x: 420, y: 290 }, { x: 450, y: 240 },
            { x: 465, y: 180 }, { x: 450, y: 125 }, { x: 420, y: 85 },
            { x: 375, y: 55 }, { x: 325, y: 40 }, { x: 275, y: 45 },
            { x: 225, y: 65 }, { x: 180, y: 95 }, { x: 140, y: 135 },
            { x: 110, y: 180 }, { x: 85, y: 230 }, { x: 70, y: 280 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the right-hand loop up, around, and down-left", path: [
        { x: 455, y: 250 }, { x: 490, y: 280 }, { x: 535, y: 305 },
        { x: 585, y: 310 }, { x: 630, y: 295 }, { x: 670, y: 270 },
        { x: 700, y: 235 }, { x: 715, y: 195 }, { x: 715, y: 150 },
        { x: 705, y: 105 }, { x: 685, y: 65 }, { x: 660, y: 25 },
        { x: 635, y: -5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 135, y: 585 }, { x: 265, y: 585 },
        { x: 395, y: 585 }, { x: 525, y: 585 }, { x: 655, y: 585 },
        { x: 795, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ऊ"),
  }],
  // The three Commons panels draw the long left stem and tail continuously,
  // then place the shorter hooked stem and headline as separate runs.
    ["devanagari:ए", {
    script: "devanagari",
    glyph: "ए",
    strokes: [
      {
        segments: [
          { label: "descend the long left stem from the headline", path: [
            { x: 120, y: 585 }, { x: 120, y: 530 }, { x: 120, y: 470 },
            { x: 120, y: 410 }, { x: 120, y: 350 }, { x: 125, y: 290 },
          ] },
          { label: "curve right through the lower shoulder and sweep down the tail without lifting", path: [
            { x: 125, y: 290 }, { x: 145, y: 245 }, { x: 185, y: 205 },
            { x: 235, y: 175 }, { x: 285, y: 145 }, { x: 335, y: 115 },
            { x: 380, y: 85 }, { x: 415, y: 50 }, { x: 435, y: 10 },
            { x: 435, y: -30 }, { x: 420, y: -70 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then descend the shorter right stem into its inward hook", path: [
        { x: 435, y: 585 }, { x: 435, y: 530 }, { x: 435, y: 470 },
        { x: 435, y: 410 }, { x: 430, y: 350 }, { x: 410, y: 300 },
        { x: 380, y: 260 }, { x: 350, y: 235 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 563, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ए"),
  }],
  // The four Commons panels reuse ए's long body and shorter hooked stem,
  // then add the upper arc and headline as two separately placed runs.
    ["devanagari:ऐ", {
    script: "devanagari",
    glyph: "ऐ",
    strokes: [
      {
        segments: [
          { label: "descend the long left stem from the headline", path: [
            { x: 120, y: 585 }, { x: 120, y: 530 }, { x: 120, y: 470 },
            { x: 120, y: 410 }, { x: 120, y: 350 }, { x: 125, y: 290 },
          ] },
          { label: "curve right through the lower shoulder and sweep down the tail without lifting", path: [
            { x: 125, y: 290 }, { x: 145, y: 245 }, { x: 185, y: 205 },
            { x: 235, y: 175 }, { x: 285, y: 145 }, { x: 335, y: 115 },
            { x: 380, y: 85 }, { x: 415, y: 50 }, { x: 435, y: 10 },
            { x: 435, y: -30 }, { x: 420, y: -70 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then descend the shorter right stem into its inward hook", path: [
        { x: 435, y: 585 }, { x: 435, y: 530 }, { x: 435, y: 470 },
        { x: 435, y: 410 }, { x: 430, y: 350 }, { x: 410, y: 300 },
        { x: 380, y: 260 }, { x: 350, y: 235 },
      ] }] },
      { segments: [{ label: "lift, then sweep the upper arc upward and left", path: [
        { x: 430, y: 620 }, { x: 415, y: 680 }, { x: 390, y: 745 },
        { x: 360, y: 800 }, { x: 325, y: 840 }, { x: 285, y: 860 },
        { x: 245, y: 865 }, { x: 205, y: 855 }, { x: 170, y: 835 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 563, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ऐ"),
  }],
  // The six Commons panels reuse आ's joined left body, separate shoulder,
  // inner stem, and trailing stem, then add the upper arc and headline as two
  // separately placed runs: six strokes and five lifts in all.
    ["devanagari:ओ", {
    script: "devanagari",
    glyph: "ओ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the inner stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then descend the trailing stem", path: [
        { x: 893, y: 590 }, { x: 893, y: 500 }, { x: 893, y: 410 },
        { x: 893, y: 320 }, { x: 893, y: 230 }, { x: 893, y: 140 },
        { x: 893, y: 50 }, { x: 893, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then sweep the upper arc upward and left", path: [
        { x: 890, y: 620 }, { x: 880, y: 680 }, { x: 860, y: 735 },
        { x: 835, y: 785 }, { x: 805, y: 825 }, { x: 770, y: 850 },
        { x: 730, y: 862 }, { x: 690, y: 860 }, { x: 655, y: 850 },
        { x: 625, y: 840 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 610, y: 585 }, { x: 695, y: 585 },
        { x: 780, y: 585 }, { x: 865, y: 585 }, { x: 950, y: 585 },
        { x: 1030, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ओ"),
  }],
  // The seven Commons panels reuse आ's four base runs, then separately sweep
  // the lower and taller upper arcs upward and left before the final headline:
  // seven strokes and six lifts in all.
    ["devanagari:औ", {
    script: "devanagari",
    glyph: "औ",
    strokes: [
      {
        segments: [
          { label: "curve right around the upper bowl", path: [
            { x: 165, y: 545 }, { x: 205, y: 575 }, { x: 250, y: 595 },
            { x: 300, y: 596 }, { x: 350, y: 580 }, { x: 395, y: 550 },
            { x: 420, y: 510 }, { x: 420, y: 470 }, { x: 400, y: 430 },
            { x: 360, y: 400 }, { x: 315, y: 375 }, { x: 275, y: 355 },
          ] },
          { label: "continue down and around the lower bowl without lifting", path: [
            { x: 275, y: 355 }, { x: 335, y: 330 }, { x: 395, y: 295 },
            { x: 430, y: 250 }, { x: 435, y: 205 }, { x: 415, y: 160 },
            { x: 375, y: 125 }, { x: 325, y: 105 }, { x: 275, y: 100 },
            { x: 225, y: 115 }, { x: 180, y: 145 }, { x: 140, y: 190 },
            { x: 105, y: 245 }, { x: 80, y: 305 },
          ] },
        ],
      },
      { segments: [{ label: "lift, then sweep the middle shoulder right", path: [
        { x: 290, y: 350 }, { x: 350, y: 340 }, { x: 410, y: 325 },
        { x: 470, y: 317 }, { x: 530, y: 320 }, { x: 585, y: 330 },
        { x: 625, y: 342 },
      ] }] },
      { segments: [{ label: "lift, then descend the inner stem", path: [
        { x: 635, y: 590 }, { x: 635, y: 500 }, { x: 635, y: 410 },
        { x: 635, y: 320 }, { x: 635, y: 230 }, { x: 635, y: 140 },
        { x: 635, y: 50 }, { x: 635, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then descend the trailing stem", path: [
        { x: 893, y: 590 }, { x: 893, y: 500 }, { x: 893, y: 410 },
        { x: 893, y: 320 }, { x: 893, y: 230 }, { x: 893, y: 140 },
        { x: 893, y: 50 }, { x: 893, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then sweep the lower upper arc upward and left", path: [
        { x: 890, y: 620 }, { x: 875, y: 650 }, { x: 850, y: 680 },
        { x: 820, y: 705 }, { x: 785, y: 730 }, { x: 745, y: 745 },
        { x: 705, y: 750 }, { x: 670, y: 745 }, { x: 640, y: 735 },
        { x: 620, y: 725 },
      ] }] },
      { segments: [{ label: "lift, then sweep the taller upper arc upward and left", path: [
        { x: 890, y: 620 }, { x: 880, y: 680 }, { x: 860, y: 735 },
        { x: 835, y: 785 }, { x: 805, y: 825 }, { x: 770, y: 850 },
        { x: 730, y: 862 }, { x: 690, y: 860 }, { x: 655, y: 850 },
        { x: 625, y: 840 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 525, y: 585 }, { x: 610, y: 585 }, { x: 695, y: 585 },
        { x: 780, y: 585 }, { x: 865, y: 585 }, { x: 950, y: 585 },
        { x: 1030, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("औ"),
  }],
  // Opiaterein's animation writes the left bowl counterclockwise, then places
  // the central stem, right-hand arch, and headline as three separate runs.
  // The Central Hindi Directorate's 2019 deskbook independently shows the
  // same four-part buildup: four strokes and three lifts in all.
    ["devanagari:क", {
    script: "devanagari",
    glyph: "क",
    strokes: [
      { segments: [{ label: "sweep left over the top and around the bowl", path: [
        { x: 355, y: 430 }, { x: 315, y: 430 },
        { x: 270, y: 435 }, { x: 225, y: 430 }, { x: 180, y: 415 },
        { x: 140, y: 390 }, { x: 110, y: 360 }, { x: 90, y: 325 },
        { x: 86, y: 290 }, { x: 95, y: 250 }, { x: 115, y: 215 },
        { x: 145, y: 185 }, { x: 180, y: 165 }, { x: 220, y: 154 },
        { x: 260, y: 155 }, { x: 300, y: 170 }, { x: 335, y: 195 },
        { x: 360, y: 225 }, { x: 377, y: 260 }, { x: 387, y: 290 },
      ] }] },
      { segments: [{ label: "lift, then descend the central stem", path: [
        { x: 417, y: 551 }, { x: 417, y: 480 }, { x: 417, y: 400 },
        { x: 417, y: 320 }, { x: 417, y: 240 }, { x: 417, y: 160 },
        { x: 417, y: 80 }, { x: 417, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then sweep the right-hand arch clockwise", path: [
        { x: 455, y: 350 }, { x: 490, y: 365 }, { x: 530, y: 370 },
        { x: 570, y: 365 }, { x: 610, y: 345 }, { x: 645, y: 320 },
        { x: 670, y: 285 }, { x: 685, y: 245 }, { x: 680, y: 205 },
        { x: 660, y: 165 }, { x: 640, y: 125 }, { x: 620, y: 95 },
        { x: 600, y: 70 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 115, y: 585 }, { x: 225, y: 585 },
        { x: 335, y: 585 }, { x: 445, y: 585 }, { x: 555, y: 585 },
        { x: 665, y: 585 }, { x: 778, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("क"),
  }],
  // Opiaterein's 28-frame animation joins the descending left stem, small
  // loop, and broad lower bowl, then separately writes the upper-right loop,
  // right stem, and headline: four strokes and three lifts in all.
    ["devanagari:ख", {
    script: "devanagari",
    glyph: "ख",
    strokes: [
      { segments: [{ label: "descend, curl around the small loop, and sweep through the lower bowl", path: [
        { x: 225, y: 551 }, { x: 225, y: 500 }, { x: 220, y: 450 },
          { x: 220, y: 405 }, { x: 210, y: 390 }, { x: 200, y: 370 },
          { x: 150, y: 370 }, { x: 140, y: 370 },
        { x: 105, y: 355 }, { x: 80, y: 330 }, { x: 85, y: 305 },
        { x: 110, y: 285 }, { x: 140, y: 290 }, { x: 160, y: 310 },
        { x: 160, y: 335 }, { x: 145, y: 355 }, { x: 138, y: 335 },
        { x: 145, y: 270 },
        { x: 180, y: 200 }, { x: 240, y: 130 }, { x: 320, y: 80 },
          { x: 400, y: 55 }, { x: 480, y: 65 }, { x: 550, y: 70 },
          { x: 650, y: 70 }, { x: 650, y: 105 }, { x: 650, y: 155 },
          { x: 650, y: 220 },
      ] }] },
      { segments: [{ label: "lift, then sweep clockwise around the upper-right loop", path: [
        { x: 380, y: 405 }, { x: 430, y: 430 }, { x: 500, y: 430 },
        { x: 600, y: 420 }, { x: 650, y: 405 }, { x: 650, y: 350 },
        { x: 650, y: 290 }, { x: 625, y: 240 }, { x: 600, y: 220 },
        { x: 550, y: 200 }, { x: 500, y: 185 }, { x: 455, y: 200 },
        { x: 420, y: 220 }, { x: 395, y: 240 }, { x: 375, y: 260 },
        { x: 360, y: 290 }, { x: 360, y: 320 }, { x: 360, y: 350 },
        { x: 375, y: 390 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 650, y: 551 }, { x: 650, y: 475 }, { x: 650, y: 395 },
        { x: 650, y: 315 }, { x: 650, y: 235 }, { x: 650, y: 155 },
        { x: 650, y: 75 }, { x: 650, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 105, y: 585 }, { x: 205, y: 585 },
        { x: 305, y: 585 }, { x: 405, y: 585 }, { x: 505, y: 585 },
        { x: 605, y: 585 }, { x: 720, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ख"),
  }],
  // Opiaterein's animation writes the counterclockwise loop and carries the
  // same run up its joined stem, then separately descends the right stem and
  // finishes the headline. The Central Hindi Directorate's 2019 deskbook
  // independently shows the same three-part buildup: three strokes, two lifts.
    ["devanagari:ग", {
    script: "devanagari",
    glyph: "ग",
    strokes: [
      { segments: [{ label: "sweep counterclockwise around the loop and up the joined stem", path: [
        { x: 168, y: 315 }, { x: 140, y: 322 }, { x: 110, y: 312 },
        { x: 85, y: 290 }, { x: 76, y: 262 }, { x: 86, y: 232 },
        { x: 112, y: 208 }, { x: 142, y: 198 }, { x: 168, y: 205 },
        { x: 168, y: 250 }, { x: 168, y: 320 }, { x: 168, y: 400 },
        { x: 168, y: 475 }, { x: 168, y: 550 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 434, y: 551 }, { x: 434, y: 475 }, { x: 434, y: 395 },
        { x: 434, y: 315 }, { x: 434, y: 235 }, { x: 434, y: 155 },
        { x: 434, y: 75 }, { x: 434, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 572, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ग"),
  }],
  // Opiaterein's animation keeps the upper curl, middle hook, lower bowl, and
  // rising right side in one run, then separately descends the short lower
  // stem and finishes the headline: three strokes, two lifts.
    ["devanagari:घ", {
    script: "devanagari",
    glyph: "घ",
    strokes: [
      { segments: [{ label: "sweep through the upper curl and hook, around the lower bowl, and up the right side", path: [
        { x: 115, y: 525 }, { x: 90, y: 515 }, { x: 82, y: 485 },
        { x: 90, y: 450 }, { x: 115, y: 420 }, { x: 155, y: 395 },
        { x: 205, y: 375 }, { x: 260, y: 365 }, { x: 285, y: 360 },
        { x: 255, y: 350 }, { x: 215, y: 345 }, { x: 170, y: 342 },
        { x: 135, y: 325 }, { x: 115, y: 295 }, { x: 110, y: 260 },
        { x: 120, y: 225 }, { x: 145, y: 195 }, { x: 185, y: 170 },
        { x: 235, y: 155 }, { x: 290, y: 150 }, { x: 345, y: 155 },
        { x: 395, y: 165 }, { x: 430, y: 185 }, { x: 450, y: 225 },
        { x: 468, y: 290 }, { x: 470, y: 360 }, { x: 470, y: 430 },
        { x: 470, y: 500 }, { x: 470, y: 551 },
      ] }] },
      { segments: [{ label: "lift, then descend the short lower stem", path: [
        { x: 470, y: 175 }, { x: 470, y: 135 }, { x: 470, y: 95 },
        { x: 470, y: 70 }, { x: 470, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 572, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("घ"),
  }],
  // Opiaterein's animation joins the short upper bar directly to the rounded
  // body, then separately descends the right stem and finishes the headline.
  // The Central Hindi Directorate deskbook corroborates component order while
  // staging the bar and body separately: three animated strokes, two lifts.
    ["devanagari:च", {
    script: "devanagari",
    glyph: "च",
    strokes: [
      { segments: [{ label: "draw the upper bar right and curve around the open body", path: [
        { x: 45, y: 412 }, { x: 100, y: 412 }, { x: 160, y: 412 },
        { x: 220, y: 412 }, { x: 280, y: 412 }, { x: 340, y: 412 },
        { x: 320, y: 395 }, { x: 300, y: 380 }, { x: 270, y: 372 },
        { x: 235, y: 365 }, { x: 215, y: 350 }, { x: 200, y: 330 },
        { x: 187, y: 305 }, { x: 178, y: 275 }, { x: 177, y: 250 },
        { x: 180, y: 218 },
        { x: 200, y: 185 }, { x: 235, y: 160 }, { x: 280, y: 145 },
        { x: 325, y: 145 }, { x: 370, y: 160 }, { x: 410, y: 182 },
        { x: 447, y: 210 }, { x: 470, y: 238 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 505, y: 551 }, { x: 505, y: 475 }, { x: 505, y: 395 },
        { x: 505, y: 315 }, { x: 505, y: 235 }, { x: 505, y: 155 },
        { x: 505, y: 75 }, { x: 505, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 545, y: 585 }, { x: 644, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("च"),
  }],
  // Opiaterein's animation keeps the upper-left loop, lower bowl, outer-right
  // rise, and inner loop in one continuous run, then separately descends the
  // short upper stem and finishes the headline: three strokes, two lifts.
    ["devanagari:छ", {
    script: "devanagari",
    glyph: "छ",
    strokes: [
      { segments: [{ label: "sweep through both left loops, around the lower bowl, and into the inner loop", path: [
        { x: 275, y: 458 }, { x: 245, y: 478 }, { x: 195, y: 480 },
        { x: 145, y: 472 }, { x: 100, y: 452 }, { x: 68, y: 420 },
        { x: 48, y: 382 }, { x: 45, y: 345 }, { x: 58, y: 310 },
        { x: 85, y: 280 }, { x: 120, y: 258 }, { x: 158, y: 245 },
        { x: 195, y: 238 }, { x: 170, y: 220 }, { x: 138, y: 195 },
        { x: 112, y: 165 }, { x: 108, y: 142 }, { x: 120, y: 120 },
        { x: 145, y: 92 }, { x: 175, y: 65 }, { x: 210, y: 42 },
        { x: 248, y: 10 }, { x: 305, y: 10 }, { x: 365, y: 20 },
        { x: 425, y: 42 }, { x: 480, y: 78 }, { x: 528, y: 125 },
        { x: 565, y: 180 }, { x: 590, y: 240 }, { x: 602, y: 305 },
        { x: 600, y: 365 }, { x: 585, y: 415 }, { x: 558, y: 452 },
        { x: 525, y: 474 }, { x: 492, y: 478 }, { x: 465, y: 464 },
        { x: 435, y: 445 }, { x: 412, y: 420 }, { x: 398, y: 390 },
        { x: 392, y: 357 }, { x: 398, y: 325 }, { x: 414, y: 295 },
        { x: 440, y: 270 }, { x: 470, y: 250 }, { x: 500, y: 232 },
        { x: 525, y: 218 },
      ] }] },
      { segments: [{ label: "lift, then descend the short upper stem", path: [
        { x: 510, y: 551 }, { x: 510, y: 530 }, { x: 510, y: 505 },
        { x: 510, y: 480 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 95, y: 585 }, { x: 185, y: 585 },
        { x: 275, y: 585 }, { x: 365, y: 585 }, { x: 455, y: 585 },
        { x: 545, y: 585 }, { x: 635, y: 585 }, { x: 715, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("छ"),
  }],
  // Opiaterein's animation keeps the open hook, clockwise lower bowl, inner
  // shoulder, and middle bar in one run, then separately descends the right
  // stem and finishes the headline: three strokes, two lifts.
    ["devanagari:ज", {
    script: "devanagari",
    glyph: "ज",
    strokes: [
      { segments: [{ label: "sweep around the lower bowl and continue right through the middle bar", path: [
        { x: 90, y: 420 }, { x: 70, y: 370 }, { x: 90, y: 300 },
        { x: 115, y: 240 }, { x: 155, y: 180 }, { x: 210, y: 140 },
        { x: 270, y: 115 }, { x: 330, y: 110 }, { x: 390, y: 135 },
        { x: 415, y: 180 }, { x: 420, y: 240 }, { x: 410, y: 300 },
        { x: 390, y: 350 }, { x: 350, y: 380 }, { x: 300, y: 400 },
        { x: 380, y: 400 }, { x: 480, y: 400 }, { x: 575, y: 400 },
        { x: 610, y: 400 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 610, y: 551 }, { x: 610, y: 475 }, { x: 610, y: 395 },
        { x: 610, y: 315 }, { x: 610, y: 235 }, { x: 610, y: 155 },
        { x: 610, y: 75 }, { x: 610, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 585, y: 585 }, { x: 675, y: 585 },
        { x: 750, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ज"),
  }],
  // Opiaterein's animation joins the short upper stem, upper bowl, lower loop,
  // and diagonal tail, then separately adds the middle crossbar, right stem,
  // and headline: four strokes, three lifts.
    ["devanagari:झ", {
    script: "devanagari",
    glyph: "झ",
    strokes: [
      { segments: [{ label: "descend through both bowls and finish through the diagonal tail", path: [
        { x: 362, y: 551 }, { x: 362, y: 510 }, { x: 362, y: 470 },
        { x: 340, y: 445 }, { x: 300, y: 428 }, { x: 245, y: 420 },
        { x: 185, y: 420 }, { x: 130, y: 415 }, { x: 95, y: 395 },
        { x: 88, y: 365 }, { x: 92, y: 335 }, { x: 110, y: 305 },
        { x: 145, y: 282 }, { x: 195, y: 270 }, { x: 250, y: 262 },
        { x: 315, y: 250 }, { x: 365, y: 225 }, { x: 395, y: 195 },
        { x: 408, y: 160 }, { x: 405, y: 125 }, { x: 390, y: 95 },
        { x: 360, y: 72 }, { x: 315, y: 58 }, { x: 265, y: 52 },
        { x: 210, y: 55 }, { x: 155, y: 62 }, { x: 110, y: 78 },
        { x: 82, y: 100 }, { x: 75, y: 122 }, { x: 85, y: 100 },
        { x: 110, y: 82 }, { x: 135, y: 70 }, { x: 160, y: 62 },
        { x: 185, y: 45 }, { x: 210, y: 10 }, { x: 235, y: -25 },
        { x: 260, y: -55 },
      ] }] },
      { segments: [{ label: "lift, then draw the middle crossbar left-to-right", path: [
        { x: 315, y: 240 }, { x: 365, y: 240 }, { x: 415, y: 240 },
        { x: 465, y: 240 }, { x: 520, y: 240 }, { x: 610, y: 240 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 610, y: 551 }, { x: 610, y: 475 }, { x: 610, y: 395 },
        { x: 610, y: 315 }, { x: 610, y: 235 }, { x: 610, y: 155 },
        { x: 610, y: 75 }, { x: 610, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 585, y: 585 }, { x: 675, y: 585 },
        { x: 750, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("झ"),
  }],
  // Opiaterein's animation separately draws the open-left bowl, the rightward
  // shoulder rising to the headline, the short lower stem, and the headline:
  // four strokes, three lifts.
    ["devanagari:ञ", {
    script: "devanagari",
    glyph: "ञ",
    strokes: [
      { segments: [{ label: "sweep clockwise around the open-left bowl", path: [
        { x: 215, y: 420 }, { x: 255, y: 438 }, { x: 305, y: 442 },
        { x: 355, y: 432 }, { x: 400, y: 410 }, { x: 435, y: 380 },
        { x: 455, y: 345 }, { x: 445, y: 315 }, { x: 430, y: 285 },
        { x: 418, y: 250 }, { x: 400, y: 220 }, { x: 365, y: 190 },
        { x: 320, y: 165 }, { x: 270, y: 150 }, { x: 220, y: 145 },
        { x: 180, y: 155 }, { x: 145, y: 175 }, { x: 118, y: 202 },
        { x: 95, y: 230 }, { x: 78, y: 260 }, { x: 62, y: 290 },
        { x: 48, y: 320 },
      ] }] },
      { segments: [{ label: "lift, then sweep the shoulder right and rise to the headline", path: [
        { x: 415, y: 275 }, { x: 460, y: 270 }, { x: 505, y: 275 },
        { x: 545, y: 300 }, { x: 575, y: 330 }, { x: 610, y: 350 },
        { x: 610, y: 410 }, { x: 610, y: 475 }, { x: 610, y: 551 },
      ] }] },
      { segments: [{ label: "lift, then descend the short lower stem", path: [
        { x: 610, y: 275 }, { x: 610, y: 220 }, { x: 610, y: 165 },
        { x: 610, y: 110 }, { x: 610, y: 55 }, { x: 610, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 585, y: 585 }, { x: 675, y: 585 },
        { x: 750, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ञ"),
  }],
  // Opiaterein's animation joins the descending central stem to the
  // counterclockwise open body, then finishes with a separate headline:
  // two strokes, one lift.
    ["devanagari:ट", {
    script: "devanagari",
    glyph: "ट",
    strokes: [
      { segments: [{ label: "descend the stem and sweep counterclockwise around the open body", path: [
        { x: 358, y: 551 }, { x: 358, y: 500 }, { x: 358, y: 445 },
        { x: 358, y: 395 }, { x: 330, y: 390 }, { x: 290, y: 388 },
        { x: 245, y: 382 }, { x: 200, y: 368 }, { x: 158, y: 345 },
        { x: 125, y: 312 }, { x: 100, y: 272 }, { x: 86, y: 225 },
        { x: 88, y: 178 }, { x: 105, y: 135 }, { x: 138, y: 98 },
        { x: 180, y: 72 }, { x: 228, y: 58 }, { x: 278, y: 57 },
        { x: 328, y: 66 }, { x: 375, y: 84 }, { x: 416, y: 110 },
        { x: 430, y: 125 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 500, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ट"),
  }],
  // Opiaterein's animation descends the short central stem, then separately
  // traces the closed body counterclockwise before the final headline:
  // three strokes, two lifts.
    ["devanagari:ठ", {
    script: "devanagari",
    glyph: "ठ",
    strokes: [
      { segments: [{ label: "descend the short central stem", path: [
        { x: 350, y: 551 }, { x: 350, y: 510 }, { x: 350, y: 470 },
        { x: 350, y: 430 }, { x: 350, y: 390 },
      ] }] },
      { segments: [{ label: "lift, then trace the closed body counterclockwise", path: [
        { x: 350, y: 390 }, { x: 305, y: 390 }, { x: 260, y: 388 },
        { x: 215, y: 380 }, { x: 170, y: 362 }, { x: 130, y: 335 },
        { x: 98, y: 300 }, { x: 75, y: 260 }, { x: 62, y: 215 },
        { x: 62, y: 170 }, { x: 75, y: 128 }, { x: 98, y: 90 },
        { x: 132, y: 58 }, { x: 175, y: 35 }, { x: 225, y: 20 },
        { x: 275, y: 15 }, { x: 325, y: 18 }, { x: 375, y: 30 },
        { x: 420, y: 52 }, { x: 458, y: 82 }, { x: 488, y: 118 },
        { x: 505, y: 160 }, { x: 510, y: 205 }, { x: 505, y: 250 },
        { x: 488, y: 292 }, { x: 460, y: 328 }, { x: 425, y: 358 },
        { x: 390, y: 378 }, { x: 350, y: 390 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 500, y: 585 }, { x: 585, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ठ"),
  }],
  // Opiaterein's animation keeps the descending right stem, upper-left loop,
  // and broad open lower bowl in one continuous S-shaped run, then finishes
  // with a separate headline: two strokes, one lift.
    ["devanagari:ड", {
    script: "devanagari",
    glyph: "ड",
    strokes: [
      { segments: [{ label: "descend the stem and sweep through the upper loop and open lower bowl", path: [
        { x: 415, y: 551 }, { x: 415, y: 500 }, { x: 415, y: 450 },
        { x: 415, y: 400 }, { x: 370, y: 400 }, { x: 325, y: 400 },
        { x: 280, y: 400 }, { x: 240, y: 400 }, { x: 195, y: 398 },
        { x: 155, y: 382 }, { x: 125, y: 355 }, { x: 108, y: 322 },
        { x: 110, y: 288 }, { x: 138, y: 252 }, { x: 160, y: 235 },
        { x: 198, y: 218 }, { x: 275, y: 220 },
        { x: 315, y: 238 }, { x: 360, y: 242 }, { x: 405, y: 230 },
        { x: 445, y: 205 }, { x: 472, y: 170 }, { x: 485, y: 130 },
        { x: 470, y: 100 }, { x: 450, y: 70 }, { x: 425, y: 32 },
        { x: 380, y: 17 }, { x: 325, y: 10 }, { x: 270, y: 12 },
        { x: 215, y: 24 }, { x: 165, y: 45 }, { x: 120, y: 72 },
        { x: 82, y: 102 }, { x: 55, y: 125 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 500, y: 585 }, { x: 555, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ड"),
  }],
  // Opiaterein's animation keeps the descending right stem, broad outer bowl,
  // and closed inner loop in one continuous run, then finishes with a separate
  // headline: two strokes, one lift.
    ["devanagari:ढ", {
    script: "devanagari",
    glyph: "ढ",
    strokes: [
      { segments: [{ label: "descend the stem and sweep through the outer bowl and inner loop", path: [
        { x: 415, y: 551 }, { x: 415, y: 500 }, { x: 415, y: 450 },
        { x: 415, y: 400 }, { x: 370, y: 400 }, { x: 325, y: 400 },
        { x: 280, y: 400 }, { x: 235, y: 395 }, { x: 190, y: 380 },
        { x: 150, y: 358 }, { x: 118, y: 330 }, { x: 95, y: 295 },
        { x: 80, y: 255 }, { x: 75, y: 210 }, { x: 82, y: 165 },
        { x: 100, y: 125 }, { x: 130, y: 90 }, { x: 170, y: 62 },
        { x: 215, y: 40 }, { x: 265, y: 27 }, { x: 315, y: 22 },
        { x: 365, y: 25 }, { x: 410, y: 38 }, { x: 450, y: 62 },
        { x: 478, y: 95 }, { x: 490, y: 135 }, { x: 488, y: 175 },
        { x: 475, y: 212 }, { x: 450, y: 242 }, { x: 418, y: 260 },
        { x: 380, y: 268 }, { x: 342, y: 262 }, { x: 310, y: 245 },
        { x: 285, y: 218 }, { x: 270, y: 185 }, { x: 264, y: 148 },
        { x: 268, y: 112 }, { x: 280, y: 78 }, { x: 300, y: 50 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 500, y: 585 }, { x: 555, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ढ"),
  }],
  // Opiaterein's animation joins the descending left stem, clockwise lower
  // bowl, and ascending inner right stem in one run, then separately descends
  // the outer right stem and finishes the headline: three strokes, two lifts.
    ["devanagari:ण", {
    script: "devanagari",
    glyph: "ण",
    strokes: [
      { segments: [{ label: "descend the left stem, curve around the bowl, and rise along the inner stem", path: [
        { x: 120, y: 551 }, { x: 120, y: 480 }, { x: 120, y: 410 },
        { x: 120, y: 355 }, { x: 128, y: 310 }, { x: 145, y: 270 },
        { x: 170, y: 238 }, { x: 205, y: 215 }, { x: 245, y: 202 },
        { x: 285, y: 202 }, { x: 325, y: 215 }, { x: 360, y: 238 },
        { x: 385, y: 270 }, { x: 400, y: 310 }, { x: 408, y: 355 },
        { x: 408, y: 410 }, { x: 408, y: 480 }, { x: 408, y: 551 },
      ] }] },
      { segments: [{ label: "lift, then descend the outer right stem", path: [
        { x: 600, y: 551 }, { x: 600, y: 475 }, { x: 600, y: 395 },
        { x: 600, y: 315 }, { x: 600, y: 235 }, { x: 600, y: 155 },
        { x: 600, y: 75 }, { x: 600, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 585, y: 585 }, { x: 670, y: 585 },
        { x: 730, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ण"),
  }],
  // Opiaterein's animation sweeps the shoulder right-to-left and carries the
  // same run down around the open body, then separately descends the right stem
  // and finishes the headline. The Central Hindi Directorate deskbook shows
  // the same three-part buildup: three strokes, two lifts.
    ["devanagari:त", {
    script: "devanagari",
    glyph: "त",
    strokes: [
      { segments: [{ label: "sweep left across the shoulder and curve down to the open tip", path: [
        { x: 400, y: 364 }, { x: 350, y: 364 }, { x: 300, y: 364 },
        { x: 247, y: 364 }, { x: 205, y: 363 }, { x: 165, y: 345 },
        { x: 130, y: 315 }, { x: 105, y: 280 }, { x: 86, y: 242 },
        { x: 88, y: 205 }, { x: 103, y: 165 }, { x: 125, y: 125 },
        { x: 152, y: 88 }, { x: 184, y: 52 }, { x: 219, y: 14 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 440, y: 551 }, { x: 440, y: 475 }, { x: 440, y: 395 },
        { x: 440, y: 315 }, { x: 440, y: 235 }, { x: 440, y: 155 },
        { x: 440, y: 75 }, { x: 440, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 579, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("त"),
  }],
  // Opiaterein's animation keeps the upper spiral and broad lower bowl in one
  // continuous run, then separately descends the right stem and finishes the
  // headline: three strokes, two lifts.
    ["devanagari:थ", {
    script: "devanagari",
    glyph: "थ",
    strokes: [
      { segments: [{ label: "curl around the upper spiral and continue around the broad lower bowl", path: [
        { x: 200, y: 445 }, { x: 170, y: 425 }, { x: 135, y: 415 },
        { x: 104, y: 424 }, { x: 81, y: 448 }, { x: 69, y: 470 },
        { x: 70, y: 500 }, { x: 90, y: 525 }, { x: 120, y: 540 },
        { x: 147, y: 546 }, { x: 180, y: 566 }, { x: 220, y: 570 },
        { x: 260, y: 565 }, { x: 300, y: 545 }, { x: 330, y: 515 },
        { x: 345, y: 480 }, { x: 350, y: 445 }, { x: 341, y: 410 },
        { x: 328, y: 382 }, { x: 305, y: 355 }, { x: 275, y: 338 },
        { x: 240, y: 328 }, { x: 200, y: 320 }, { x: 160, y: 315 },
        { x: 120, y: 318 }, { x: 90, y: 323 }, { x: 58, y: 329 },
        { x: 74, y: 285 }, { x: 94, y: 242 }, { x: 124, y: 199 },
        { x: 162, y: 164 }, { x: 207, y: 139 }, { x: 255, y: 126 }, { x: 305, y: 122 },
        { x: 355, y: 135 }, { x: 400, y: 160 }, { x: 435, y: 195 },
        { x: 460, y: 230 }, { x: 480, y: 260 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 508, y: 551 }, { x: 508, y: 475 }, { x: 508, y: 395 },
        { x: 508, y: 315 }, { x: 508, y: 235 }, { x: 508, y: 155 },
        { x: 508, y: 75 }, { x: 508, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 394, y: 585 }, { x: 435, y: 585 }, { x: 475, y: 585 },
        { x: 515, y: 585 }, { x: 555, y: 585 }, { x: 600, y: 585 },
        { x: 652, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("थ"),
  }],
  // Opiaterein's animation descends the short stem, then joins the outer body
  // directly through the inward curl and tail before the final headline. The
  // Central Hindi Directorate deskbook corroborates component order while
  // staging the body and curl-tail separately: three animated strokes, two lifts.
    ["devanagari:द", {
    script: "devanagari",
    glyph: "द",
    strokes: [
      { segments: [{ label: "descend the short stem", path: [
        { x: 395, y: 551 }, { x: 395, y: 505 }, { x: 395, y: 460 },
        { x: 395, y: 420 },
      ] }] },
      { segments: [{ label: "lift, then sweep around the body, inner curl, and tail", path: [
        { x: 395, y: 420 }, { x: 350, y: 420 }, { x: 300, y: 420 },
        { x: 245, y: 418 }, { x: 190, y: 400 }, { x: 145, y: 370 },
        { x: 110, y: 335 }, { x: 90, y: 295 }, { x: 90, y: 255 },
        { x: 95, y: 210 }, { x: 125, y: 170 }, { x: 170, y: 140 },
        { x: 215, y: 115 }, { x: 260, y: 110 }, { x: 300, y: 112 },
        { x: 340, y: 118 }, { x: 385, y: 130 }, { x: 420, y: 155 },
        { x: 440, y: 185 }, { x: 435, y: 215 }, { x: 410, y: 235 },
        { x: 380, y: 235 }, { x: 355, y: 220 }, { x: 348, y: 195 },
        { x: 355, y: 170 }, { x: 375, y: 150 }, { x: 400, y: 128 },
        { x: 415, y: 98 }, { x: 435, y: 55 }, { x: 458, y: 10 },
        { x: 482, y: -38 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 80, y: 585 }, { x: 155, y: 585 },
        { x: 230, y: 585 }, { x: 305, y: 585 }, { x: 380, y: 585 },
        { x: 455, y: 585 }, { x: 536, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("द"),
  }],
  // Opiaterein's animation writes the upper spiral and shoulder, lower bowl,
  // right stem, and headline as four separate runs. The Central Hindi
  // Directorate deskbook independently shows the same buildup: three lifts.
    ["devanagari:ध", {
    script: "devanagari",
    glyph: "ध",
    strokes: [
      { segments: [{ label: "curl around the upper spiral and sweep right through the shoulder", path: [
        { x: 285, y: 450 }, { x: 300, y: 475 }, { x: 305, y: 505 },
        { x: 300, y: 535 }, { x: 285, y: 560 }, { x: 260, y: 585 },
        { x: 225, y: 600 }, { x: 185, y: 605 }, { x: 145, y: 590 },
        { x: 110, y: 565 }, { x: 85, y: 530 }, { x: 75, y: 490 },
        { x: 80, y: 450 }, { x: 95, y: 420 }, { x: 115, y: 395 },
        { x: 145, y: 375 }, { x: 175, y: 355 }, { x: 210, y: 340 },
        { x: 250, y: 335 }, { x: 290, y: 340 }, { x: 325, y: 350 },
      ] }] },
      { segments: [{ label: "lift, then sweep down and around the lower bowl", path: [
        { x: 170, y: 330 }, { x: 140, y: 320 }, { x: 125, y: 295 },
        { x: 125, y: 265 }, { x: 130, y: 230 }, { x: 140, y: 195 },
        { x: 155, y: 165 }, { x: 160, y: 140 }, { x: 205, y: 120 },
        { x: 250, y: 112 }, { x: 300, y: 112 }, { x: 350, y: 122 },
        { x: 395, y: 145 }, { x: 435, y: 180 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 485, y: 551 }, { x: 485, y: 475 }, { x: 485, y: 395 },
        { x: 485, y: 315 }, { x: 485, y: 235 }, { x: 485, y: 155 },
        { x: 485, y: 75 }, { x: 485, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 388, y: 585 }, { x: 430, y: 585 }, { x: 475, y: 585 },
        { x: 520, y: 585 }, { x: 570, y: 585 }, { x: 625, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ध"),
  }],
  // Opiaterein's animation writes the clockwise loop and rightward shoulder,
  // right stem, and headline as three separate runs. The Central Hindi
  // Directorate deskbook independently shows the same buildup: two lifts.
    ["devanagari:न", {
    script: "devanagari",
    glyph: "न",
    strokes: [
      { segments: [{ label: "circle clockwise around the left loop and sweep right", path: [
        { x: 185, y: 255 }, { x: 178, y: 225 }, { x: 158, y: 205 },
        { x: 130, y: 202 }, { x: 100, y: 215 }, { x: 72, y: 242 },
        { x: 52, y: 275 }, { x: 48, y: 310 }, { x: 58, y: 338 },
        { x: 82, y: 350 }, { x: 115, y: 350 }, { x: 155, y: 335 },
        { x: 205, y: 335 }, { x: 260, y: 335 }, { x: 320, y: 335 },
        { x: 380, y: 335 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 425, y: 551 }, { x: 425, y: 475 }, { x: 425, y: 395 },
        { x: 425, y: 315 }, { x: 425, y: 235 }, { x: 425, y: 155 },
        { x: 425, y: 75 }, { x: 425, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 80, y: 585 }, { x: 155, y: 585 },
        { x: 230, y: 585 }, { x: 305, y: 585 }, { x: 380, y: 585 },
        { x: 455, y: 585 }, { x: 565, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("न"),
  }],
  // Opiaterein's animation descends the left stem and curves right around the
  // lower bowl, then separately descends the right stem and finishes the
  // headline. The Central Hindi Directorate deskbook independently shows the
  // same three-part buildup and directions: three strokes, two lifts.
    ["devanagari:प", {
    script: "devanagari",
    glyph: "प",
    strokes: [
      { segments: [{ label: "descend the left stem and curve right around the lower bowl", path: [
        { x: 120, y: 551 }, { x: 120, y: 480 }, { x: 120, y: 410 },
        { x: 120, y: 355 }, { x: 128, y: 310 }, { x: 145, y: 270 },
        { x: 170, y: 238 }, { x: 205, y: 215 }, { x: 245, y: 202 },
        { x: 285, y: 202 }, { x: 325, y: 215 }, { x: 360, y: 238 },
        { x: 385, y: 265 }, { x: 402, y: 295 }, { x: 408, y: 320 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 438, y: 551 }, { x: 438, y: 475 }, { x: 438, y: 395 },
        { x: 438, y: 315 }, { x: 438, y: 235 }, { x: 438, y: 155 },
        { x: 438, y: 75 }, { x: 438, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 578, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("प"),
  }],
  // JackPotte's animation joins the descending left stem, lower bowl, rising
  // central side, and descending central stem in one retraced run, then adds
  // the right arch and headline separately: three strokes, two lifts.
    ["devanagari:फ", {
    script: "devanagari",
    glyph: "फ",
    strokes: [
      { segments: [{ label: "descend around the lower bowl, rise, and retrace down the central stem", path: [
        { x: 120, y: 551 }, { x: 120, y: 480 }, { x: 120, y: 410 },
        { x: 120, y: 350 }, { x: 128, y: 300 }, { x: 145, y: 255 },
        { x: 170, y: 220 }, { x: 205, y: 195 }, { x: 245, y: 182 },
        { x: 285, y: 182 }, { x: 325, y: 195 }, { x: 360, y: 220 },
        { x: 388, y: 255 }, { x: 408, y: 300 }, { x: 420, y: 350 },
        { x: 420, y: 410 }, { x: 420, y: 475 }, { x: 420, y: 551 },
        { x: 420, y: 475 }, { x: 420, y: 395 }, { x: 420, y: 315 },
        { x: 420, y: 235 }, { x: 420, y: 155 }, { x: 420, y: 75 },
        { x: 420, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then sweep clockwise through the right arch", path: [
        { x: 420, y: 315 }, { x: 470, y: 335 }, { x: 520, y: 350 },
        { x: 575, y: 355 }, { x: 620, y: 340 }, { x: 655, y: 315 },
        { x: 670, y: 290 }, { x: 676, y: 260 }, { x: 687, y: 215 },
        { x: 685, y: 170 }, { x: 671, y: 125 }, { x: 650, y: 85 },
        { x: 626, y: 52 }, { x: 637, y: 45 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 565, y: 585 }, { x: 650, y: 585 },
        { x: 730, y: 585 }, { x: 785, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("फ"),
  }],
  // JackPotte's animation circles counterclockwise around the oval, then
  // separately descends the right stem, crosses down-right through the body,
  // and finishes the headline. The Central Hindi Directorate deskbook shows
  // the same four-part buildup and directions: four strokes, three lifts.
    ["devanagari:ब", {
    script: "devanagari",
    glyph: "ब",
    strokes: [
      { segments: [{ label: "circle counterclockwise around the oval body", path: [
        { x: 350, y: 390 }, { x: 320, y: 415 }, { x: 275, y: 432 }, { x: 225, y: 432 },
        { x: 175, y: 415 }, { x: 135, y: 385 }, { x: 105, y: 345 },
        { x: 88, y: 300 }, { x: 88, y: 255 }, { x: 105, y: 215 },
        { x: 135, y: 182 }, { x: 175, y: 158 }, { x: 225, y: 147 },
        { x: 275, y: 150 }, { x: 320, y: 168 }, { x: 350, y: 198 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 442, y: 551 }, { x: 442, y: 475 }, { x: 442, y: 395 },
        { x: 442, y: 315 }, { x: 442, y: 235 }, { x: 442, y: 155 },
        { x: 442, y: 75 }, { x: 442, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then cross the body down and right", path: [
        { x: 175, y: 405 }, { x: 205, y: 365 }, { x: 235, y: 325 },
        { x: 265, y: 285 }, { x: 295, y: 245 }, { x: 325, y: 205 },
        { x: 354, y: 176 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 580, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ब"),
  }],
  // JackPotte's animation keeps the clockwise upper loop, descending trunk,
  // clockwise lower bowl, and rightward crossbar in one continuous run, then
  // separately descends the right stem and finishes the headline. The Central
  // Hindi Directorate deskbook confirms the component order but stages the two
  // body parts separately: three animation-backed strokes, two lifts.
    ["devanagari:भ", {
    script: "devanagari",
    glyph: "भ",
    strokes: [
      { segments: [{ label: "circle clockwise through both loops and sweep right", path: [
        { x: 200, y: 410 }, { x: 165, y: 414 }, { x: 135, y: 425 },
        { x: 95, y: 455 }, { x: 75, y: 495 },
        { x: 78, y: 540 }, { x: 100, y: 575 }, { x: 135, y: 595 },
        { x: 180, y: 602 }, { x: 225, y: 592 }, { x: 260, y: 565 },
        { x: 285, y: 528 }, { x: 292, y: 485 }, { x: 285, y: 450 },
        { x: 292, y: 405 }, { x: 292, y: 360 }, { x: 292, y: 315 },
        { x: 292, y: 265 }, { x: 286, y: 220 }, { x: 260, y: 188 },
        { x: 225, y: 184 }, { x: 205, y: 215 }, { x: 180, y: 242 },
        { x: 182, y: 278 }, { x: 200, y: 300 }, { x: 235, y: 285 },
        { x: 275, y: 285 }, { x: 325, y: 285 }, { x: 380, y: 285 },
        { x: 440, y: 285 }, { x: 495, y: 285 }, { x: 530, y: 285 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 575, y: 551 }, { x: 575, y: 475 }, { x: 575, y: 395 },
        { x: 575, y: 315 }, { x: 575, y: 235 }, { x: 575, y: 155 },
        { x: 575, y: 75 }, { x: 575, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 405, y: 585 }, { x: 455, y: 585 }, { x: 510, y: 585 },
        { x: 565, y: 585 }, { x: 620, y: 585 }, { x: 675, y: 585 },
        { x: 715, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("भ"),
  }],
  // JackPotte's animation joins the descending left stem directly to the
  // clockwise lower loop and rightward crossbar, then separately descends the
  // right stem and finishes the headline. The Central Hindi Directorate
  // deskbook confirms the component order but stages the left stem and lower
  // body separately: three animation-backed strokes, two lifts.
    ["devanagari:म", {
    script: "devanagari",
    glyph: "म",
    strokes: [
      { segments: [{ label: "descend the left stem, circle clockwise through the loop, and sweep right", path: [
        { x: 166, y: 551 }, { x: 166, y: 475 }, { x: 166, y: 405 },
        { x: 166, y: 350 }, { x: 167, y: 315 }, { x: 167, y: 265 },
        { x: 161, y: 220 }, { x: 135, y: 188 }, { x: 100, y: 184 },
        { x: 80, y: 215 }, { x: 55, y: 242 }, { x: 57, y: 278 },
        { x: 75, y: 300 }, { x: 110, y: 285 }, { x: 150, y: 285 },
        { x: 200, y: 285 }, { x: 255, y: 285 }, { x: 315, y: 285 },
        { x: 370, y: 285 }, { x: 405, y: 285 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 468, y: 551 }, { x: 468, y: 475 }, { x: 468, y: 395 },
        { x: 468, y: 315 }, { x: 468, y: 235 }, { x: 468, y: 155 },
        { x: 468, y: 75 }, { x: 468, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 610, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("म"),
  }],
  // Opiaterein's animation and the Central Hindi Directorate deskbook agree
  // on four runs: the clockwise inner curl, the restarted lower bowl, the
  // descending right stem, and the headline. JackPotte documents a joined
  // two-lift body as a real variation; this path follows the corroborated
  // four-stroke form.
    ["devanagari:य", {
    script: "devanagari",
    glyph: "य",
    strokes: [
      { segments: [{ label: "curve clockwise around the inner curl", path: [
        { x: 165, y: 551 }, { x: 195, y: 540 }, { x: 215, y: 510 },
        { x: 220, y: 475 }, { x: 207, y: 440 }, { x: 185, y: 410 },
        { x: 150, y: 382 }, { x: 105, y: 360 }, { x: 55, y: 355 },
      ] }] },
      { segments: [{ label: "lift, then curve around the lower bowl to the right", path: [
        { x: 55, y: 350 }, { x: 80, y: 310 }, { x: 110, y: 270 },
        { x: 150, y: 235 }, { x: 205, y: 190 }, { x: 270, y: 165 },
        { x: 335, y: 173 }, { x: 385, y: 202 }, { x: 425, y: 245 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 450, y: 551 }, { x: 450, y: 475 }, { x: 450, y: 395 },
        { x: 450, y: 315 }, { x: 450, y: 235 }, { x: 450, y: 155 },
        { x: 450, y: 75 }, { x: 450, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 590, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("य"),
  }],
  // Opiaterein's animation and the Central Hindi Directorate deskbook agree
  // on three runs: the descending stem and clockwise lower loop, the restarted
  // diagonal tail, and the headline. JackPotte documents a joined loop-and-tail
  // variation; this path follows the corroborated three-stroke form.
    ["devanagari:र", {
    script: "devanagari",
    glyph: "र",
    strokes: [
      { segments: [{ label: "descend and curl clockwise around the lower loop", path: [
        { x: 285, y: 551 }, { x: 285, y: 510 }, { x: 286, y: 470 },
        { x: 280, y: 430 }, { x: 265, y: 390 }, { x: 245, y: 360 },
        { x: 220, y: 340 }, { x: 195, y: 325 }, { x: 175, y: 342 },
        { x: 150, y: 360 }, { x: 120, y: 375 }, { x: 90, y: 370 },
        { x: 65, y: 350 }, { x: 55, y: 325 }, { x: 63, y: 300 },
        { x: 82, y: 280 }, { x: 105, y: 260 }, { x: 130, y: 245 },
        { x: 155, y: 245 },
      ] }] },
      { segments: [{ label: "lift, then draw the diagonal tail down-right", path: [
        { x: 145, y: 235 }, { x: 170, y: 205 }, { x: 200, y: 170 },
        { x: 235, y: 135 }, { x: 270, y: 100 }, { x: 305, y: 65 },
        { x: 345, y: 30 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 420, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("र"),
  }],
  // Opiaterein's animation and the Central Hindi Directorate deskbook agree
  // on four runs: the clockwise open loop, diagonal arm, descending right stem,
  // and headline. JackPotte documents a stem-first order variation; this path
  // follows the corroborated loop-first form.
    ["devanagari:ल", {
    script: "devanagari",
    glyph: "ल",
    strokes: [
      { segments: [{ label: "curve up and clockwise around the open left loop", path: [
        { x: 255, y: 5 }, { x: 220, y: 25 }, { x: 185, y: 50 },
        { x: 150, y: 80 }, { x: 120, y: 115 }, { x: 95, y: 155 },
        { x: 75, y: 205 }, { x: 70, y: 255 }, { x: 82, y: 305 },
        { x: 110, y: 345 }, { x: 150, y: 375 }, { x: 195, y: 392 },
        { x: 235, y: 392 }, { x: 270, y: 380 }, { x: 300, y: 360 },
        { x: 325, y: 330 }, { x: 345, y: 295 }, { x: 350, y: 260 },
        { x: 330, y: 240 }, { x: 300, y: 260 },
      ] }] },
      { segments: [{ label: "lift, then sweep the diagonal arm up-right", path: [
        { x: 300, y: 260 }, { x: 320, y: 280 }, { x: 340, y: 300 },
        { x: 365, y: 325 }, { x: 390, y: 350 }, { x: 420, y: 375 },
        { x: 455, y: 393 }, { x: 495, y: 395 }, { x: 510, y: 395 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 548, y: 551 }, { x: 548, y: 475 }, { x: 548, y: 395 },
        { x: 548, y: 315 }, { x: 548, y: 235 }, { x: 548, y: 155 },
        { x: 548, y: 75 }, { x: 548, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 565, y: 585 }, { x: 690, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ल"),
  }],
  // Hela Nomad's published stroke data keeps both loops in one figure-eight
  // body, then separately descends the short upper stem and draws the headline:
  // three strokes, two lifts.
    ["devanagari:ळ", {
    script: "devanagari",
    glyph: "ळ",
    strokes: [
      { segments: [{ label: "sweep through the joined left and right loops", path: [
        { x: 389, y: 295 }, { x: 355, y: 332 }, { x: 310, y: 370 },
        { x: 258, y: 397 }, { x: 198, y: 403 }, { x: 143, y: 385 },
        { x: 102, y: 345 }, { x: 84, y: 291 }, { x: 80, y: 233 },
        { x: 94, y: 177 }, { x: 128, y: 131 }, { x: 178, y: 103 },
        { x: 236, y: 100 }, { x: 289, y: 123 }, { x: 330, y: 165 },
        { x: 358, y: 216 }, { x: 384, y: 269 }, { x: 414, y: 320 },
        { x: 448, y: 367 }, { x: 498, y: 397 }, { x: 557, y: 405 },
        { x: 613, y: 389 }, { x: 655, y: 350 }, { x: 674, y: 295 },
        { x: 680, y: 237 }, { x: 672, y: 180 }, { x: 642, y: 131 },
        { x: 592, y: 103 }, { x: 533, y: 95 }, { x: 475, y: 106 },
        { x: 426, y: 137 }, { x: 392, y: 184 }, { x: 375, y: 214 },
      ] }] },
      { segments: [{ label: "lift, then descend the short upper stem", path: [
        { x: 550, y: 551 }, { x: 550, y: 515 }, { x: 550, y: 480 },
        { x: 550, y: 445 }, { x: 550, y: 410 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 85, y: 585 }, { x: 165, y: 585 },
        { x: 245, y: 585 }, { x: 325, y: 585 }, { x: 405, y: 585 },
        { x: 485, y: 585 }, { x: 565, y: 585 }, { x: 650, y: 585 },
        { x: 720, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ळ"),
  }],
  // JackPotte's animation and the Central Hindi Directorate deskbook agree on
  // three parts: the counterclockwise loop, descending right stem, and final
  // headline. The animation supplies the within-run directions and two lifts.
    ["devanagari:व", {
    script: "devanagari",
    glyph: "व",
    strokes: [
      { segments: [{ label: "circle counterclockwise around the left loop", path: [
        { x: 350, y: 415 }, { x: 305, y: 428 }, { x: 255, y: 430 },
        { x: 205, y: 422 }, { x: 160, y: 402 }, { x: 125, y: 375 },
        { x: 100, y: 340 }, { x: 87, y: 300 }, { x: 88, y: 260 },
        { x: 105, y: 220 }, { x: 140, y: 185 }, { x: 190, y: 160 },
        { x: 240, y: 150 }, { x: 290, y: 158 }, { x: 335, y: 180 },
        { x: 370, y: 215 }, { x: 392, y: 260 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 427, y: 551 }, { x: 427, y: 475 }, { x: 427, y: 395 },
        { x: 427, y: 315 }, { x: 427, y: 235 }, { x: 427, y: 155 },
        { x: 427, y: 75 }, { x: 427, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 495, y: 585 }, { x: 565, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("व"),
  }],
  // Both animations and the Central Hindi Directorate deskbook agree on three
  // parts: one joined double-loop body and tail, the descending right stem,
  // and the final headline. Opiaterein's holds make the two lifts explicit.
    ["devanagari:श", {
    script: "devanagari",
    glyph: "श",
    strokes: [
      { segments: [{ label: "trace the joined double-loop body and diagonal tail", path: [
        { x: 240, y: 380 }, { x: 220, y: 395 }, { x: 175, y: 410 },
        { x: 135, y: 440 }, { x: 110, y: 480 }, { x: 105, y: 520 },
        { x: 120, y: 560 }, { x: 155, y: 590 }, { x: 200, y: 605 },
        { x: 245, y: 600 }, { x: 285, y: 580 }, { x: 315, y: 545 },
        { x: 335, y: 500 }, { x: 340, y: 450 }, { x: 330, y: 400 },
        { x: 310, y: 350 }, { x: 275, y: 310 }, { x: 235, y: 280 },
        { x: 190, y: 260 }, { x: 175, y: 265 }, { x: 155, y: 278 },
        { x: 115, y: 290 }, { x: 75, y: 280 }, { x: 50, y: 255 },
        { x: 55, y: 225 }, { x: 85, y: 200 }, { x: 125, y: 192 },
        { x: 165, y: 210 }, { x: 200, y: 245 }, { x: 205, y: 220 },
        { x: 235, y: 185 }, { x: 270, y: 145 }, { x: 330, y: 80 },
        { x: 350, y: 25 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 550, y: 550 }, { x: 550, y: 475 }, { x: 550, y: 395 },
        { x: 550, y: 315 }, { x: 550, y: 235 }, { x: 550, y: 155 },
        { x: 550, y: 75 }, { x: 550, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 425, y: 585 }, { x: 480, y: 585 }, { x: 535, y: 585 },
        { x: 590, y: 585 }, { x: 645, y: 585 }, { x: 690, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("श"),
  }],
  // Opiaterein's animation draws the U-shaped body first, then separately
  // retraces and descends its right stem, adds the diagonal, and finishes the
  // headline: four strokes, three lifts.
    ["devanagari:ष", {
    script: "devanagari",
    glyph: "ष",
    strokes: [
      { segments: [{ label: "descend the left side, curve around the bowl, and rise along the right side", path: [
        { x: 120, y: 551 }, { x: 120, y: 480 }, { x: 120, y: 410 },
        { x: 120, y: 350 }, { x: 128, y: 300 }, { x: 145, y: 255 },
        { x: 170, y: 220 }, { x: 205, y: 195 }, { x: 245, y: 182 },
        { x: 285, y: 182 }, { x: 325, y: 195 }, { x: 360, y: 220 },
        { x: 388, y: 255 }, { x: 408, y: 300 }, { x: 420, y: 350 },
        { x: 420, y: 410 }, { x: 420, y: 475 }, { x: 420, y: 551 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 420, y: 551 }, { x: 420, y: 475 }, { x: 420, y: 395 },
        { x: 420, y: 315 }, { x: 420, y: 235 }, { x: 420, y: 155 },
        { x: 420, y: 75 }, { x: 420, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the inner diagonal down-right", path: [
        { x: 175, y: 530 }, { x: 220, y: 475 }, { x: 270, y: 415 },
        { x: 320, y: 355 }, { x: 370, y: 295 }, { x: 410, y: 245 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 500, y: 585 }, { x: 592, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ष"),
  }],
  // JackPotte's animation joins the descending left stem, hook, and tail, then
  // restarts for the crossbar, right stem, and headline. The Directorate
  // deskbook confirms that order while staging the hook and tail separately.
    ["devanagari:स", {
    script: "devanagari",
    glyph: "स",
    strokes: [
      { segments: [{ label: "descend through the hook and diagonal tail", path: [
        { x: 250, y: 540 }, { x: 255, y: 505 }, { x: 265, y: 465 },
        { x: 265, y: 425 }, { x: 255, y: 385 }, { x: 235, y: 350 },
        { x: 205, y: 320 }, { x: 170, y: 305 }, { x: 135, y: 315 },
        { x: 100, y: 340 }, { x: 70, y: 340 }, { x: 60, y: 315 },
        { x: 75, y: 285 }, { x: 110, y: 260 }, { x: 135, y: 240 },
        { x: 140, y: 210 }, { x: 155, y: 180 }, { x: 180, y: 145 },
        { x: 210, y: 110 }, { x: 240, y: 75 }, { x: 265, y: 35 },
        { x: 285, y: 0 },
      ] }] },
      { segments: [{ label: "lift, then draw the middle crossbar left-to-right", path: [
        { x: 230, y: 300 }, { x: 280, y: 285 }, { x: 340, y: 280 },
        { x: 400, y: 280 }, { x: 460, y: 285 }, { x: 520, y: 300 },
        { x: 550, y: 310 },
      ] }] },
      { segments: [{ label: "lift, then descend the right stem", path: [
        { x: 546, y: 550 }, { x: 546, y: 475 }, { x: 546, y: 395 },
        { x: 546, y: 315 }, { x: 546, y: 235 }, { x: 546, y: 155 },
        { x: 546, y: 75 }, { x: 546, y: 5 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 75, y: 585 }, { x: 145, y: 585 },
        { x: 215, y: 585 }, { x: 285, y: 585 }, { x: 355, y: 585 },
        { x: 425, y: 585 }, { x: 495, y: 585 }, { x: 565, y: 585 },
        { x: 635, y: 585 }, { x: 685, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("स"),
  }],
  // Opiaterein's animation joins the descending right stem, leftward shoulder,
  // and clockwise hooked body, then restarts for the down-right outer tail and
  // the headline. The Directorate deskbook confirms that component order while
  // staging the joined first body across more buildup steps.
    ["devanagari:ह", {
    script: "devanagari",
    glyph: "ह",
    strokes: [
      { segments: [{ label: "descend, sweep left, and curve around the hooked body", path: [
        { x: 402, y: 550 }, { x: 402, y: 510 }, { x: 402, y: 470 },
        { x: 402, y: 430 }, { x: 360, y: 430 }, { x: 315, y: 430 },
        { x: 270, y: 430 }, { x: 225, y: 430 }, { x: 180, y: 420 },
        { x: 140, y: 400 }, { x: 110, y: 370 }, { x: 105, y: 340 },
        { x: 110, y: 310 }, { x: 135, y: 285 }, { x: 175, y: 265 },
        { x: 225, y: 258 }, { x: 280, y: 260 }, { x: 335, y: 245 },
        { x: 385, y: 220 }, { x: 425, y: 185 }, { x: 445, y: 145 },
        { x: 445, y: 110 }, { x: 425, y: 70 }, { x: 390, y: 40 },
      ] }] },
      { segments: [{ label: "lift, then sweep down-left and through the diagonal tail", path: [
        { x: 150, y: 245 }, { x: 125, y: 220 }, { x: 100, y: 185 },
        { x: 88, y: 145 }, { x: 90, y: 105 }, { x: 110, y: 65 },
        { x: 145, y: 25 }, { x: 190, y: -10 }, { x: 240, y: -40 },
        { x: 295, y: -70 }, { x: 350, y: -100 },
      ] }] },
      { segments: [{ label: "lift, then draw the shirorekha left-to-right", path: [
        { x: 5, y: 585 }, { x: 70, y: 585 }, { x: 135, y: 585 },
        { x: 200, y: 585 }, { x: 265, y: 585 }, { x: 330, y: 585 },
        { x: 395, y: 585 }, { x: 460, y: 585 }, { x: 540, y: 585 },
      ] }] },
    ],
    source: devanagariAlphabetSource("ह"),
  }],
];
