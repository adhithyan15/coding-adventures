import { beforeAll, describe, expect, it } from "vitest";
import {
  DUCTUS,
  ductusKey,
  penPathD,
  type LetterDuctus,
} from "../../src/strokes";
import {
  ductusFilmstrip,
  ductusFor,
  ductusFrame,
  ductusSteps,
  escapeXml,
  isSafeName,
  segmentEndFractions,
  svgMarkup,
  viewBoxFor,
  wrapCaption,
  type GlyphOutline,
  type SvgNode,
} from "../../src/ductusview";
import {
  chineseOutline,
  cyrillicOutline,
  devanagariOutline,
  gujaratiOutline,
  hebrewOutline,
  japaneseOutline,
  kannadaOutline,
  malayalamOutline,
  naskhOutline,
  tamilOutline,
  teluguOutline,
} from "../support/font-fixtures";
import { byTag } from "../support/svg-tree";

const KANNADA_A = DUCTUS[ductusKey("kannada", "ಅ")];
const kannadaAOutline = kannadaOutline("ಅ");
const KANNADA_AA = DUCTUS[ductusKey("kannada", "ಆ")];
const kannadaAaOutline = kannadaOutline("ಆ");
const KANNADA_I = DUCTUS[ductusKey("kannada", "ಇ")];
const kannadaIOutline = kannadaOutline("ಇ");
const KANNADA_LONG_I = DUCTUS[ductusKey("kannada", "ಈ")];
const kannadaLongIOutline = kannadaOutline("ಈ");
const KANNADA_U = DUCTUS[ductusKey("kannada", "ಉ")];
const kannadaUOutline = kannadaOutline("ಉ");
const KANNADA_UU = DUCTUS[ductusKey("kannada", "ಊ")];
const kannadaUuOutline = kannadaOutline("ಊ");
const KANNADA_E = DUCTUS[ductusKey("kannada", "ಎ")];
const kannadaEOutline = kannadaOutline("ಎ");
const KANNADA_EE = DUCTUS[ductusKey("kannada", "ಏ")];
const kannadaEeOutline = kannadaOutline("ಏ");
const KANNADA_O = DUCTUS[ductusKey("kannada", "ಒ")];
const kannadaOOutline = kannadaOutline("ಒ");
const KANNADA_OO = DUCTUS[ductusKey("kannada", "ಓ")];
const kannadaOoOutline = kannadaOutline("ಓ");
const KANNADA_AI = DUCTUS[ductusKey("kannada", "ಐ")];
const kannadaAiOutline = kannadaOutline("ಐ");
const KANNADA_VOCALIC_R = DUCTUS[ductusKey("kannada", "ಋ")];
const kannadaVocalicROutline = kannadaOutline("ಋ");
const KANNADA_VISARGA = DUCTUS[ductusKey("kannada", "ಃ")];
const kannadaVisargaOutline = kannadaOutline("ಃ");
const KANNADA_NA = DUCTUS[ductusKey("kannada", "ನ")];
const kannadaNaOutline = kannadaOutline("ನ");
const KANNADA_TA = DUCTUS[ductusKey("kannada", "ತ")];
const kannadaTaOutline = kannadaOutline("ತ");
const KANNADA_DA = DUCTUS[ductusKey("kannada", "ದ")];
const kannadaDaOutline = kannadaOutline("ದ");
const KANNADA_RA = DUCTUS[ductusKey("kannada", "ರ")];
const kannadaRaOutline = kannadaOutline("ರ");
const KANNADA_KA = DUCTUS[ductusKey("kannada", "ಕ")];
const kannadaKaOutline = kannadaOutline("ಕ");
const KANNADA_GA = DUCTUS[ductusKey("kannada", "ಗ")];
const kannadaGaOutline = kannadaOutline("ಗ");
const KANNADA_BA = DUCTUS[ductusKey("kannada", "ಬ")];
const kannadaBaOutline = kannadaOutline("ಬ");
const KANNADA_LLA = DUCTUS[ductusKey("kannada", "ಳ")];
const kannadaLlaOutline = kannadaOutline("ಳ");
const KANNADA_YA = DUCTUS[ductusKey("kannada", "ಯ")];
const kannadaYaOutline = kannadaOutline("ಯ");
const KANNADA_DDA = DUCTUS[ductusKey("kannada", "ಡ")];
const kannadaDdaOutline = kannadaOutline("ಡ");
const KANNADA_HA = DUCTUS[ductusKey("kannada", "ಹ")];
const kannadaHaOutline = kannadaOutline("ಹ");
const KANNADA_SA = DUCTUS[ductusKey("kannada", "ಸ")];
const kannadaSaOutline = kannadaOutline("ಸ");
const KANNADA_CA = DUCTUS[ductusKey("kannada", "ಚ")];
const kannadaCaOutline = kannadaOutline("ಚ");
const KANNADA_PA = DUCTUS[ductusKey("kannada", "ಪ")];
const kannadaPaOutline = kannadaOutline("ಪ");
const KANNADA_JHA = DUCTUS[ductusKey("kannada", "ಝ")];
const kannadaJhaOutline = kannadaOutline("ಝ");
const KANNADA_THA = DUCTUS[ductusKey("kannada", "ಥ")];
const kannadaThaOutline = kannadaOutline("ಥ");
const KANNADA_MA = DUCTUS[ductusKey("kannada", "ಮ")];
const kannadaMaOutline = kannadaOutline("ಮ");
const KANNADA_LA = DUCTUS[ductusKey("kannada", "ಲ")];
const kannadaLaOutline = kannadaOutline("ಲ");
const KANNADA_VA = DUCTUS[ductusKey("kannada", "ವ")];
const kannadaVaOutline = kannadaOutline("ವ");
const KANNADA_JA = DUCTUS[ductusKey("kannada", "ಜ")];
const kannadaJaOutline = kannadaOutline("ಜ");
const KANNADA_TTA = DUCTUS[ductusKey("kannada", "ಟ")];
const kannadaTtaOutline = kannadaOutline("ಟ");
const KANNADA_NNA = DUCTUS[ductusKey("kannada", "ಣ")];
const kannadaNnaOutline = kannadaOutline("ಣ");
const KANNADA_SHA = DUCTUS[ductusKey("kannada", "ಶ")];
const kannadaShaOutline = kannadaOutline("ಶ");
const KANNADA_SSA = DUCTUS[ductusKey("kannada", "ಷ")];
const kannadaSsaOutline = kannadaOutline("ಷ");
const KANNADA_DHA = DUCTUS[ductusKey("kannada", "ಧ")];
const kannadaDhaOutline = kannadaOutline("ಧ");
const KANNADA_BHA = DUCTUS[ductusKey("kannada", "ಭ")];
const kannadaBhaOutline = kannadaOutline("ಭ");
const KANNADA_PHA = DUCTUS[ductusKey("kannada", "ಫ")];
const kannadaPhaOutline = kannadaOutline("ಫ");
const KANNADA_KHA = DUCTUS[ductusKey("kannada", "ಖ")];
const kannadaKhaOutline = kannadaOutline("ಖ");
const KANNADA_GHA = DUCTUS[ductusKey("kannada", "ಘ")];
const kannadaGhaOutline = kannadaOutline("ಘ");
const KANNADA_DDHA = DUCTUS[ductusKey("kannada", "ಢ")];
const kannadaDdhaOutline = kannadaOutline("ಢ");

