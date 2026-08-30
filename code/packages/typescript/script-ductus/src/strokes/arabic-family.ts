// Authored arabic-family ductus records. This is the stable source-ownership boundary.

import type { LetterDuctus, Stroke, StrokeSource } from "../strokes.ts";
import type { DuctusEntry } from "./registry.ts";
import arabic from "../../../../../learning/human-languages/data/scripts/arabic.json";
import { SCRIPTS, type ScriptData } from "../scriptdata.ts";

const canonicalScript = (id: string): ScriptData => {
  const inventory = SCRIPTS.find((candidate) => candidate.script === id);
  if (inventory === undefined)
    throw new Error(`Script Ductus has no ${id} inventory`);
  return inventory;
};

const persoArabic = canonicalScript("perso-arabic");

const urduNastaliq = canonicalScript("urdu-nastaliq");

const arabicAlphabetSource = (glyph: string): StrokeSource => {
  const letter = arabic.letters.find((candidate) => candidate.glyph === glyph);
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Arabic ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const arabicLigatureSource = (sequence: string): StrokeSource => {
  const ligature = arabic.ligatures.find(
    (candidate) => candidate.sequence === sequence,
  );
  if (!ligature?.strokeOrderSource) {
    throw new Error(`Arabic ${sequence} has no verified ligature source`);
  }
  return ligature.strokeOrderSource;
};

const persianAlphabetSource = (glyph: string): StrokeSource => {
  const letter = persoArabic.letters.find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Persian ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const urduAlphabetSource = (glyph: string): StrokeSource => {
  const letter = urduNastaliq.letters.find(
    (candidate) => candidate.glyph === glyph,
  );
  if (
    !letter ||
    !("strokeOrderSource" in letter) ||
    !letter.strokeOrderSource
  ) {
    throw new Error(`Urdu ${glyph} has no verified source`);
  }
  return letter.strokeOrderSource;
};

const independentKhehStrokes = (): Stroke[] => [
  {
    segments: [
      {
        label: "draw the short upper head from left to right",
        path: [
          { x: 110, y: 315 },
          { x: 150, y: 335 },
          { x: 210, y: 340 },
          { x: 280, y: 325 },
          { x: 350, y: 305 },
          { x: 420, y: 285 },
          { x: 490, y: 270 },
          { x: 540, y: 270 },
        ],
      },
      {
        label: "continue down and around the deep bowl",
        path: [
          { x: 540, y: 270 },
          { x: 490, y: 270 },
          { x: 420, y: 285 },
          { x: 350, y: 305 },
          { x: 280, y: 325 },
          { x: 210, y: 340 },
          { x: 150, y: 335 },
          { x: 110, y: 315 },
          { x: 100, y: 290 },
          { x: 130, y: 305 },
          { x: 170, y: 310 },
          { x: 220, y: 305 },
          { x: 270, y: 285 },
          { x: 320, y: 265 },
          { x: 300, y: 245 },
          { x: 260, y: 220 },
          { x: 216, y: 190 },
          { x: 180, y: 130 },
          { x: 145, y: 65 },
          { x: 118, y: -42 },
          { x: 130, y: -110 },
          { x: 180, y: -175 },
          { x: 225, y: -200 },
          { x: 300, y: -245 },
          { x: 400, y: -245 },
          { x: 500, y: -230 },
          { x: 575, y: -210 },
          { x: 608, y: -195 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: "lift once, then place the dot above",
        path: [
          { x: 340, y: 460 },
          { x: 285, y: 510 },
          { x: 338, y: 565 },
          { x: 390, y: 515 },
          { x: 340, y: 460 },
        ],
      },
    ],
  },
];

const independentHahStrokes = (headLabel: string): Stroke[] => {
  const body = independentKhehStrokes()[0]!;
  return [
    {
      segments: [
        { ...body.segments[0]!, label: headLabel },
        {
          ...body.segments[1]!,
          label: "continue down and around the deep bowl without lifting",
        },
      ],
    },
  ];
};

const independentFehStrokes = (
  headLabel: string,
  bodyLabel: string,
): Stroke[] => [
  {
    segments: [
      {
        label: headLabel,
        path: [
          { x: 735, y: 250 },
          { x: 750, y: 330 },
          { x: 710, y: 400 },
          { x: 640, y: 435 },
          { x: 570, y: 410 },
          { x: 520, y: 350 },
          { x: 520, y: 285 },
          { x: 560, y: 230 },
          { x: 640, y: 215 },
          { x: 715, y: 240 },
          { x: 735, y: 250 },
        ],
      },
      {
        label: bodyLabel,
        path: [
          { x: 735, y: 250 },
          { x: 775, y: 230 },
          { x: 785, y: 180 },
          { x: 760, y: 110 },
          { x: 700, y: 80 },
          { x: 600, y: 60 },
          { x: 480, y: 55 },
          { x: 350, y: 40 },
          { x: 230, y: 55 },
          { x: 140, y: 105 },
          { x: 95, y: 170 },
          { x: 90, y: 240 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: "lift once, then place the upper dot last",
        path: [
          { x: 615, y: 550 },
          { x: 560, y: 607 },
          { x: 615, y: 664 },
          { x: 670, y: 607 },
          { x: 615, y: 550 },
        ],
      },
    ],
  },
];

const independentQafStrokes = (
  headLabel: string,
  bodyLabel: string,
  rightDotLabel: string,
  leftDotLabel: string,
): Stroke[] => [
  {
    segments: [
      {
        label: headLabel,
        path: [
          { x: 545, y: 160 },
          { x: 520, y: 235 },
          { x: 470, y: 285 },
          { x: 410, y: 300 },
          { x: 350, y: 270 },
          { x: 305, y: 220 },
          { x: 290, y: 165 },
          { x: 305, y: 105 },
          { x: 345, y: 55 },
          { x: 405, y: 40 },
          { x: 475, y: 55 },
          { x: 525, y: 100 },
          { x: 545, y: 160 },
        ],
      },
      {
        label: bodyLabel,
        path: [
          { x: 545, y: 160 },
          { x: 575, y: 105 },
          { x: 570, y: 35 },
          { x: 545, y: -55 },
          { x: 500, y: -130 },
          { x: 430, y: -185 },
          { x: 345, y: -220 },
          { x: 260, y: -225 },
          { x: 180, y: -205 },
          { x: 120, y: -155 },
          { x: 90, y: -95 },
          { x: 90, y: -35 },
          { x: 90, y: 35 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: rightDotLabel,
        path: [
          { x: 475, y: 405 },
          { x: 425, y: 457 },
          { x: 475, y: 510 },
          { x: 525, y: 457 },
          { x: 475, y: 405 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: leftDotLabel,
        path: [
          { x: 325, y: 390 },
          { x: 275, y: 442 },
          { x: 325, y: 495 },
          { x: 375, y: 442 },
          { x: 325, y: 390 },
        ],
      },
    ],
  },
];

const independentPehStrokes = (
  bowlLabel = "sweep the independent be-series bowl from right to left",
  leftDotLabel = "after one lift, place the lower-left dot nearer the main line",
  rightDotLabel = "after another lift, place the lower-right dot nearer the main line",
  centerDotLabel = "after a third lift, place the lower-center dot",
): Stroke[] => [
  {
    segments: [
      {
        label: bowlLabel,
        path: [
          { x: 678, y: 382 },
          { x: 663, y: 345 },
          { x: 650, y: 305 },
          { x: 654, y: 260 },
          { x: 672, y: 215 },
          { x: 688, y: 170 },
          { x: 686, y: 126 },
          { x: 620, y: 94 },
          { x: 530, y: 65 },
          { x: 430, y: 42 },
          { x: 335, y: 38 },
          { x: 245, y: 51 },
          { x: 170, y: 83 },
          { x: 120, y: 135 },
          { x: 96, y: 205 },
          { x: 100, y: 255 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: leftDotLabel,
        path: [
          { x: 350, y: -130 },
          { x: 317, y: -94 },
          { x: 282, y: -130 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: rightDotLabel,
        path: [
          { x: 493, y: -117 },
          { x: 460, y: -81 },
          { x: 425, y: -117 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: centerDotLabel,
        path: [
          { x: 420, y: -208 },
          { x: 390, y: -172 },
          { x: 356, y: -208 },
        ],
      },
    ],
  },
];

const independentUrduBehStrokes = (): Stroke[] => {
  const [bowl] = independentPehStrokes();
  return [
    bowl,
    {
      segments: [
        {
          label: "after one lift, place the single dot below",
          path: [
            { x: 412, y: -137 },
            { x: 379, y: -101 },
            { x: 344, y: -137 },
          ],
        },
      ],
    },
  ];
};

const independentUrduTeStrokes = (): Stroke[] => {
  const [bowl] = independentPehStrokes();
  return [
    bowl,
    {
      segments: [
        {
          label: "after one lift, place the left dot above the main line",
          path: [
            { x: 247, y: 374 },
            { x: 284, y: 412 },
            { x: 319, y: 379 },
          ],
        },
      ],
    },
    {
      segments: [
        {
          label: "after another lift, place the right dot beside it",
          path: [
            { x: 395, y: 389 },
            { x: 434, y: 430 },
            { x: 470, y: 395 },
          ],
        },
      ],
    },
  ];
};

const independentUrduTteStrokes = (): Stroke[] => {
  const [bowl] = independentPehStrokes();
  return [
    bowl,
    {
      segments: [
        {
          label:
            "after one lift, draw the small retroflex mark downward, back upward, and down again to close its loop",
          path: [
            { x: 340, y: 500 },
            { x: 335, y: 445 },
            { x: 340, y: 415 },
            { x: 335, y: 445 },
            { x: 340, y: 500 },
            { x: 335, y: 445 },
            { x: 340, y: 415 },
            { x: 325, y: 380 },
            { x: 280, y: 370 },
            { x: 325, y: 348 },
            { x: 375, y: 338 },
            { x: 425, y: 348 },
            { x: 460, y: 375 },
            { x: 470, y: 395 },
            { x: 450, y: 420 },
            { x: 405, y: 448 },
          ],
        },
      ],
    },
  ];
};

const independentCheStrokes = (
  headLabel = "sweep left through the pointed hooked head",
  bowlLabel = "continue down and around the bowl without lifting",
  leftDotLabel = "after one lift, place the lower-left dot",
  rightDotLabel = "after another lift, place the lower-right dot",
  centerDotLabel = "after a third lift, place the lower-center dot",
): Stroke[] => [
  {
    segments: [
      {
        label: headLabel,
        path: [
          { x: 540, y: 270 },
          { x: 490, y: 270 },
          { x: 420, y: 285 },
          { x: 350, y: 305 },
          { x: 280, y: 325 },
          { x: 210, y: 340 },
          { x: 150, y: 335 },
          { x: 110, y: 315 },
          { x: 100, y: 290 },
          { x: 130, y: 305 },
          { x: 170, y: 310 },
          { x: 220, y: 305 },
          { x: 270, y: 285 },
          { x: 320, y: 265 },
          { x: 300, y: 245 },
          { x: 260, y: 220 },
          { x: 216, y: 190 },
        ],
      },
      {
        label: bowlLabel,
        path: [
          { x: 216, y: 190 },
          { x: 180, y: 130 },
          { x: 145, y: 65 },
          { x: 118, y: -42 },
          { x: 130, y: -110 },
          { x: 180, y: -175 },
          { x: 225, y: -200 },
          { x: 300, y: -245 },
          { x: 400, y: -245 },
          { x: 500, y: -230 },
          { x: 575, y: -210 },
          { x: 608, y: -195 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: leftDotLabel,
        path: [
          { x: 302, y: 60 },
          { x: 350, y: 13 },
          { x: 304, y: -42 },
          { x: 250, y: 5 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: rightDotLabel,
        path: [
          { x: 446, y: 72 },
          { x: 493, y: 25 },
          { x: 447, y: -30 },
          { x: 395, y: 19 },
        ],
      },
    ],
  },
  {
    segments: [
      {
        label: centerDotLabel,
        path: [
          { x: 378, y: -110 },
          { x: 422, y: -68 },
          { x: 375, y: -20 },
          { x: 328, y: -68 },
        ],
      },
    ],
  },
];

const independentYehStrokes = (
  upperLabel = "descend from the upper right through the independent S curve",
  bowlLabel = "continue left around the below-baseline bowl and finish at its rising tip",
): Stroke[] => [
  {
    segments: [
      {
        label: upperLabel,
        path: [
          { x: 548, y: 285 },
          { x: 510, y: 288 },
          { x: 472, y: 270 },
          { x: 430, y: 238 },
          { x: 395, y: 205 },
          { x: 365, y: 168 },
          { x: 345, y: 125 },
          { x: 330, y: 82 },
          { x: 340, y: 45 },
          { x: 375, y: 25 },
          { x: 420, y: 8 },
          { x: 470, y: -2 },
          { x: 520, y: -24 },
          { x: 555, y: -55 },
        ],
      },
      {
        label: bowlLabel,
        path: [
          { x: 555, y: -55 },
          { x: 535, y: -98 },
          { x: 495, y: -145 },
          { x: 445, y: -188 },
          { x: 390, y: -218 },
          { x: 325, y: -238 },
          { x: 255, y: -238 },
          { x: 190, y: -218 },
          { x: 140, y: -180 },
          { x: 105, y: -130 },
          { x: 90, y: -78 },
          { x: 94, y: -25 },
          { x: 105, y: 28 },
        ],
      },
    ],
  },
];

export const mainEntries: DuctusEntry[] = [
  [
    "ا",
    {
      script: "perso-arabic",
      glyph: "ا",
      strokes: [
        {
          segments: [
            {
              label: "down",
              path: [
                { x: 120, y: 640 },
                { x: 120, y: 580 },
                { x: 119, y: 500 },
                { x: 124, y: 400 },
                { x: 128, y: 250 },
                { x: 129, y: 100 },
                { x: 127, y: 10 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ا"),
    },
  ],
  [
    "urdu-nastaliq:ا",
    {
      script: "urdu-nastaliq",
      glyph: "ا",
      strokes: [
        {
          segments: [
            {
              label: "down",
              path: [
                { x: 120, y: 640 },
                { x: 120, y: 580 },
                { x: 119, y: 500 },
                { x: 124, y: 400 },
                { x: 128, y: 250 },
                { x: 129, y: 100 },
                { x: 127, y: 10 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ا"),
    },
  ],
  [
    "arabic:ا",
    {
      script: "arabic",
      glyph: "ا",
      strokes: [
        {
          segments: [
            {
              label: "down",
              path: [
                { x: 120, y: 640 },
                { x: 120, y: 580 },
                { x: 119, y: 500 },
                { x: 124, y: 400 },
                { x: 128, y: 250 },
                { x: 129, y: 100 },
                { x: 127, y: 10 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ا"),
    },
  ],
  [
    "arabic:ب",
    {
      script: "arabic",
      glyph: "ب",
      strokes: [
        {
          segments: [
            {
              label: "sweep the shallow bowl from right to left",
              path: [
                { x: 678, y: 382 },
                { x: 663, y: 345 },
                { x: 650, y: 305 },
                { x: 654, y: 260 },
                { x: 672, y: 215 },
                { x: 688, y: 170 },
                { x: 686, y: 126 },
                { x: 620, y: 94 },
                { x: 530, y: 65 },
                { x: 430, y: 42 },
                { x: 335, y: 38 },
                { x: 245, y: 51 },
                { x: 170, y: 83 },
                { x: 120, y: 135 },
                { x: 96, y: 205 },
                { x: 100, y: 255 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the dot below",
              path: [
                { x: 412, y: -137 },
                { x: 379, y: -101 },
                { x: 344, y: -137 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ب"),
    },
  ],
  [
    "arabic:ت",
    {
      script: "arabic",
      glyph: "ت",
      strokes: [
        {
          segments: [
            {
              label: "sweep the shallow bowl from right to left",
              path: [
                { x: 678, y: 382 },
                { x: 663, y: 345 },
                { x: 650, y: 305 },
                { x: 654, y: 260 },
                { x: 672, y: 215 },
                { x: 688, y: 170 },
                { x: 686, y: 126 },
                { x: 620, y: 94 },
                { x: 530, y: 65 },
                { x: 430, y: 42 },
                { x: 335, y: 38 },
                { x: 245, y: 51 },
                { x: 170, y: 83 },
                { x: 120, y: 135 },
                { x: 96, y: 205 },
                { x: 100, y: 255 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the left dot above",
              path: [
                { x: 247, y: 374 },
                { x: 284, y: 412 },
                { x: 319, y: 379 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again and place the right dot",
              path: [
                { x: 395, y: 389 },
                { x: 434, y: 430 },
                { x: 470, y: 395 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ت"),
    },
  ],
  [
    "arabic:ث",
    {
      script: "arabic",
      glyph: "ث",
      strokes: [
        {
          segments: [
            {
              label: "sweep the shallow bowl from right to left",
              path: [
                { x: 678, y: 382 },
                { x: 663, y: 345 },
                { x: 650, y: 305 },
                { x: 654, y: 260 },
                { x: 672, y: 215 },
                { x: 688, y: 170 },
                { x: 686, y: 126 },
                { x: 620, y: 94 },
                { x: 530, y: 65 },
                { x: 430, y: 42 },
                { x: 335, y: 38 },
                { x: 245, y: 51 },
                { x: 170, y: 83 },
                { x: 120, y: 135 },
                { x: 96, y: 205 },
                { x: 100, y: 255 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the lower-left dot above",
              path: [
                { x: 247, y: 369 },
                { x: 295, y: 420 },
                { x: 340, y: 375 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again and place the lower-right dot",
              path: [
                { x: 390, y: 382 },
                { x: 438, y: 433 },
                { x: 483, y: 388 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift a third time and place the centred upper dot",
              path: [
                { x: 320, y: 458 },
                { x: 365, y: 504 },
                { x: 410, y: 458 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ث"),
    },
  ],
  [
    "arabic:ج",
    {
      script: "arabic",
      glyph: "ج",
      strokes: [
        {
          segments: [
            {
              label: "draw the short upper head from left to right",
              path: [
                { x: 110, y: 315 },
                { x: 150, y: 335 },
                { x: 210, y: 340 },
                { x: 280, y: 325 },
                { x: 350, y: 305 },
                { x: 420, y: 285 },
                { x: 490, y: 270 },
                { x: 540, y: 270 },
              ],
            },
            {
              label: "continue down and around the bowl",
              path: [
                { x: 540, y: 270 },
                { x: 490, y: 270 },
                { x: 420, y: 285 },
                { x: 350, y: 305 },
                { x: 280, y: 325 },
                { x: 210, y: 340 },
                { x: 150, y: 335 },
                { x: 110, y: 315 },
                { x: 100, y: 290 },
                { x: 130, y: 305 },
                { x: 170, y: 310 },
                { x: 220, y: 305 },
                { x: 270, y: 285 },
                { x: 320, y: 265 },
                { x: 300, y: 245 },
                { x: 260, y: 220 },
                { x: 216, y: 190 },
                { x: 180, y: 130 },
                { x: 145, y: 65 },
                { x: 118, y: -42 },
                { x: 130, y: -110 },
                { x: 180, y: -175 },
                { x: 225, y: -200 },
                { x: 300, y: -245 },
                { x: 400, y: -245 },
                { x: 500, y: -230 },
                { x: 575, y: -210 },
                { x: 608, y: -195 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the dot below",
              path: [
                { x: 415, y: -1 },
                { x: 374, y: 38 },
                { x: 330, y: -9 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ج"),
    },
  ],
  [
    "arabic:ح",
    {
      script: "arabic",
      glyph: "ح",
      strokes: [
        {
          segments: [
            {
              label: "draw the short left stem downward",
              path: [
                { x: 110, y: 315 },
                { x: 105, y: 302 },
                { x: 100, y: 290 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once and restart near the stem's top",
              path: [
                { x: 110, y: 315 },
                { x: 150, y: 335 },
                { x: 210, y: 340 },
                { x: 280, y: 325 },
                { x: 350, y: 305 },
                { x: 420, y: 285 },
                { x: 490, y: 270 },
                { x: 540, y: 270 },
              ],
            },
            {
              label: "continue down and around the bowl",
              path: [
                { x: 540, y: 270 },
                { x: 490, y: 270 },
                { x: 420, y: 285 },
                { x: 350, y: 305 },
                { x: 280, y: 325 },
                { x: 210, y: 340 },
                { x: 150, y: 335 },
                { x: 110, y: 315 },
                { x: 100, y: 290 },
                { x: 130, y: 305 },
                { x: 170, y: 310 },
                { x: 220, y: 305 },
                { x: 270, y: 285 },
                { x: 320, y: 265 },
                { x: 300, y: 245 },
                { x: 260, y: 220 },
                { x: 216, y: 190 },
                { x: 180, y: 130 },
                { x: 145, y: 65 },
                { x: 118, y: -42 },
                { x: 130, y: -110 },
                { x: 180, y: -175 },
                { x: 225, y: -200 },
                { x: 300, y: -245 },
                { x: 400, y: -245 },
                { x: 500, y: -230 },
                { x: 575, y: -210 },
                { x: 608, y: -195 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ح"),
    },
  ],
  // Persian Online and Zer o Zabar independently demonstrate dotless ح as a
  // single body-first run. Keep those script-scoped sources separate from the
  // Arabic course's two-run stem-first order above.
  [
    "perso-arabic:ح",
    {
      script: "perso-arabic",
      glyph: "ح",
      strokes: independentHahStrokes(
        "draw the short upper head from left to right",
      ),
      source: persianAlphabetSource("ح"),
    },
  ],
  [
    "urdu-nastaliq:ح",
    {
      script: "urdu-nastaliq",
      glyph: "ح",
      strokes: independentHahStrokes(
        "sweep left through the pointed hooked head",
      ),
      source: urduAlphabetSource("ح"),
    },
  ],
  [
    "arabic:خ",
    {
      script: "arabic",
      glyph: "خ",
      strokes: [
        {
          segments: [
            {
              label: "draw the short upper head from left to right",
              path: [
                { x: 110, y: 315 },
                { x: 150, y: 335 },
                { x: 210, y: 340 },
                { x: 280, y: 325 },
                { x: 350, y: 305 },
                { x: 420, y: 285 },
                { x: 490, y: 270 },
                { x: 540, y: 270 },
              ],
            },
            {
              label: "continue down and around the bowl",
              path: [
                { x: 540, y: 270 },
                { x: 490, y: 270 },
                { x: 420, y: 285 },
                { x: 350, y: 305 },
                { x: 280, y: 325 },
                { x: 210, y: 340 },
                { x: 150, y: 335 },
                { x: 110, y: 315 },
                { x: 100, y: 290 },
                { x: 130, y: 305 },
                { x: 170, y: 310 },
                { x: 220, y: 305 },
                { x: 270, y: 285 },
                { x: 320, y: 265 },
                { x: 300, y: 245 },
                { x: 260, y: 220 },
                { x: 216, y: 190 },
                { x: 180, y: 130 },
                { x: 145, y: 65 },
                { x: 118, y: -42 },
                { x: 130, y: -110 },
                { x: 180, y: -175 },
                { x: 225, y: -200 },
                { x: 300, y: -245 },
                { x: 400, y: -245 },
                { x: 500, y: -230 },
                { x: 575, y: -210 },
                { x: 608, y: -195 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the dot above",
              path: [
                { x: 340, y: 460 },
                { x: 285, y: 510 },
                { x: 338, y: 565 },
                { x: 390, y: 515 },
                { x: 340, y: 460 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("خ"),
    },
  ],
  [
    "perso-arabic:خ",
    {
      script: "perso-arabic",
      glyph: "خ",
      strokes: independentKhehStrokes(),
      source: persianAlphabetSource("خ"),
    },
  ],
  [
    "urdu-nastaliq:خ",
    {
      script: "urdu-nastaliq",
      glyph: "خ",
      strokes: independentKhehStrokes(),
      source: urduAlphabetSource("خ"),
    },
  ],
  [
    "arabic:د",
    {
      script: "arabic",
      glyph: "د",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper tip and descend diagonally down and right through the curved shoulder",
              path: [
                { x: 270, y: 350 },
                { x: 260, y: 325 },
                { x: 260, y: 300 },
                { x: 270, y: 275 },
                { x: 285, y: 245 },
                { x: 300, y: 215 },
                { x: 318, y: 185 },
                { x: 333, y: 155 },
                { x: 343, y: 130 },
                { x: 345, y: 110 },
                { x: 342, y: 100 },
              ],
            },
            {
              label: "turn left along the baseline without lifting",
              path: [
                { x: 342, y: 100 },
                { x: 320, y: 90 },
                { x: 290, y: 75 },
                { x: 250, y: 60 },
                { x: 210, y: 50 },
                { x: 170, y: 40 },
                { x: 130, y: 40 },
                { x: 90, y: 50 },
                { x: 60, y: 65 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("د"),
    },
  ],
  [
    "arabic:ذ",
    {
      script: "arabic",
      glyph: "ذ",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper tip and descend diagonally down and right through the curved shoulder",
              path: [
                { x: 270, y: 350 },
                { x: 260, y: 325 },
                { x: 260, y: 300 },
                { x: 270, y: 275 },
                { x: 285, y: 245 },
                { x: 300, y: 215 },
                { x: 318, y: 185 },
                { x: 333, y: 155 },
                { x: 343, y: 130 },
                { x: 345, y: 110 },
                { x: 342, y: 100 },
              ],
            },
            {
              label: "turn left along the baseline without lifting",
              path: [
                { x: 342, y: 100 },
                { x: 320, y: 90 },
                { x: 290, y: 75 },
                { x: 250, y: 60 },
                { x: 210, y: 50 },
                { x: 170, y: 40 },
                { x: 130, y: 40 },
                { x: 90, y: 50 },
                { x: 60, y: 65 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the dot above",
              path: [
                { x: 218, y: 490 },
                { x: 180, y: 525 },
                { x: 216, y: 575 },
                { x: 265, y: 532 },
                { x: 218, y: 490 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ذ"),
    },
  ],
  [
    "arabic:ر",
    {
      script: "arabic",
      glyph: "ر",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper tip and descend through the short stroke",
              path: [
                { x: 250, y: 320 },
                { x: 248, y: 280 },
                { x: 255, y: 235 },
                { x: 270, y: 190 },
                { x: 287, y: 145 },
                { x: 300, y: 95 },
                { x: 304, y: 48 },
              ],
            },
            {
              label: "sweep left through the lower curve without lifting",
              path: [
                { x: 304, y: 48 },
                { x: 298, y: 8 },
                { x: 284, y: -30 },
                { x: 260, y: -68 },
                { x: 226, y: -103 },
                { x: 185, y: -130 },
                { x: 140, y: -146 },
                { x: 95, y: -151 },
                { x: 52, y: -147 },
                { x: 10, y: -136 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ر"),
    },
  ],
  [
    "arabic:ز",
    {
      script: "arabic",
      glyph: "ز",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper tip and descend through the short stroke",
              path: [
                { x: 250, y: 320 },
                { x: 248, y: 280 },
                { x: 255, y: 235 },
                { x: 270, y: 190 },
                { x: 287, y: 145 },
                { x: 300, y: 95 },
                { x: 304, y: 48 },
              ],
            },
            {
              label: "sweep left through the lower curve without lifting",
              path: [
                { x: 304, y: 48 },
                { x: 298, y: 8 },
                { x: 284, y: -30 },
                { x: 260, y: -68 },
                { x: 226, y: -103 },
                { x: 185, y: -130 },
                { x: 140, y: -146 },
                { x: 95, y: -151 },
                { x: 52, y: -147 },
                { x: 10, y: -136 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the dot above",
              path: [
                { x: 200, y: 405 },
                { x: 140, y: 465 },
                { x: 200, y: 525 },
                { x: 260, y: 465 },
                { x: 200, y: 405 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ز"),
    },
  ],
  [
    "arabic:س",
    {
      script: "arabic",
      glyph: "س",
      strokes: [
        {
          segments: [
            {
              label: "form the three close teeth from right to left",
              path: [
                { x: 923, y: 310 },
                { x: 935, y: 120 },
                { x: 925, y: 70 },
                { x: 870, y: 45 },
                { x: 770, y: 75 },
                { x: 748, y: 110 },
                { x: 748, y: 230 },
                { x: 690, y: 65 },
                { x: 640, y: 45 },
                { x: 540, y: 55 },
                { x: 478, y: 190 },
                { x: 515, y: 20 },
              ],
            },
            {
              label: "flow directly into the final bowl without lifting",
              path: [
                { x: 515, y: 20 },
                { x: 515, y: -25 },
                { x: 470, y: -125 },
                { x: 370, y: -205 },
                { x: 250, y: -230 },
                { x: 145, y: -180 },
                { x: 92, y: -95 },
                { x: 110, y: 35 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("س"),
    },
  ],
  [
    "arabic:ش",
    {
      script: "arabic",
      glyph: "ش",
      strokes: [
        {
          segments: [
            {
              label: "shape the three close teeth from right to left",
              path: [
                { x: 923, y: 310 },
                { x: 935, y: 120 },
                { x: 925, y: 70 },
                { x: 870, y: 45 },
                { x: 770, y: 75 },
                { x: 748, y: 110 },
                { x: 748, y: 230 },
                { x: 690, y: 65 },
                { x: 640, y: 45 },
                { x: 540, y: 55 },
                { x: 478, y: 190 },
                { x: 515, y: 20 },
              ],
            },
            {
              label: "flow directly into the final bowl without lifting",
              path: [
                { x: 515, y: 20 },
                { x: 515, y: -25 },
                { x: 470, y: -125 },
                { x: 370, y: -205 },
                { x: 250, y: -230 },
                { x: 145, y: -180 },
                { x: 92, y: -95 },
                { x: 110, y: 35 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the lower-left dot",
              path: [
                { x: 610, y: 360 },
                { x: 648, y: 410 },
                { x: 686, y: 365 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, then place the lower-right dot",
              path: [
                { x: 753, y: 370 },
                { x: 792, y: 423 },
                { x: 830, y: 376 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift a third time, then place the centered upper dot",
              path: [
                { x: 684, y: 446 },
                { x: 720, y: 494 },
                { x: 757, y: 446 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ش"),
    },
  ],
  [
    "arabic:ص",
    {
      script: "arabic",
      glyph: "ص",
      strokes: [
        {
          segments: [
            {
              label: "close the oval clockwise from its lower-left junction",
              path: [
                { x: 535, y: 30 },
                { x: 560, y: 90 },
                { x: 620, y: 160 },
                { x: 700, y: 230 },
                { x: 790, y: 305 },
                { x: 870, y: 320 },
                { x: 950, y: 285 },
                { x: 1010, y: 230 },
                { x: 1015, y: 175 },
                { x: 970, y: 115 },
                { x: 900, y: 70 },
                { x: 810, y: 45 },
                { x: 720, y: 38 },
                { x: 630, y: 42 },
                { x: 535, y: 30 },
              ],
            },
            {
              label:
                "turn left and rise into the short shoulder without lifting",
              path: [
                { x: 535, y: 30 },
                { x: 530, y: 65 },
                { x: 520, y: 105 },
                { x: 510, y: 145 },
                { x: 495, y: 190 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, restart at the baseline junction, and sweep through the trailing bowl",
              path: [
                { x: 500, y: -54 },
                { x: 475, y: -115 },
                { x: 425, y: -175 },
                { x: 360, y: -215 },
                { x: 280, y: -232 },
                { x: 205, y: -225 },
                { x: 145, y: -185 },
                { x: 105, y: -125 },
                { x: 92, y: -65 },
                { x: 100, y: 20 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ص"),
    },
  ],
  [
    "arabic:ض",
    {
      script: "arabic",
      glyph: "ض",
      strokes: [
        {
          segments: [
            {
              label: "close the oval clockwise from its lower-left junction",
              path: [
                { x: 535, y: 30 },
                { x: 560, y: 90 },
                { x: 620, y: 160 },
                { x: 700, y: 230 },
                { x: 790, y: 305 },
                { x: 870, y: 320 },
                { x: 950, y: 285 },
                { x: 1010, y: 230 },
                { x: 1015, y: 175 },
                { x: 970, y: 115 },
                { x: 900, y: 70 },
                { x: 810, y: 45 },
                { x: 720, y: 38 },
                { x: 630, y: 42 },
                { x: 535, y: 30 },
              ],
            },
            {
              label:
                "turn left and rise into the short shoulder without lifting",
              path: [
                { x: 535, y: 30 },
                { x: 530, y: 65 },
                { x: 520, y: 105 },
                { x: 510, y: 145 },
                { x: 495, y: 190 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, restart at the baseline junction, and sweep through the trailing bowl",
              path: [
                { x: 500, y: -54 },
                { x: 475, y: -115 },
                { x: 425, y: -175 },
                { x: 360, y: -215 },
                { x: 280, y: -232 },
                { x: 205, y: -225 },
                { x: 145, y: -185 },
                { x: 105, y: -125 },
                { x: 92, y: -65 },
                { x: 100, y: 20 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, then place the upper dot last",
              path: [
                { x: 725, y: 470 },
                { x: 675, y: 515 },
                { x: 725, y: 568 },
                { x: 770, y: 520 },
                { x: 725, y: 470 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ض"),
    },
  ],
  [
    "arabic:ط",
    {
      script: "arabic",
      glyph: "ط",
      strokes: [
        {
          segments: [
            {
              label: "loop counterclockwise around the closed oval",
              path: [
                { x: 675, y: 280 },
                { x: 600, y: 315 },
                { x: 500, y: 310 },
                { x: 450, y: 290 },
                { x: 400, y: 265 },
                { x: 350, y: 230 },
                { x: 310, y: 195 },
                { x: 280, y: 190 },
                { x: 275, y: 120 },
                { x: 260, y: 95 },
                { x: 275, y: 65 },
                { x: 330, y: 45 },
                { x: 430, y: 35 },
                { x: 540, y: 50 },
                { x: 635, y: 90 },
                { x: 690, y: 150 },
                { x: 690, y: 220 },
                { x: 675, y: 280 },
              ],
            },
            {
              label: "finish left along the baseline without lifting",
              path: [
                { x: 675, y: 280 },
                { x: 600, y: 315 },
                { x: 500, y: 310 },
                { x: 450, y: 290 },
                { x: 400, y: 265 },
                { x: 350, y: 230 },
                { x: 310, y: 195 },
                { x: 280, y: 190 },
                { x: 270, y: 130 },
                { x: 220, y: 90 },
                { x: 150, y: 75 },
                { x: 80, y: 85 },
                { x: 45, y: 90 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then draw the tall upright top-to-bottom",
              path: [
                { x: 245, y: 650 },
                { x: 240, y: 590 },
                { x: 248, y: 520 },
                { x: 255, y: 450 },
                { x: 263, y: 380 },
                { x: 272, y: 315 },
                { x: 282, y: 255 },
                { x: 295, y: 205 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ط"),
    },
  ],
  [
    "arabic:ظ",
    {
      script: "arabic",
      glyph: "ظ",
      strokes: [
        {
          segments: [
            {
              label: "loop counterclockwise around the closed oval",
              path: [
                { x: 675, y: 280 },
                { x: 600, y: 315 },
                { x: 500, y: 310 },
                { x: 450, y: 290 },
                { x: 400, y: 265 },
                { x: 350, y: 230 },
                { x: 310, y: 195 },
                { x: 280, y: 190 },
                { x: 275, y: 120 },
                { x: 260, y: 95 },
                { x: 275, y: 65 },
                { x: 330, y: 45 },
                { x: 430, y: 35 },
                { x: 540, y: 50 },
                { x: 635, y: 90 },
                { x: 690, y: 150 },
                { x: 690, y: 220 },
                { x: 675, y: 280 },
              ],
            },
            {
              label: "finish left along the baseline without lifting",
              path: [
                { x: 675, y: 280 },
                { x: 600, y: 315 },
                { x: 500, y: 310 },
                { x: 450, y: 290 },
                { x: 400, y: 265 },
                { x: 350, y: 230 },
                { x: 310, y: 195 },
                { x: 280, y: 190 },
                { x: 270, y: 130 },
                { x: 220, y: 90 },
                { x: 150, y: 75 },
                { x: 80, y: 85 },
                { x: 45, y: 90 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the upper dot",
              path: [
                { x: 513, y: 460 },
                { x: 460, y: 517 },
                { x: 513, y: 574 },
                { x: 568, y: 517 },
                { x: 513, y: 460 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, then draw the tall upright top-to-bottom",
              path: [
                { x: 245, y: 650 },
                { x: 240, y: 590 },
                { x: 248, y: 520 },
                { x: 255, y: 450 },
                { x: 263, y: 380 },
                { x: 272, y: 315 },
                { x: 282, y: 255 },
                { x: 295, y: 205 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ظ"),
    },
  ],
  [
    "arabic:ع",
    {
      script: "arabic",
      glyph: "ع",
      strokes: [
        {
          segments: [
            {
              label:
                "sweep left from the upper-right tip and shape the open head",
              path: [
                { x: 355, y: 400 },
                { x: 315, y: 420 },
                { x: 255, y: 430 },
                { x: 195, y: 415 },
                { x: 145, y: 375 },
                { x: 110, y: 320 },
                { x: 105, y: 270 },
                { x: 135, y: 235 },
                { x: 145, y: 205 },
                { x: 185, y: 175 },
                { x: 250, y: 165 },
                { x: 325, y: 175 },
                { x: 395, y: 205 },
                { x: 450, y: 235 },
                { x: 410, y: 205 },
                { x: 350, y: 175 },
                { x: 285, y: 145 },
                { x: 230, y: 110 },
                { x: 190, y: 75 },
                { x: 175, y: 50 },
              ],
            },
            {
              label: "continue down and around the lower bowl without lifting",
              path: [
                { x: 175, y: 50 },
                { x: 150, y: -5 },
                { x: 135, y: -70 },
                { x: 145, y: -135 },
                { x: 185, y: -195 },
                { x: 245, y: -235 },
                { x: 320, y: -250 },
                { x: 400, y: -245 },
                { x: 480, y: -230 },
                { x: 555, y: -205 },
                { x: 610, y: -180 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ع"),
    },
  ],
  [
    "arabic:غ",
    {
      script: "arabic",
      glyph: "غ",
      strokes: [
        {
          segments: [
            {
              label:
                "sweep left from the upper-right tip and shape the open head",
              path: [
                { x: 355, y: 400 },
                { x: 315, y: 420 },
                { x: 255, y: 430 },
                { x: 195, y: 415 },
                { x: 145, y: 375 },
                { x: 110, y: 320 },
                { x: 105, y: 270 },
                { x: 135, y: 235 },
                { x: 145, y: 205 },
                { x: 185, y: 175 },
                { x: 250, y: 165 },
                { x: 325, y: 175 },
                { x: 395, y: 205 },
                { x: 450, y: 235 },
                { x: 410, y: 205 },
                { x: 350, y: 175 },
                { x: 285, y: 145 },
                { x: 230, y: 110 },
                { x: 190, y: 75 },
                { x: 175, y: 50 },
              ],
            },
            {
              label: "continue down and around the lower bowl without lifting",
              path: [
                { x: 175, y: 50 },
                { x: 150, y: -5 },
                { x: 135, y: -70 },
                { x: 145, y: -135 },
                { x: 185, y: -195 },
                { x: 245, y: -235 },
                { x: 320, y: -250 },
                { x: 400, y: -245 },
                { x: 480, y: -230 },
                { x: 555, y: -205 },
                { x: 610, y: -180 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the upper dot last",
              path: [
                { x: 223, y: 547 },
                { x: 170, y: 604 },
                { x: 223, y: 661 },
                { x: 278, y: 604 },
                { x: 223, y: 547 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("غ"),
    },
  ],
  [
    "arabic:ف",
    {
      script: "arabic",
      glyph: "ف",
      strokes: [
        {
          segments: [
            {
              label: "loop counterclockwise around the small closed head",
              path: [
                { x: 735, y: 250 },
                { x: 750, y: 330 },
                { x: 710, y: 400 },
                { x: 640, y: 435 },
                { x: 570, y: 410 },
                { x: 520, y: 350 },
                { x: 520, y: 285 },
                { x: 560, y: 230 },
                { x: 640, y: 215 },
                { x: 715, y: 240 },
                { x: 735, y: 250 },
              ],
            },
            {
              label: "continue left through the broad bowl without lifting",
              path: [
                { x: 735, y: 250 },
                { x: 775, y: 230 },
                { x: 785, y: 180 },
                { x: 760, y: 110 },
                { x: 700, y: 80 },
                { x: 600, y: 60 },
                { x: 480, y: 55 },
                { x: 350, y: 40 },
                { x: 230, y: 55 },
                { x: 140, y: 105 },
                { x: 95, y: 170 },
                { x: 90, y: 240 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the upper dot last",
              path: [
                { x: 615, y: 550 },
                { x: 560, y: 607 },
                { x: 615, y: 664 },
                { x: 670, y: 607 },
                { x: 615, y: 550 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ف"),
    },
  ],
  [
    "perso-arabic:ف",
    {
      script: "perso-arabic",
      glyph: "ف",
      strokes: independentFehStrokes(
        "loop clockwise around the small closed head",
        "continue left through the broad bowl without lifting",
      ),
      source: persianAlphabetSource("ف"),
    },
  ],
  [
    "urdu-nastaliq:ف",
    {
      script: "urdu-nastaliq",
      glyph: "ف",
      strokes: independentFehStrokes(
        "loop clockwise around the rounded head above the main line",
        "continue left through the shallow curved tail without lifting",
      ),
      source: urduAlphabetSource("ف"),
    },
  ],
  [
    "arabic:ق",
    {
      script: "arabic",
      glyph: "ق",
      strokes: [
        {
          segments: [
            {
              label: "loop counterclockwise around the small closed head",
              path: [
                { x: 545, y: 160 },
                { x: 520, y: 235 },
                { x: 470, y: 285 },
                { x: 410, y: 300 },
                { x: 350, y: 270 },
                { x: 305, y: 220 },
                { x: 290, y: 165 },
                { x: 305, y: 105 },
                { x: 345, y: 55 },
                { x: 405, y: 40 },
                { x: 475, y: 55 },
                { x: 525, y: 100 },
                { x: 545, y: 160 },
              ],
            },
            {
              label:
                "continue down and left through the deep bowl without lifting",
              path: [
                { x: 545, y: 160 },
                { x: 575, y: 105 },
                { x: 570, y: 35 },
                { x: 545, y: -55 },
                { x: 500, y: -130 },
                { x: 430, y: -185 },
                { x: 345, y: -220 },
                { x: 260, y: -225 },
                { x: 180, y: -205 },
                { x: 120, y: -155 },
                { x: 90, y: -95 },
                { x: 90, y: -35 },
                { x: 90, y: 35 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift once, then place the upper-right dot",
              path: [
                { x: 475, y: 405 },
                { x: 425, y: 457 },
                { x: 475, y: 510 },
                { x: 525, y: 457 },
                { x: 475, y: 405 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, then place the upper-left dot",
              path: [
                { x: 325, y: 390 },
                { x: 275, y: 442 },
                { x: 325, y: 495 },
                { x: 375, y: 442 },
                { x: 325, y: 390 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ق"),
    },
  ],
  [
    "perso-arabic:ق",
    {
      script: "perso-arabic",
      glyph: "ق",
      strokes: independentQafStrokes(
        "loop counterclockwise around the small closed head",
        "continue down and left through the deep bowl without lifting",
        "lift once, then place the upper-right dot",
        "lift again, then place the upper-left dot",
      ),
      source: persianAlphabetSource("ق"),
    },
  ],
  [
    "urdu-nastaliq:ق",
    {
      script: "urdu-nastaliq",
      glyph: "ق",
      strokes: independentQafStrokes(
        "loop clockwise around the rounded head above the main line",
        "continue down and left through the deep bowl without lifting",
        "after one lift, place the upper-right dot",
        "after another lift, place the upper-left dot",
      ),
      source: urduAlphabetSource("ق"),
    },
  ],
  [
    "arabic:ك",
    {
      script: "arabic",
      glyph: "ك",
      strokes: [
        {
          segments: [
            {
              label: "descend the main upright",
              path: [
                { x: 430, y: 630 },
                { x: 435, y: 550 },
                { x: 440, y: 450 },
                { x: 450, y: 350 },
                { x: 465, y: 250 },
                { x: 475, y: 150 },
                { x: 470, y: 80 },
              ],
            },
            {
              label: "turn left along the baseline without lifting",
              path: [
                { x: 470, y: 80 },
                { x: 410, y: 52 },
                { x: 320, y: 40 },
                { x: 220, y: 38 },
                { x: 120, y: 42 },
                { x: 45, y: 58 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then draw the inner arm from upper right down-left",
              path: [
                { x: 255, y: 385 },
                { x: 235, y: 375 },
                { x: 215, y: 360 },
                { x: 195, y: 340 },
                { x: 185, y: 320 },
                { x: 185, y: 305 },
                { x: 215, y: 295 },
                { x: 245, y: 292 },
                { x: 275, y: 285 },
                { x: 282, y: 273 },
                { x: 270, y: 258 },
                { x: 250, y: 240 },
                { x: 225, y: 222 },
                { x: 180, y: 207 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ك"),
    },
  ],
  [
    "arabic:ل",
    {
      script: "arabic",
      glyph: "ل",
      strokes: [
        {
          segments: [
            {
              label: "descend the tall upright",
              path: [
                { x: 458, y: 640 },
                { x: 445, y: 500 },
                { x: 440, y: 420 },
                { x: 450, y: 240 },
                { x: 475, y: 80 },
                { x: 510, y: -20 },
              ],
            },
            {
              label: "continue left through the base bowl without lifting",
              path: [
                { x: 510, y: -20 },
                { x: 465, y: -120 },
                { x: 350, y: -205 },
                { x: 205, y: -215 },
                { x: 105, y: -135 },
                { x: 90, y: -75 },
                { x: 100, y: 25 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ل"),
    },
  ],
  [
    "arabic:م",
    {
      script: "arabic",
      glyph: "م",
      strokes: [
        {
          segments: [
            {
              label: "form the small closed head in a tight circular movement",
              path: [
                { x: 120, y: 210 },
                { x: 150, y: 250 },
                { x: 200, y: 300 },
                { x: 245, y: 315 },
                { x: 285, y: 300 },
                { x: 330, y: 260 },
                { x: 365, y: 215 },
                { x: 400, y: 175 },
                { x: 430, y: 150 },
              ],
            },
            {
              label:
                "continue down and left through the below-baseline tail without lifting",
              path: [
                { x: 430, y: 150 },
                { x: 390, y: 110 },
                { x: 330, y: 95 },
                { x: 260, y: 80 },
                { x: 180, y: 65 },
                { x: 100, y: 35 },
                { x: 90, y: -20 },
                { x: 100, y: -90 },
                { x: 110, y: -160 },
                { x: 120, y: -240 },
                { x: 105, y: -285 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("م"),
    },
  ],
  [
    "arabic:ن",
    {
      script: "arabic",
      glyph: "ن",
      strokes: [
        {
          segments: [
            {
              label: "sweep down and around the deep bowl from right to left",
              path: [
                { x: 495, y: 210 },
                { x: 475, y: 160 },
                { x: 480, y: 100 },
                { x: 500, y: 40 },
                { x: 510, y: -20 },
                { x: 485, y: -80 },
                { x: 430, y: -140 },
                { x: 360, y: -190 },
                { x: 280, y: -220 },
                { x: 210, y: -215 },
                { x: 150, y: -170 },
                { x: 105, y: -110 },
                { x: 90, y: -60 },
                { x: 95, y: 0 },
                { x: 105, y: 45 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the dot above the bowl's midpoint",
              path: [
                { x: 235, y: 305 },
                { x: 275, y: 345 },
                { x: 315, y: 305 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ن"),
    },
  ],
  [
    "arabic:ه",
    {
      script: "arabic",
      glyph: "ه",
      strokes: [
        {
          segments: [
            {
              label: "curve down-left and close the lower counter",
              path: [
                { x: 315, y: 400 },
                { x: 285, y: 375 },
                { x: 255, y: 350 },
                { x: 230, y: 325 },
                { x: 205, y: 300 },
                { x: 190, y: 260 },
                { x: 190, y: 210 },
                { x: 205, y: 165 },
                { x: 235, y: 125 },
                { x: 275, y: 105 },
                { x: 320, y: 110 },
                { x: 355, y: 135 },
                { x: 380, y: 175 },
                { x: 390, y: 225 },
                { x: 380, y: 275 },
                { x: 355, y: 320 },
                { x: 315, y: 355 },
              ],
            },
            {
              label:
                "thread through the centre and close the upper-right counter without lifting",
              path: [
                { x: 315, y: 355 },
                { x: 360, y: 355 },
                { x: 410, y: 340 },
                { x: 455, y: 315 },
                { x: 500, y: 275 },
                { x: 535, y: 225 },
                { x: 555, y: 170 },
                { x: 555, y: 115 },
                { x: 535, y: 70 },
                { x: 535, y: 50 },
                { x: 500, y: 40 },
                { x: 455, y: 30 },
                { x: 415, y: 45 },
                { x: 385, y: 75 },
                { x: 365, y: 100 },
              ],
            },
            {
              label: "sweep left along the baseline without lifting",
              path: [
                { x: 365, y: 100 },
                { x: 345, y: 75 },
                { x: 310, y: 65 },
                { x: 270, y: 65 },
                { x: 225, y: 70 },
                { x: 175, y: 65 },
                { x: 120, y: 65 },
                { x: 70, y: 65 },
                { x: 25, y: 65 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ه"),
    },
  ],
  // Taa marbuta is word-final only. Its isolated Naskh body closes clockwise
  // before the two upper dots; the cited lesson records both dot-order customs.
  [
    "arabic:ة",
    {
      script: "arabic",
      glyph: "ة",
      strokes: [
        {
          segments: [
            {
              label:
                "circle clockwise through the compact body and close it on the baseline",
              path: [
                { x: 181, y: 351 },
                { x: 225, y: 325 },
                { x: 270, y: 285 },
                { x: 300, y: 240 },
                { x: 325, y: 185 },
                { x: 310, y: 130 },
                { x: 275, y: 85 },
                { x: 225, y: 40 },
                { x: 190, y: 40 },
                { x: 145, y: 65 },
                { x: 91, y: 140 },
                { x: 92, y: 185 },
                { x: 136, y: 264 },
                { x: 175, y: 315 },
                { x: 181, y: 351 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the upper-left dot",
              path: [
                { x: 75, y: 500 },
                { x: 109, y: 545 },
                { x: 145, y: 500 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, then place the upper-right dot",
              path: [
                { x: 220, y: 515 },
                { x: 260, y: 560 },
                { x: 300, y: 515 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ة"),
    },
  ],
  [
    "arabic:و",
    {
      script: "arabic",
      glyph: "و",
      strokes: [
        {
          segments: [
            {
              label:
                "sweep left from the lower-right junction and close the small head loop",
              path: [
                { x: 390, y: 60 },
                { x: 340, y: 45 },
                { x: 285, y: 40 },
                { x: 225, y: 45 },
                { x: 175, y: 80 },
                { x: 145, y: 125 },
                { x: 145, y: 165 },
                { x: 170, y: 215 },
                { x: 210, y: 260 },
                { x: 250, y: 285 },
                { x: 300, y: 285 },
                { x: 345, y: 245 },
                { x: 375, y: 185 },
                { x: 390, y: 115 },
                { x: 390, y: 60 },
              ],
            },
            {
              label: "continue down and left through the tail without lifting",
              path: [
                { x: 390, y: 60 },
                { x: 370, y: -5 },
                { x: 340, y: -70 },
                { x: 300, y: -120 },
                { x: 250, y: -160 },
                { x: 195, y: -170 },
                { x: 135, y: -160 },
                { x: 80, y: -140 },
                { x: 45, y: -120 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("و"),
    },
  ],
  // Arabic ي (U+064A) shares the isolated bowl skeleton with Urdu ی (U+06CC),
  // but keeps its own source and adds the two lower dots observed in yaa.mov.
  [
    "arabic:ي",
    {
      script: "arabic",
      glyph: "ي",
      strokes: [
        {
          segments: [
            {
              label: "descend from the upper right into the independent bowl",
              path: [
                { x: 548, y: 285 },
                { x: 510, y: 288 },
                { x: 472, y: 270 },
                { x: 430, y: 238 },
                { x: 395, y: 205 },
                { x: 365, y: 168 },
                { x: 345, y: 125 },
                { x: 330, y: 82 },
                { x: 340, y: 45 },
                { x: 375, y: 25 },
                { x: 420, y: 8 },
                { x: 470, y: -2 },
                { x: 520, y: -24 },
                { x: 555, y: -55 },
              ],
            },
            {
              label: "sweep left through the bowl without lifting",
              path: [
                { x: 555, y: -55 },
                { x: 535, y: -98 },
                { x: 495, y: -145 },
                { x: 445, y: -188 },
                { x: 390, y: -218 },
                { x: 325, y: -238 },
                { x: 255, y: -238 },
                { x: 190, y: -218 },
                { x: 140, y: -180 },
                { x: 105, y: -130 },
                { x: 90, y: -78 },
                { x: 94, y: -25 },
                { x: 105, y: 28 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the lower-left dot",
              path: [
                { x: 150, y: -373 },
                { x: 198, y: -323 },
                { x: 245, y: -370 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, then place the lower-right dot",
              path: [
                { x: 300, y: -360 },
                { x: 352, y: -310 },
                { x: 400, y: -356 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ي"),
    },
  ],
  // Alif maqsura keeps the old dotless final-Yaa body as one continuous S,
  // but it is a distinct word-final long-aa character with its own source.
  [
    "arabic:ى",
    {
      script: "arabic",
      glyph: "ى",
      strokes: [
        {
          segments: [
            {
              label:
                "curve from above the baseline through the upper half of the S",
              path: [
                { x: 548, y: 285 },
                { x: 510, y: 288 },
                { x: 472, y: 270 },
                { x: 430, y: 238 },
                { x: 395, y: 205 },
                { x: 365, y: 168 },
                { x: 345, y: 125 },
                { x: 330, y: 82 },
                { x: 340, y: 45 },
                { x: 375, y: 25 },
                { x: 420, y: 8 },
                { x: 470, y: -2 },
                { x: 520, y: -24 },
                { x: 555, y: -55 },
              ],
            },
            {
              label:
                "continue through the wide flat lower curve and finish near the baseline",
              path: [
                { x: 555, y: -55 },
                { x: 535, y: -98 },
                { x: 495, y: -145 },
                { x: 445, y: -188 },
                { x: 390, y: -218 },
                { x: 325, y: -238 },
                { x: 255, y: -238 },
                { x: 190, y: -218 },
                { x: 140, y: -180 },
                { x: 105, y: -130 },
                { x: 90, y: -78 },
                { x: 94, y: -25 },
                { x: 105, y: 28 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ى"),
    },
  ],
  // Lam plus Alif is obligatorily shaped as a crossed ligature. The table key
  // remains the editable two-character sequence; U+FEFB supplies only the
  // joined Noto Naskh outline against which these sourced movements are fit.
  [
    "arabic:لا",
    {
      script: "arabic",
      sequence: "لا",
      glyph: "ﻻ",
      strokes: [
        {
          segments: [
            {
              label:
                "descend from the upper right and curve left near the baseline",
              path: [
                { x: 398, y: 660 },
                { x: 390, y: 610 },
                { x: 386, y: 555 },
                { x: 401, y: 485 },
                { x: 404, y: 410 },
                { x: 398, y: 335 },
                { x: 374, y: 255 },
                { x: 340, y: 185 },
                { x: 292, y: 128 },
                { x: 230, y: 84 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, cross down from the upper left, and meet the first endpoint",
              path: [
                { x: 58, y: 500 },
                { x: 82, y: 470 },
                { x: 120, y: 435 },
                { x: 175, y: 395 },
                { x: 235, y: 345 },
                { x: 300, y: 285 },
                { x: 350, y: 230 },
                { x: 392, y: 165 },
                { x: 430, y: 95 },
                { x: 437, y: 58 },
                { x: 390, y: 42 },
                { x: 325, y: 34 },
                { x: 260, y: 27 },
                { x: 200, y: 23 },
                { x: 150, y: 24 },
              ],
            },
          ],
        },
      ],
      source: arabicLigatureSource("لا"),
    },
  ],
  // The source explicitly demonstrates the one-stroke Hamza variant: its
  // c-shaped upper head flows directly into the lower diagonal.
  [
    "arabic:ء",
    {
      script: "arabic",
      glyph: "ء",
      strokes: [
        {
          segments: [
            {
              label: "sweep counterclockwise through the c-shaped upper head",
              path: [
                { x: 292, y: 194 },
                { x: 300, y: 219 },
                { x: 286, y: 239 },
                { x: 261, y: 250 },
                { x: 232, y: 252 },
                { x: 203, y: 242 },
                { x: 178, y: 225 },
                { x: 156, y: 202 },
                { x: 139, y: 177 },
                { x: 126, y: 151 },
                { x: 118, y: 126 },
              ],
            },
            {
              label:
                "continue through the lower diagonal toward the right without lifting",
              path: [
                { x: 118, y: 126 },
                { x: 124, y: 105 },
                { x: 134, y: 82 },
                { x: 144, y: 60 },
                { x: 151, y: 42 },
                { x: 137, y: 31 },
                { x: 111, y: 19 },
                { x: 86, y: 7 },
                { x: 71, y: -5 },
                { x: 104, y: 5 },
                { x: 140, y: 18 },
                { x: 179, y: 29 },
                { x: 221, y: 36 },
                { x: 265, y: 42 },
                { x: 308, y: 48 },
                { x: 348, y: 55 },
                { x: 383, y: 62 },
                { x: 402, y: 66 },
              ],
            },
          ],
        },
      ],
      source: arabicAlphabetSource("ء"),
    },
  ],
  [
    "urdu-nastaliq:ج",
    {
      script: "urdu-nastaliq",
      glyph: "ج",
      strokes: [
        {
          segments: [
            {
              label: "place the dot below",
              path: [
                { x: 415, y: -1 },
                { x: 374, y: 38 },
                { x: 330, y: -9 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then sweep left through the pointed hooked head",
              path: [
                { x: 540, y: 270 },
                { x: 490, y: 270 },
                { x: 420, y: 285 },
                { x: 350, y: 305 },
                { x: 280, y: 325 },
                { x: 210, y: 340 },
                { x: 150, y: 335 },
                { x: 110, y: 315 },
                { x: 100, y: 290 },
                { x: 130, y: 305 },
                { x: 170, y: 310 },
                { x: 220, y: 305 },
                { x: 270, y: 285 },
                { x: 320, y: 265 },
                { x: 300, y: 245 },
                { x: 260, y: 220 },
                { x: 216, y: 190 },
              ],
            },
            {
              label: "continue down and around the bowl",
              path: [
                { x: 216, y: 190 },
                { x: 180, y: 130 },
                { x: 145, y: 65 },
                { x: 118, y: -42 },
                { x: 130, y: -110 },
                { x: 180, y: -175 },
                { x: 225, y: -200 },
                { x: 300, y: -245 },
                { x: 400, y: -245 },
                { x: 500, y: -230 },
                { x: 575, y: -210 },
                { x: 608, y: -195 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ج"),
    },
  ],
  // Che shares jīm's Noto fallback body, but Zer o Zabar independently shows
  // a body-first order followed by the lower-left, lower-right, and lower-center
  // dots. Keep that Urdu-scoped evidence distinct from jīm's dot-first motion.
  [
    "urdu-nastaliq:چ",
    {
      script: "urdu-nastaliq",
      glyph: "چ",
      strokes: independentCheStrokes(),
      source: urduAlphabetSource("چ"),
    },
  ],
  [
    "urdu-nastaliq:د",
    {
      script: "urdu-nastaliq",
      glyph: "د",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the independent form's upper tip and descend through the folded shoulder",
              path: [
                { x: 270, y: 350 },
                { x: 260, y: 325 },
                { x: 260, y: 300 },
                { x: 270, y: 275 },
                { x: 285, y: 245 },
                { x: 300, y: 215 },
                { x: 318, y: 185 },
                { x: 333, y: 155 },
                { x: 343, y: 130 },
                { x: 345, y: 110 },
                { x: 342, y: 100 },
              ],
            },
            {
              label: "turn left along the baseline without lifting",
              path: [
                { x: 342, y: 100 },
                { x: 320, y: 90 },
                { x: 290, y: 75 },
                { x: 250, y: 60 },
                { x: 210, y: 50 },
                { x: 170, y: 40 },
                { x: 130, y: 40 },
                { x: 90, y: 50 },
                { x: 60, y: 65 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("د"),
    },
  ],
  [
    "urdu-nastaliq:ر",
    {
      script: "urdu-nastaliq",
      glyph: "ر",
      strokes: [
        {
          segments: [
            {
              label: "draw the downward line",
              path: [
                { x: 250, y: 320 },
                { x: 248, y: 280 },
                { x: 255, y: 235 },
                { x: 270, y: 190 },
                { x: 287, y: 145 },
                { x: 300, y: 95 },
                { x: 304, y: 48 },
              ],
            },
            {
              label: "continue curving to the left",
              path: [
                { x: 304, y: 48 },
                { x: 298, y: 8 },
                { x: 284, y: -30 },
                { x: 260, y: -68 },
                { x: 226, y: -103 },
                { x: 185, y: -130 },
                { x: 140, y: -146 },
                { x: 95, y: -151 },
                { x: 52, y: -147 },
                { x: 10, y: -136 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ر"),
    },
  ],
  // Zer o Zabar demonstrates the independent re-series body first, then adds
  // the small to'e-shaped retroflex mark after one lift. The body follows the
  // independently sourced Urdu re geometry; the mark is fitted to ڑ itself.
  [
    "urdu-nastaliq:ڑ",
    {
      script: "urdu-nastaliq",
      glyph: "ڑ",
      strokes: [
        {
          segments: [
            {
              label: "draw the independent re-series body downward",
              path: [
                { x: 250, y: 320 },
                { x: 248, y: 280 },
                { x: 255, y: 235 },
                { x: 270, y: 190 },
                { x: 287, y: 145 },
                { x: 300, y: 95 },
                { x: 304, y: 48 },
              ],
            },
            {
              label: "continue curving to the left",
              path: [
                { x: 304, y: 48 },
                { x: 298, y: 8 },
                { x: 284, y: -30 },
                { x: 260, y: -68 },
                { x: 226, y: -103 },
                { x: 185, y: -130 },
                { x: 140, y: -146 },
                { x: 95, y: -151 },
                { x: 52, y: -147 },
                { x: 10, y: -136 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "after one lift, draw the small retroflex mark downward, back upward, and down again to close its loop",
              path: [
                { x: 172, y: 580 },
                { x: 172, y: 540 },
                { x: 172, y: 510 },
                { x: 172, y: 540 },
                { x: 172, y: 580 },
                { x: 172, y: 540 },
                { x: 172, y: 510 },
                { x: 174, y: 480 },
                { x: 170, y: 450 },
                { x: 108, y: 436 },
                { x: 140, y: 420 },
                { x: 200, y: 412 },
                { x: 260, y: 420 },
                { x: 300, y: 440 },
                { x: 316, y: 470 },
                { x: 292, y: 500 },
                { x: 250, y: 524 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ڑ"),
    },
  ],
  [
    "urdu-nastaliq:و",
    {
      script: "urdu-nastaliq",
      glyph: "و",
      strokes: [
        {
          segments: [
            {
              label: "shape the independent wāw's looped head",
              path: [
                { x: 220, y: 300 },
                { x: 265, y: 315 },
                { x: 315, y: 285 },
                { x: 355, y: 235 },
                { x: 385, y: 170 },
                { x: 393, y: 115 },
                { x: 380, y: 70 },
                { x: 340, y: 45 },
                { x: 285, y: 40 },
                { x: 225, y: 45 },
                { x: 175, y: 80 },
                { x: 145, y: 125 },
                { x: 145, y: 165 },
                { x: 170, y: 215 },
                { x: 210, y: 260 },
                { x: 250, y: 285 },
                { x: 300, y: 285 },
                { x: 345, y: 245 },
                { x: 375, y: 185 },
                { x: 390, y: 115 },
                { x: 390, y: 60 },
              ],
            },
            {
              label: "continue down and left through the tail without lifting",
              path: [
                { x: 390, y: 60 },
                { x: 370, y: -5 },
                { x: 340, y: -70 },
                { x: 300, y: -120 },
                { x: 250, y: -160 },
                { x: 195, y: -170 },
                { x: 135, y: -160 },
                { x: 80, y: -140 },
                { x: 45, y: -120 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("و"),
    },
  ],
  [
    "urdu-nastaliq:س",
    {
      script: "urdu-nastaliq",
      glyph: "س",
      strokes: [
        {
          segments: [
            {
              label: "shape the three close teeth from right to left",
              path: [
                { x: 923, y: 310 },
                { x: 935, y: 120 },
                { x: 925, y: 70 },
                { x: 870, y: 45 },
                { x: 770, y: 75 },
                { x: 748, y: 110 },
                { x: 748, y: 230 },
                { x: 690, y: 65 },
                { x: 640, y: 45 },
                { x: 540, y: 55 },
                { x: 478, y: 190 },
                { x: 515, y: 20 },
              ],
            },
            {
              label: "flow directly into the final bowl without lifting",
              path: [
                { x: 515, y: 20 },
                { x: 515, y: -25 },
                { x: 470, y: -125 },
                { x: 370, y: -205 },
                { x: 250, y: -230 },
                { x: 145, y: -180 },
                { x: 92, y: -95 },
                { x: 110, y: 35 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("س"),
    },
  ],
  [
    "urdu-nastaliq:ش",
    {
      script: "urdu-nastaliq",
      glyph: "ش",
      strokes: [
        {
          segments: [
            {
              label: "shape the three close teeth from right to left",
              path: [
                { x: 923, y: 310 },
                { x: 935, y: 120 },
                { x: 925, y: 70 },
                { x: 870, y: 45 },
                { x: 770, y: 75 },
                { x: 748, y: 110 },
                { x: 748, y: 230 },
                { x: 690, y: 65 },
                { x: 640, y: 45 },
                { x: 540, y: 55 },
                { x: 478, y: 190 },
                { x: 515, y: 20 },
              ],
            },
            {
              label: "flow directly into the final bowl without lifting",
              path: [
                { x: 515, y: 20 },
                { x: 515, y: -25 },
                { x: 470, y: -125 },
                { x: 370, y: -205 },
                { x: 250, y: -230 },
                { x: 145, y: -180 },
                { x: 92, y: -95 },
                { x: 110, y: 35 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the lower-left dot",
              path: [
                { x: 610, y: 360 },
                { x: 648, y: 410 },
                { x: 686, y: 365 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again, then place the lower-right dot",
              path: [
                { x: 753, y: 370 },
                { x: 792, y: 423 },
                { x: 830, y: 376 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift a third time, then place the centered upper dot",
              path: [
                { x: 684, y: 446 },
                { x: 720, y: 494 },
                { x: 757, y: 446 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ش"),
    },
  ],
  [
    "urdu-nastaliq:ک",
    {
      script: "urdu-nastaliq",
      glyph: "ک",
      strokes: [
        {
          segments: [
            {
              label: "draw the independent stem downward",
              path: [
                { x: 620, y: 250 },
                { x: 622, y: 200 },
                { x: 620, y: 150 },
              ],
            },
            {
              label:
                "flow right to left through the flatter bowl and finish with the hook without lifting",
              path: [
                { x: 620, y: 150 },
                { x: 570, y: 100 },
                { x: 500, y: 65 },
                { x: 400, y: 40 },
                { x: 300, y: 35 },
                { x: 210, y: 50 },
                { x: 140, y: 85 },
                { x: 95, y: 125 },
                { x: 95, y: 185 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "lift, then draw the long slash down from the upper right toward the stem",
              path: [
                { x: 680, y: 625 },
                { x: 600, y: 590 },
                { x: 520, y: 550 },
                { x: 440, y: 510 },
                { x: 365, y: 470 },
                { x: 335, y: 425 },
                { x: 355, y: 400 },
                { x: 390, y: 380 },
                { x: 425, y: 360 },
                { x: 460, y: 340 },
                { x: 480, y: 320 },
                { x: 520, y: 300 },
                { x: 540, y: 280 },
                { x: 560, y: 260 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ک"),
    },
  ],
  // Zer o Zabar's independent animations build گ from the same kāf-family
  // body and long slash, then add the shorter floating slash above it.
  [
    "urdu-nastaliq:گ",
    {
      script: "urdu-nastaliq",
      glyph: "گ",
      strokes: [
        {
          segments: [
            {
              label: "draw the independent stem downward",
              path: [
                { x: 620, y: 250 },
                { x: 622, y: 200 },
                { x: 620, y: 150 },
              ],
            },
            {
              label:
                "flow right to left through the flatter bowl and finish with the hook without lifting",
              path: [
                { x: 620, y: 150 },
                { x: 570, y: 100 },
                { x: 500, y: 65 },
                { x: 400, y: 40 },
                { x: 300, y: 35 },
                { x: 210, y: 50 },
                { x: 140, y: 85 },
                { x: 95, y: 125 },
                { x: 95, y: 185 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "after one lift, draw the long slash down from the upper right toward the stem",
              path: [
                { x: 680, y: 625 },
                { x: 600, y: 590 },
                { x: 520, y: 550 },
                { x: 440, y: 510 },
                { x: 365, y: 470 },
                { x: 335, y: 425 },
                { x: 355, y: 400 },
                { x: 390, y: 380 },
                { x: 425, y: 360 },
                { x: 460, y: 340 },
                { x: 480, y: 320 },
                { x: 520, y: 300 },
                { x: 540, y: 280 },
                { x: 560, y: 260 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "after another lift, draw the shorter floating slash above the first",
              path: [
                { x: 650, y: 705 },
                { x: 575, y: 670 },
                { x: 500, y: 635 },
                { x: 425, y: 600 },
                { x: 350, y: 565 },
                { x: 300, y: 540 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("گ"),
    },
  ],
  [
    "urdu-nastaliq:ل",
    {
      script: "urdu-nastaliq",
      glyph: "ل",
      strokes: [
        {
          segments: [
            {
              label: "draw the tall independent upright downward",
              path: [
                { x: 458, y: 640 },
                { x: 445, y: 500 },
                { x: 440, y: 420 },
                { x: 450, y: 240 },
                { x: 475, y: 80 },
                { x: 510, y: -20 },
              ],
            },
            {
              label:
                "continue below the baseline through the leftward bowl and back up without lifting",
              path: [
                { x: 510, y: -20 },
                { x: 465, y: -120 },
                { x: 350, y: -205 },
                { x: 205, y: -215 },
                { x: 105, y: -135 },
                { x: 90, y: -75 },
                { x: 100, y: 25 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ل"),
    },
  ],
  [
    "urdu-nastaliq:م",
    {
      script: "urdu-nastaliq",
      glyph: "م",
      strokes: [
        {
          segments: [
            {
              label: "shape the round head",
              path: [
                { x: 120, y: 210 },
                { x: 150, y: 250 },
                { x: 200, y: 300 },
                { x: 245, y: 315 },
                { x: 285, y: 300 },
                { x: 330, y: 260 },
                { x: 365, y: 215 },
                { x: 400, y: 175 },
                { x: 430, y: 150 },
              ],
            },
            {
              label:
                "continue down the tail below the baseline without lifting",
              path: [
                { x: 430, y: 150 },
                { x: 390, y: 110 },
                { x: 330, y: 95 },
                { x: 260, y: 80 },
                { x: 180, y: 65 },
                { x: 100, y: 35 },
                { x: 90, y: -20 },
                { x: 100, y: -90 },
                { x: 110, y: -160 },
                { x: 120, y: -240 },
                { x: 105, y: -285 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("م"),
    },
  ],
  [
    "urdu-nastaliq:ن",
    {
      script: "urdu-nastaliq",
      glyph: "ن",
      strokes: [
        {
          segments: [
            {
              label:
                "sweep the independent bowl right to left below the baseline",
              path: [
                { x: 495, y: 210 },
                { x: 475, y: 160 },
                { x: 480, y: 100 },
                { x: 500, y: 40 },
                { x: 510, y: -20 },
                { x: 485, y: -80 },
                { x: 430, y: -140 },
                { x: 360, y: -190 },
                { x: 280, y: -220 },
                { x: 210, y: -215 },
                { x: 150, y: -170 },
                { x: 105, y: -110 },
                { x: 90, y: -60 },
                { x: 95, y: 0 },
                { x: 105, y: 45 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the dot near the baseline",
              path: [
                { x: 235, y: 305 },
                { x: 275, y: 345 },
                { x: 315, y: 305 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ن"),
    },
  ],
  [
    "urdu-nastaliq:ں",
    {
      script: "urdu-nastaliq",
      glyph: "ں",
      strokes: [
        {
          segments: [
            {
              label:
                "sweep the independent dotless bowl right to left below the baseline",
              path: [
                { x: 495, y: 210 },
                { x: 475, y: 160 },
                { x: 480, y: 100 },
                { x: 500, y: 40 },
                { x: 510, y: -20 },
                { x: 485, y: -80 },
                { x: 430, y: -140 },
                { x: 360, y: -190 },
                { x: 280, y: -220 },
                { x: 210, y: -215 },
                { x: 150, y: -170 },
                { x: 105, y: -110 },
                { x: 90, y: -60 },
                { x: 95, y: 0 },
                { x: 105, y: 45 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ں"),
    },
  ],
  [
    "urdu-nastaliq:ہ",
    {
      script: "urdu-nastaliq",
      glyph: "ہ",
      strokes: [
        {
          segments: [
            {
              label:
                "loop the independent teardrop counterclockwise without lifting",
              path: [
                { x: 250, y: 330 },
                { x: 190, y: 305 },
                { x: 150, y: 280 },
                { x: 120, y: 230 },
                { x: 95, y: 170 },
                { x: 90, y: 115 },
                { x: 115, y: 70 },
                { x: 155, y: 40 },
                { x: 200, y: 30 },
                { x: 245, y: 40 },
                { x: 290, y: 70 },
                { x: 320, y: 110 },
                { x: 330, y: 160 },
                { x: 325, y: 210 },
                { x: 305, y: 255 },
                { x: 275, y: 295 },
                { x: 235, y: 330 },
                { x: 195, y: 360 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ہ"),
    },
  ],
  // Zer o Zabar's independent calligraphic and handwriting animations both
  // keep the two eyes and low finish in one continuous motion. This Noto Naskh
  // fallback fit preserves the right-eye-first order and the reversal at the
  // far-left baseline without borrowing chhoṭī he's separate path.
  [
    "urdu-nastaliq:ھ",
    {
      script: "urdu-nastaliq",
      glyph: "ھ",
      strokes: [
        {
          segments: [
            {
              label: "circle the right eye clockwise from the upper center",
              path: [
                { x: 285, y: 395 },
                { x: 350, y: 375 },
                { x: 420, y: 335 },
                { x: 480, y: 280 },
                { x: 525, y: 210 },
                { x: 550, y: 135 },
                { x: 550, y: 75 },
                { x: 535, y: 35 },
                { x: 500, y: 18 },
              ],
            },
            {
              label:
                "continue down and left along the baseline without lifting",
              path: [
                { x: 500, y: 18 },
                { x: 450, y: 35 },
                { x: 400, y: 52 },
                { x: 350, y: 72 },
                { x: 307, y: 90 },
                { x: 250, y: 62 },
                { x: 185, y: 45 },
                { x: 115, y: 45 },
                { x: 55, y: 60 },
              ],
            },
            {
              label: "reverse at the left edge and rise around the left eye",
              path: [
                { x: 55, y: 60 },
                { x: 105, y: 65 },
                { x: 155, y: 70 },
                { x: 205, y: 78 },
                { x: 235, y: 92 },
                { x: 205, y: 130 },
                { x: 185, y: 175 },
                { x: 198, y: 235 },
                { x: 215, y: 285 },
                { x: 242, y: 335 },
                { x: 275, y: 390 },
                { x: 310, y: 370 },
                { x: 345, y: 335 },
                { x: 375, y: 290 },
                { x: 390, y: 240 },
                { x: 385, y: 190 },
                { x: 360, y: 140 },
                { x: 315, y: 95 },
              ],
            },
            {
              label:
                "close at the center and finish with the low leftward sweep",
              path: [
                { x: 315, y: 95 },
                { x: 275, y: 72 },
                { x: 225, y: 55 },
                { x: 170, y: 48 },
                { x: 115, y: 50 },
                { x: 70, y: 58 },
                { x: 30, y: 68 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ھ"),
    },
  ],
  [
    "urdu-nastaliq:ی",
    {
      script: "urdu-nastaliq",
      glyph: "ی",
      strokes: independentYehStrokes(),
      source: urduAlphabetSource("ی"),
    },
  ],
  // Persian Online independently demonstrates the same dotless isolated body
  // as one continuous S-shaped run. Share only the Noto fallback geometry;
  // the scoped key keeps its Persian source separate from the Urdu record.
  [
    "perso-arabic:ی",
    {
      script: "perso-arabic",
      glyph: "ی",
      strokes: independentYehStrokes(
        "sweep left from the upper right and descend through the S curve",
        "continue around the below-baseline bowl and finish at its rising tip without lifting",
      ),
      source: persianAlphabetSource("ی"),
    },
  ],
  [
    "urdu-nastaliq:ے",
    {
      script: "urdu-nastaliq",
      glyph: "ے",
      strokes: [
        {
          segments: [
            {
              label:
                "descend from the upper right and sweep left across the broad bowl",
              path: [
                { x: 360, y: 280 },
                { x: 350, y: 275 },
                { x: 330, y: 252 },
                { x: 310, y: 238 },
                { x: 292, y: 230 },
                { x: 250, y: 215 },
                { x: 200, y: 195 },
                { x: 150, y: 173 },
                { x: 115, y: 145 },
                { x: 100, y: 110 },
              ],
            },
            {
              label: "curl back underneath at the far left without lifting",
              path: [
                { x: 100, y: 110 },
                { x: 90, y: 95 },
                { x: 82, y: 78 },
                { x: 82, y: 62 },
                { x: 95, y: 55 },
                { x: 120, y: 52 },
              ],
            },
            {
              label: "continue right along the lower fold without lifting",
              path: [
                { x: 120, y: 52 },
                { x: 170, y: 30 },
                { x: 250, y: 20 },
                { x: 350, y: 12 },
                { x: 450, y: 10 },
                { x: 550, y: 20 },
                { x: 650, y: 40 },
                { x: 720, y: 62 },
                { x: 740, y: 90 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ے"),
    },
  ],
  [
    "ب",
    {
      script: "perso-arabic",
      glyph: "ب",
      strokes: [
        {
          segments: [
            {
              label: "sweep the shallow bowl from right to left",
              path: [
                { x: 678, y: 382 },
                { x: 663, y: 345 },
                { x: 650, y: 305 },
                { x: 654, y: 260 },
                { x: 672, y: 215 },
                { x: 688, y: 170 },
                { x: 686, y: 126 },
                { x: 620, y: 94 },
                { x: 530, y: 65 },
                { x: 430, y: 42 },
                { x: 335, y: 38 },
                { x: 245, y: 51 },
                { x: 170, y: 83 },
                { x: 120, y: 135 },
                { x: 96, y: 205 },
                { x: 100, y: 255 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the dot below",
              path: [
                { x: 412, y: -137 },
                { x: 379, y: -101 },
                { x: 344, y: -137 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ب"),
    },
  ],
  // Zer o Zabar demonstrates Urdu independent be as the be-series main line
  // first, followed after one lift by its single lower dot. Share only the
  // checked Noto fallback geometry with Arabic and Persian; the scoped key
  // keeps the Urdu handwriting animation and prose independently addressable.
  [
    "urdu-nastaliq:ب",
    {
      script: "urdu-nastaliq",
      glyph: "ب",
      strokes: independentUrduBehStrokes(),
      source: urduAlphabetSource("ب"),
    },
  ],
  [
    "پ",
    {
      script: "perso-arabic",
      glyph: "پ",
      strokes: independentPehStrokes(
        "sweep the shallow bowl from right to left",
        "lift, then place the left dot below",
        "lift again and place the right dot below",
        "lift again and place the lower-center dot",
      ),
      source: persianAlphabetSource("پ"),
    },
  ],
  // Northwestern independently demonstrates the same bowl-left, bowl-right,
  // lower-center dot order for Urdu. Only the Noto fallback geometry is shared;
  // the scoped key keeps the Urdu source distinct from the Persian record.
  [
    "urdu-nastaliq:پ",
    {
      script: "urdu-nastaliq",
      glyph: "پ",
      strokes: independentPehStrokes(),
      source: urduAlphabetSource("پ"),
    },
  ],
  // Zer o Zabar demonstrates the independent be-series bowl first, followed
  // by two separately placed dots side by side above the main line. Share only
  // the checked Noto fallback geometry; the scoped key keeps Urdu provenance
  // separate from the independently verified Arabic and Persian ت records.
  [
    "urdu-nastaliq:ت",
    {
      script: "urdu-nastaliq",
      glyph: "ت",
      strokes: independentUrduTeStrokes(),
      source: urduAlphabetSource("ت"),
    },
  ],
  // Zer o Zabar demonstrates the independent be-series bowl first, followed
  // after one lift by the small to'e-shaped retroflex mark above it. Share the
  // bowl's Noto fallback geometry with Urdu pe while retaining ṭe's own source.
  [
    "urdu-nastaliq:ٹ",
    {
      script: "urdu-nastaliq",
      glyph: "ٹ",
      strokes: independentUrduTteStrokes(),
      source: urduAlphabetSource("ٹ"),
    },
  ],
  // Persian Online independently demonstrates che body-first, then places the
  // three lower dots left, right, and lower-center. Share only Noto geometry
  // with the Urdu filmstrip; the scoped key preserves its Persian provenance.
  [
    "perso-arabic:چ",
    {
      script: "perso-arabic",
      glyph: "چ",
      strokes: independentCheStrokes(
        "draw the short upper head from left to right",
        "continue down and around the deep bowl without lifting",
        "lift, then place the lower-left dot",
        "lift again and place the lower-right dot",
        "lift again and place the lower-center dot",
      ),
      source: persianAlphabetSource("چ"),
    },
  ],
  [
    "ت",
    {
      script: "perso-arabic",
      glyph: "ت",
      strokes: [
        {
          segments: [
            {
              label: "sweep the shallow bowl from right to left",
              path: [
                { x: 678, y: 382 },
                { x: 663, y: 345 },
                { x: 650, y: 305 },
                { x: 654, y: 260 },
                { x: 672, y: 215 },
                { x: 688, y: 170 },
                { x: 686, y: 126 },
                { x: 620, y: 94 },
                { x: 530, y: 65 },
                { x: 430, y: 42 },
                { x: 335, y: 38 },
                { x: 245, y: 51 },
                { x: 170, y: 83 },
                { x: 120, y: 135 },
                { x: 96, y: 205 },
                { x: 100, y: 255 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the left dot above",
              path: [
                { x: 247, y: 374 },
                { x: 284, y: 412 },
                { x: 319, y: 379 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again and place the right dot",
              path: [
                { x: 395, y: 389 },
                { x: 434, y: 430 },
                { x: 470, y: 395 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ت"),
    },
  ],
  [
    "د",
    {
      script: "perso-arabic",
      glyph: "د",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper tip and descend through the folded shoulder",
              path: [
                { x: 270, y: 350 },
                { x: 260, y: 325 },
                { x: 260, y: 300 },
                { x: 270, y: 275 },
                { x: 285, y: 245 },
                { x: 300, y: 215 },
                { x: 318, y: 185 },
                { x: 333, y: 155 },
                { x: 343, y: 130 },
                { x: 345, y: 110 },
                { x: 342, y: 100 },
              ],
            },
            {
              label: "turn left along the baseline without lifting",
              path: [
                { x: 342, y: 100 },
                { x: 320, y: 90 },
                { x: 290, y: 75 },
                { x: 250, y: 60 },
                { x: 210, y: 50 },
                { x: 170, y: 40 },
                { x: 130, y: 40 },
                { x: 90, y: 50 },
                { x: 60, y: 65 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("د"),
    },
  ],
  [
    "perso-arabic:ر",
    {
      script: "perso-arabic",
      glyph: "ر",
      strokes: [
        {
          segments: [
            {
              label:
                "begin at the upper tip and descend through the short stroke",
              path: [
                { x: 250, y: 320 },
                { x: 248, y: 280 },
                { x: 255, y: 235 },
                { x: 270, y: 190 },
                { x: 287, y: 145 },
                { x: 300, y: 95 },
                { x: 304, y: 48 },
              ],
            },
            {
              label: "without lifting, sweep left through the lower curve",
              path: [
                { x: 304, y: 48 },
                { x: 298, y: 8 },
                { x: 284, y: -30 },
                { x: 260, y: -68 },
                { x: 226, y: -103 },
                { x: 185, y: -130 },
                { x: 140, y: -146 },
                { x: 95, y: -151 },
                { x: 52, y: -147 },
                { x: 10, y: -136 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ر"),
    },
  ],
  [
    "س",
    {
      script: "perso-arabic",
      glyph: "س",
      strokes: [
        {
          segments: [
            {
              label: "form the three teeth from right to left",
              path: [
                { x: 923, y: 310 },
                { x: 935, y: 120 },
                { x: 925, y: 70 },
                { x: 870, y: 45 },
                { x: 770, y: 75 },
                { x: 748, y: 110 },
                { x: 748, y: 230 },
                { x: 690, y: 65 },
                { x: 640, y: 45 },
                { x: 540, y: 55 },
                { x: 478, y: 190 },
                { x: 515, y: 20 },
              ],
            },
            {
              label: "flow into the final bowl without lifting",
              path: [
                { x: 515, y: 20 },
                { x: 515, y: -25 },
                { x: 470, y: -125 },
                { x: 370, y: -205 },
                { x: 250, y: -230 },
                { x: 145, y: -180 },
                { x: 92, y: -95 },
                { x: 110, y: 35 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("س"),
    },
  ],
  [
    "perso-arabic:ش",
    {
      script: "perso-arabic",
      glyph: "ش",
      strokes: [
        {
          segments: [
            {
              label: "form the three teeth from right to left",
              path: [
                { x: 923, y: 310 },
                { x: 935, y: 120 },
                { x: 925, y: 70 },
                { x: 870, y: 45 },
                { x: 770, y: 75 },
                { x: 748, y: 110 },
                { x: 748, y: 230 },
                { x: 690, y: 65 },
                { x: 640, y: 45 },
                { x: 540, y: 55 },
                { x: 478, y: 190 },
                { x: 515, y: 20 },
              ],
            },
            {
              label: "flow into the final bowl without lifting",
              path: [
                { x: 515, y: 20 },
                { x: 515, y: -25 },
                { x: 470, y: -125 },
                { x: 370, y: -205 },
                { x: 250, y: -230 },
                { x: 145, y: -180 },
                { x: 92, y: -95 },
                { x: 110, y: 35 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the lower-left dot",
              path: [
                { x: 610, y: 360 },
                { x: 648, y: 410 },
                { x: 686, y: 365 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again and place the lower-right dot",
              path: [
                { x: 753, y: 370 },
                { x: 792, y: 423 },
                { x: 830, y: 376 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift again and place the centered upper dot",
              path: [
                { x: 684, y: 446 },
                { x: 720, y: 494 },
                { x: 757, y: 446 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ش"),
    },
  ],
  [
    "ل",
    {
      script: "perso-arabic",
      glyph: "ل",
      strokes: [
        {
          segments: [
            {
              label: "draw the upright downward",
              path: [
                { x: 458, y: 640 },
                { x: 445, y: 500 },
                { x: 440, y: 420 },
                { x: 450, y: 240 },
                { x: 475, y: 80 },
                { x: 510, y: -20 },
              ],
            },
            {
              label: "turn into the base curve without lifting",
              path: [
                { x: 510, y: -20 },
                { x: 465, y: -120 },
                { x: 350, y: -205 },
                { x: 205, y: -215 },
                { x: 105, y: -135 },
                { x: 90, y: -75 },
                { x: 100, y: 25 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ل"),
    },
  ],
  [
    "م",
    {
      script: "perso-arabic",
      glyph: "م",
      strokes: [
        {
          segments: [
            {
              label: "shape the round head",
              path: [
                { x: 120, y: 210 },
                { x: 150, y: 250 },
                { x: 200, y: 300 },
                { x: 245, y: 315 },
                { x: 285, y: 300 },
                { x: 330, y: 260 },
                { x: 365, y: 215 },
                { x: 400, y: 175 },
                { x: 430, y: 150 },
              ],
            },
            {
              label: "continue down the tail without lifting",
              path: [
                { x: 430, y: 150 },
                { x: 390, y: 110 },
                { x: 330, y: 95 },
                { x: 260, y: 80 },
                { x: 180, y: 65 },
                { x: 100, y: 35 },
                { x: 90, y: -20 },
                { x: 100, y: -90 },
                { x: 110, y: -160 },
                { x: 120, y: -240 },
                { x: 105, y: -285 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("م"),
    },
  ],
  [
    "ن",
    {
      script: "perso-arabic",
      glyph: "ن",
      strokes: [
        {
          segments: [
            {
              label: "sweep the bowl from right to left",
              path: [
                { x: 495, y: 210 },
                { x: 475, y: 160 },
                { x: 480, y: 100 },
                { x: 500, y: 40 },
                { x: 510, y: -20 },
                { x: 485, y: -80 },
                { x: 430, y: -140 },
                { x: 360, y: -190 },
                { x: 280, y: -220 },
                { x: 210, y: -215 },
                { x: 150, y: -170 },
                { x: 105, y: -110 },
                { x: 90, y: -60 },
                { x: 95, y: 0 },
                { x: 105, y: 45 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "lift, then place the dot above",
              path: [
                { x: 235, y: 305 },
                { x: 275, y: 345 },
                { x: 315, y: 305 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ن"),
    },
  ],
  [
    "و",
    {
      script: "perso-arabic",
      glyph: "و",
      strokes: [
        {
          segments: [
            {
              label: "shape the small head loop",
              path: [
                { x: 220, y: 300 },
                { x: 265, y: 315 },
                { x: 315, y: 285 },
                { x: 355, y: 235 },
                { x: 385, y: 170 },
                { x: 393, y: 115 },
                { x: 380, y: 70 },
                { x: 340, y: 45 },
                { x: 285, y: 40 },
                { x: 225, y: 45 },
                { x: 175, y: 80 },
                { x: 145, y: 125 },
                { x: 145, y: 165 },
                { x: 170, y: 215 },
                { x: 210, y: 260 },
                { x: 250, y: 285 },
                { x: 300, y: 285 },
                { x: 345, y: 245 },
                { x: 375, y: 185 },
                { x: 390, y: 115 },
                { x: 390, y: 60 },
              ],
            },
            {
              label: "flow into the leftward tail without lifting",
              path: [
                { x: 390, y: 60 },
                { x: 370, y: -5 },
                { x: 340, y: -70 },
                { x: 300, y: -120 },
                { x: 250, y: -160 },
                { x: 195, y: -170 },
                { x: 135, y: -160 },
                { x: 80, y: -140 },
                { x: 45, y: -120 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("و"),
    },
  ],
  [
    "ه",
    {
      script: "perso-arabic",
      glyph: "ه",
      strokes: [
        {
          segments: [
            {
              label: "loop the isolated body and finish left without lifting",
              path: [
                { x: 315, y: 400 },
                { x: 285, y: 375 },
                { x: 255, y: 350 },
                { x: 230, y: 325 },
                { x: 205, y: 300 },
                { x: 190, y: 260 },
                { x: 190, y: 210 },
                { x: 205, y: 165 },
                { x: 235, y: 125 },
                { x: 275, y: 105 },
                { x: 320, y: 110 },
                { x: 355, y: 135 },
                { x: 380, y: 175 },
                { x: 390, y: 225 },
                { x: 380, y: 275 },
                { x: 355, y: 320 },
                { x: 315, y: 355 },
                { x: 360, y: 355 },
                { x: 410, y: 340 },
                { x: 455, y: 315 },
                { x: 500, y: 275 },
                { x: 535, y: 225 },
                { x: 555, y: 170 },
                { x: 555, y: 115 },
                { x: 535, y: 70 },
                { x: 535, y: 50 },
                { x: 500, y: 40 },
                { x: 455, y: 30 },
                { x: 415, y: 45 },
                { x: 385, y: 75 },
                { x: 365, y: 100 },
                { x: 345, y: 75 },
                { x: 310, y: 65 },
                { x: 270, y: 65 },
                { x: 225, y: 70 },
                { x: 175, y: 65 },
                { x: 120, y: 65 },
                { x: 70, y: 65 },
                { x: 25, y: 65 },
              ],
            },
          ],
        },
      ],
      source: persianAlphabetSource("ه"),
    },
  ],
];

const mainRegistry = Object.fromEntries(mainEntries) as Record<
  string,
  LetterDuctus
>;
const lookup = (key: string): LetterDuctus => {
  const letter = mainRegistry[key];
  if (letter === undefined)
    throw new Error(`Script Ductus Arabic family has no ${key}`);
  return letter;
};

// Persian Online and Zer o Zabar independently attest the same body-upright
// construction for ط. The scoped entries share only the fitted Noto Naskh
// geometry with Arabic while retaining language-specific provenance.
const [sharedTahBody, sharedTahUpright] = lookup("arabic:ط").strokes;

// Persian Online and Zer o Zabar independently attest the same body-upright-dot
// construction for ظ. The scoped entries intentionally share only the Noto
// fallback geometry with Arabic while retaining language-specific provenance.
const [sharedZahBody, sharedZahDot, sharedZahUpright] =
  lookup("arabic:ظ").strokes;

// Persian Online independently demonstrates the same two-run ک construction as
// Urdu: main-line body first, then the long slash. Share only fallback geometry.
const urduKafStrokes = lookup("urdu-nastaliq:ک").strokes;

// Persian Online independently demonstrates the same three-run گ construction
// as Urdu: main-line body first, then the long and shorter floating slashes.
// Share only the Noto Naskh fallback geometry, never the source provenance.
const urduGafStrokes = lookup("urdu-nastaliq:گ").strokes;

// Persian Online and Zer o Zabar independently demonstrate ز body-first and
// dot-second. Share only the already fitted Arabic Noto Naskh geometry.
const [sharedZayBody, sharedZayDot] = lookup("arabic:ز").strokes;

// These derived identities historically followed every other owner block.
export const tailEntries: DuctusEntry[] = [
  [
    "perso-arabic:ط",
    {
      script: "perso-arabic",
      glyph: "ط",
      strokes: [
        sharedTahBody,
        {
          segments: [
            {
              ...sharedTahUpright.segments[0],
              label: "lift once, then draw the tall upright top-to-bottom",
            },
          ],
        },
      ],
      source: persianAlphabetSource("ط"),
    },
  ],
  [
    "urdu-nastaliq:ط",
    {
      script: "urdu-nastaliq",
      glyph: "ط",
      strokes: [
        {
          segments: [
            {
              ...sharedTahBody.segments[0],
              label: "draw the independent to'e-series loop",
            },
            {
              ...sharedTahBody.segments[1],
              label: "continue through its leftward finish without lifting",
            },
          ],
        },
        {
          segments: [
            {
              ...sharedTahUpright.segments[0],
              label: "after one lift, draw the tall upright",
            },
          ],
        },
      ],
      source: urduAlphabetSource("ط"),
    },
  ],
  [
    "perso-arabic:ظ",
    {
      script: "perso-arabic",
      glyph: "ظ",
      strokes: [
        sharedZahBody,
        {
          segments: [
            {
              ...sharedZahUpright.segments[0],
              label: "lift once, then draw the tall upright top-to-bottom",
            },
          ],
        },
        {
          segments: [
            {
              ...sharedZahDot.segments[0],
              label: "lift again, then place the upper dot",
            },
          ],
        },
      ],
      source: persianAlphabetSource("ظ"),
    },
  ],
  [
    "urdu-nastaliq:ظ",
    {
      script: "urdu-nastaliq",
      glyph: "ظ",
      strokes: [
        sharedZahBody,
        {
          segments: [
            {
              ...sharedZahUpright.segments[0],
              label: "after one lift, draw the tall upright",
            },
          ],
        },
        {
          segments: [
            {
              ...sharedZahDot.segments[0],
              label: "after another lift, place the single dot above",
            },
          ],
        },
      ],
      source: urduAlphabetSource("ظ"),
    },
  ],
  [
    "perso-arabic:ک",
    {
      script: "perso-arabic",
      glyph: "ک",
      strokes: [
        {
          segments: [
            {
              ...urduKafStrokes[0].segments[0],
              label: "draw the independent stem downward",
            },
            {
              ...urduKafStrokes[0].segments[1],
              label:
                "continue left through the shallow bowl and final hook without lifting",
            },
          ],
        },
        {
          segments: [
            {
              ...urduKafStrokes[1].segments[0],
              label:
                "lift once, then draw the long slash down from the upper right toward the stem",
            },
          ],
        },
      ],
      source: persianAlphabetSource("ک"),
    },
  ],
  [
    "perso-arabic:گ",
    {
      script: "perso-arabic",
      glyph: "گ",
      strokes: [
        {
          segments: [
            {
              ...urduGafStrokes[0].segments[0],
              label: "draw the independent stem downward",
            },
            {
              ...urduGafStrokes[0].segments[1],
              label:
                "continue left through the shallow bowl and final hook without lifting",
            },
          ],
        },
        {
          segments: [
            {
              ...urduGafStrokes[1].segments[0],
              label:
                "lift once, then draw the long slash down from the upper right toward the stem",
            },
          ],
        },
        {
          segments: [
            {
              ...urduGafStrokes[2].segments[0],
              label:
                "lift again, then draw the shorter floating slash above the first",
            },
          ],
        },
      ],
      source: persianAlphabetSource("گ"),
    },
  ],
  [
    "perso-arabic:ز",
    {
      script: "perso-arabic",
      glyph: "ز",
      strokes: [
        {
          segments: [
            {
              ...sharedZayBody.segments[0],
              label:
                "begin at the upper tip and descend through the short stroke",
            },
            {
              ...sharedZayBody.segments[1],
              label: "without lifting, sweep left through the lower curve",
            },
          ],
        },
        {
          segments: [
            {
              ...sharedZayDot.segments[0],
              label: "lift once, then place the dot above",
            },
          ],
        },
      ],
      source: persianAlphabetSource("ز"),
    },
  ],
  [
    "urdu-nastaliq:ز",
    {
      script: "urdu-nastaliq",
      glyph: "ز",
      strokes: [
        {
          segments: [
            {
              ...sharedZayBody.segments[0],
              label: "draw the independent ze downward",
            },
            {
              ...sharedZayBody.segments[1],
              label: "continue curving to the left without lifting",
            },
          ],
        },
        {
          segments: [
            {
              ...sharedZayDot.segments[0],
              label: "after one lift, place the dot above",
            },
          ],
        },
      ],
      source: urduAlphabetSource("ز"),
    },
  ],
  [
    "urdu-nastaliq:ض",
    {
      script: "urdu-nastaliq",
      glyph: "ض",
      strokes: [
        {
          segments: [
            {
              label:
                "close the elongated upper oval clockwise and finish its short tooth",
              path: [
                { x: 535, y: 30 },
                { x: 560, y: 90 },
                { x: 620, y: 160 },
                { x: 700, y: 230 },
                { x: 790, y: 305 },
                { x: 870, y: 320 },
                { x: 950, y: 285 },
                { x: 1010, y: 230 },
                { x: 1015, y: 175 },
                { x: 970, y: 115 },
                { x: 900, y: 70 },
                { x: 810, y: 45 },
                { x: 720, y: 38 },
                { x: 630, y: 42 },
                { x: 535, y: 30 },
                { x: 530, y: 65 },
                { x: 520, y: 105 },
                { x: 510, y: 145 },
                { x: 495, y: 190 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label:
                "after one lift, restart below the oval and sweep through the lower bowl from right to left",
              path: [
                { x: 500, y: -54 },
                { x: 475, y: -115 },
                { x: 425, y: -175 },
                { x: 360, y: -215 },
                { x: 280, y: -232 },
                { x: 205, y: -225 },
                { x: 145, y: -185 },
                { x: 105, y: -125 },
                { x: 92, y: -65 },
                { x: 100, y: 20 },
              ],
            },
          ],
        },
        {
          segments: [
            {
              label: "after another lift, place the dot above last",
              path: [
                { x: 725, y: 470 },
                { x: 675, y: 515 },
                { x: 725, y: 568 },
                { x: 770, y: 520 },
                { x: 725, y: 470 },
              ],
            },
          ],
        },
      ],
      source: urduAlphabetSource("ض"),
    },
  ],
];