describe("Kannada ಅ — four movements in one unbroken run", () => {
  const steps = ductusSteps(KANNADA_A);
  const strip = ductusFilmstrip(KANNADA_A, kannadaAOutline);

  it("never inserts a pen lift between the four movements", () => {
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
  });

  it("reports one stroke, zero lifts, and four movements", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಆ — two joined pairs separated by one lift", () => {
  const steps = ductusSteps(KANNADA_AA);
  const strip = ductusFilmstrip(KANNADA_AA, kannadaAaOutline);

  it("starts the rounded right loop only after the lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports four movements across two strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });
});

describe("Kannada ಇ — one retraced four-movement run", () => {
  const steps = ductusSteps(KANNADA_I);
  const strip = ductusFilmstrip(KANNADA_I, kannadaIOutline);

  it("keeps every movement in the same pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports four movements without a lift", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಈ — body and crossbar separated by one lift", () => {
  const steps = ductusSteps(KANNADA_LONG_I);
  const strip = ductusFilmstrip(KANNADA_LONG_I, kannadaLongIOutline);

  it("starts the crossbar only after the lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports four movements across two strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });
});

describe("Kannada ಉ — one loop-to-terminal run", () => {
  const steps = ductusSteps(KANNADA_U);
  const strip = ductusFilmstrip(KANNADA_U, kannadaUOutline);

  it("keeps all four movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports a four-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಊ — one spiral-through-two-arches run", () => {
  const steps = ductusSteps(KANNADA_UU);
  const strip = ductusFilmstrip(KANNADA_UU, kannadaUuOutline);

  it("keeps all four movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports a four-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಎ — one loop-to-arch run", () => {
  const steps = ductusSteps(KANNADA_E);
  const strip = ductusFilmstrip(KANNADA_E, kannadaEOutline);

  it("keeps all four movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports a four-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಏ — joined body, then the small upper loop", () => {
  const steps = ductusSteps(KANNADA_EE);
  const strip = ductusFilmstrip(KANNADA_EE, kannadaEeOutline);

  it("places one lift before the small upper loop", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
    ]);
  });

  it("reports four movements in two strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });
});

describe("Kannada ಒ — one upper-loop-to-terminal run", () => {
  const steps = ductusSteps(KANNADA_O);
  const strip = ductusFilmstrip(KANNADA_O, kannadaOOutline);

  it("keeps all four movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports a four-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಓ — joined body, then the upper flourish", () => {
  const steps = ductusSteps(KANNADA_OO);
  const strip = ductusFilmstrip(KANNADA_OO, kannadaOoOutline);

  it("places one lift before the small upper flourish", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
  });

  it("reports five movements in two strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Kannada ಐ — one spiral-to-returning-arch run", () => {
  const steps = ductusSteps(KANNADA_AI);
  const strip = ductusFilmstrip(KANNADA_AI, kannadaAiOutline);

  it("keeps all three movements in one pen-down run", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
  });

  it("reports a three-frame zero-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });
});

describe("Kannada ಋ — three source-attested pen-down runs", () => {
  const steps = ductusSteps(KANNADA_VOCALIC_R);
  const strip = ductusFilmstrip(KANNADA_VOCALIC_R, kannadaVocalicROutline);

  it("starts the high hook and right bowl after separate lifts", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
      false,
    ]);
  });

  it("reports a seven-frame, two-lift filmstrip", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 7 movements");
  });
});

describe("Kannada ಃ — two closed loops separated by one lift", () => {
  const steps = ductusSteps(KANNADA_VISARGA);
  const strip = ductusFilmstrip(KANNADA_VISARGA, kannadaVisargaOutline);

  it("starts the lower dot only after the lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([false, true]);
  });

  it("reports two movements across two strokes", () => {
    expect(strip.frames).toHaveLength(2);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 2 movements");
  });
});

describe("Kannada ನ — tail-to-bar body, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_NA);
  const strip = ductusFilmstrip(KANNADA_NA, kannadaNaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 5 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Kannada ತ — bowl and inner loop, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_TA);
  const strip = ductusFilmstrip(KANNADA_TA, kannadaTaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 5 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Kannada ದ — pointed bowl, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_DA);
  const strip = ductusFilmstrip(KANNADA_DA, kannadaDaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 5 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Kannada ರ — round bowl, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_RA);
  const strip = ductusFilmstrip(KANNADA_RA, kannadaRaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 4 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });
});

describe("Kannada ಕ — bowl, two bars, and the link between them", () => {
  const steps = ductusSteps(KANNADA_KA);
  const strip = ductusFilmstrip(KANNADA_KA, kannadaKaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 2, 3, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      true,
      true,
      false,
    ]);
  });

  it("reports 6 movements across 4 strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 6 movements");
  });
});

describe("Kannada ಗ — legs and arch, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_GA);
  const strip = ductusFilmstrip(KANNADA_GA, kannadaGaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 4 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 4 movements");
  });
});

describe("Kannada ಬ — head, lobes, and tall right side in one run", () => {
  const steps = ductusSteps(KANNADA_BA);
  const strip = ductusFilmstrip(KANNADA_BA, kannadaBaOutline);

  it("never inserts a pen lift between the 4 movements", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports one stroke, zero lifts, and 4 movements", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಳ — both loops and the right bowl, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_LLA);
  const strip = ductusFilmstrip(KANNADA_LLA, kannadaLlaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 6 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });
});

describe("Kannada ಯ — bowl, then the middle arm into the hooked bar, then the small right bowl", () => {
  const steps = ductusSteps(KANNADA_YA);
  const strip = ductusFilmstrip(KANNADA_YA, kannadaYaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 1, 1, 2, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 8 movements across 3 strokes", () => {
    expect(strip.frames).toHaveLength(8);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 8 movements");
  });
});

describe("Kannada ಡ — pointed bowl with its inner loop, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_DDA);
  const strip = ductusFilmstrip(KANNADA_DDA, kannadaDdaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 6 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });
});

describe("Kannada ಹ — both rings and the neck, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_HA);
  const strip = ductusFilmstrip(KANNADA_HA, kannadaHaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 6 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 6 movements");
  });
});

describe("Kannada ಸ — body, hooked bar, and dot", () => {
  const steps = ductusSteps(KANNADA_SA);
  const strip = ductusFilmstrip(KANNADA_SA, kannadaSaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
    ]);
  });

  it("reports 6 movements across 3 strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });
});

describe("Kannada ಚ — ಬ's body through the lower bar and link, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_CA);
  const strip = ductusFilmstrip(KANNADA_CA, kannadaCaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 10 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 10 movements");
  });
});

describe("Kannada ಪ — body, dot, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_PA);
  const strip = ductusFilmstrip(KANNADA_PA, kannadaPaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 2, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      true,
      false,
    ]);
  });

  it("reports 6 movements across 3 strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });
});

describe("Kannada ಝ — bowl, hooked bar, two arms, then the tail", () => {
  const steps = ductusSteps(KANNADA_JHA);
  const strip = ductusFilmstrip(KANNADA_JHA, kannadaJhaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 1, 1, 2, 2, 3, 3, 4]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      true,
      false,
      true,
      false,
      true,
      false,
      true,
    ]);
  });

  it("reports 9 movements across 5 strokes", () => {
    expect(strip.frames).toHaveLength(9);
    expect(strip.penLifts).toBe(4);
    expect(strip.summary).toBe("5 strokes · 4 pen lifts · 9 movements");
  });
});

describe("Kannada ಥ — pointed bowl, hooked bar, tail, then the dot", () => {
  const steps = ductusSteps(KANNADA_THA);
  const strip = ductusFilmstrip(KANNADA_THA, kannadaThaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
      true,
    ]);
  });

  it("reports 7 movements across 4 strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });
});

describe("Kannada ಮ — body, hooked bar, then the right bowl", () => {
  const steps = ductusSteps(KANNADA_MA);
  const strip = ductusFilmstrip(KANNADA_MA, kannadaMaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
      false,
    ]);
  });

  it("reports 7 movements across 3 strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 7 movements");
  });
});

describe("Kannada ಲ — loop, base, and right side in one run", () => {
  const steps = ductusSteps(KANNADA_LA);
  const strip = ductusFilmstrip(KANNADA_LA, kannadaLaOutline);

  it("never inserts a pen lift between the 3 movements", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
    ]);
  });

  it("reports one stroke, zero lifts, and 3 movements", () => {
    expect(strip.frames).toHaveLength(3);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 3 movements");
  });
});

describe("Kannada ವ — curl, base, and right side, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_VA);
  const strip = ductusFilmstrip(KANNADA_VA, kannadaVaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 5 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Kannada ಜ — head, lobes, and right side, then the upper arc", () => {
  const steps = ductusSteps(KANNADA_JA);
  const strip = ductusFilmstrip(KANNADA_JA, kannadaJaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
    ]);
  });

  it("reports 5 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Kannada ಟ — top curl, loop, lobes, and upper bowl in one run", () => {
  const steps = ductusSteps(KANNADA_TTA);
  const strip = ductusFilmstrip(KANNADA_TTA, kannadaTtaOutline);

  it("never inserts a pen lift between the 6 movements", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports one stroke, zero lifts, and 6 movements", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 6 movements");
  });
});

describe("Kannada ಣ — both curls, arch, and spiral in one run", () => {
  const steps = ductusSteps(KANNADA_NNA);
  const strip = ductusFilmstrip(KANNADA_NNA, kannadaNnaOutline);

  it("never inserts a pen lift between the 4 movements", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports one stroke, zero lifts, and 4 movements", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಶ — head, sweep, and right side, then the hooked bar", () => {
  const steps = ductusSteps(KANNADA_SHA);
  const strip = ductusFilmstrip(KANNADA_SHA, kannadaShaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
    ]);
  });

  it("reports 5 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(5);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 5 movements");
  });
});

describe("Kannada ಷ — body, dot, hooked bar, then the slanting stroke", () => {
  const steps = ductusSteps(KANNADA_SSA);
  const strip = ductusFilmstrip(KANNADA_SSA, kannadaSsaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 2, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      true,
      false,
      true,
    ]);
  });

  it("reports 7 movements across 4 strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });
});

describe("Kannada ಧ — ದ's bowl, hooked bar, then the tail", () => {
  const steps = ductusSteps(KANNADA_DHA);
  const strip = ductusFilmstrip(KANNADA_DHA, kannadaDhaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      false,
      true,
    ]);
  });

  it("reports 6 movements across 3 strokes", () => {
    expect(strip.frames).toHaveLength(6);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 6 movements");
  });
});

describe("Kannada ಭ — ಬ's body run back along the bar, then the tail", () => {
  const steps = ductusSteps(KANNADA_BHA);
  const strip = ductusFilmstrip(KANNADA_BHA, kannadaBhaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0, 1]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
  });

  it("reports 7 movements across 2 strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(1);
    expect(strip.summary).toBe("2 strokes · 1 pen lift · 7 movements");
  });
});

describe("Kannada ಫ — body, dot, hooked bar, then the tail", () => {
  const steps = ductusSteps(KANNADA_PHA);
  const strip = ductusFilmstrip(KANNADA_PHA, kannadaPhaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 1, 2, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      true,
      true,
      false,
      true,
    ]);
  });

  it("reports 7 movements across 4 strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 7 movements");
  });
});

describe("Kannada ಖ — curl, lower loop, and right side in one run", () => {
  const steps = ductusSteps(KANNADA_KHA);
  const strip = ductusFilmstrip(KANNADA_KHA, kannadaKhaOutline);

  it("never inserts a pen lift between the 4 movements", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
    ]);
  });

  it("reports one stroke, zero lifts, and 4 movements", () => {
    expect(strip.frames).toHaveLength(4);
    expect(strip.penLifts).toBe(0);
    expect(strip.summary).toBe("one unbroken stroke · 4 movements");
  });
});

describe("Kannada ಘ — body and both arms, hooked bar, dot, then the tail", () => {
  const steps = ductusSteps(KANNADA_GHA);
  const strip = ductusFilmstrip(KANNADA_GHA, kannadaGhaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 0, 0, 1, 1, 2, 3]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      true,
      false,
      true,
      true,
    ]);
  });

  it("reports 10 movements across 4 strokes", () => {
    expect(strip.frames).toHaveLength(10);
    expect(strip.penLifts).toBe(3);
    expect(strip.summary).toBe("4 strokes · 3 pen lifts · 10 movements");
  });
});

describe("Kannada ಢ — ಡ's bowl, hooked bar, then the tail", () => {
  const steps = ductusSteps(KANNADA_DDHA);
  const strip = ductusFilmstrip(KANNADA_DDHA, kannadaDdhaOutline);

  it("starts each new run only after a lift", () => {
    expect(steps.map((step) => step.strokeIndex)).toEqual([0, 0, 0, 0, 1, 1, 2]);
    expect(steps.map((step) => step.startsAfterLift)).toEqual([
      false,
      false,
      false,
      false,
      true,
      false,
      true,
    ]);
  });

  it("reports 7 movements across 3 strokes", () => {
    expect(strip.frames).toHaveLength(7);
    expect(strip.penLifts).toBe(2);
    expect(strip.summary).toBe("3 strokes · 2 pen lifts · 7 movements");
  });
});
