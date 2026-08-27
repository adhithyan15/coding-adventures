// Exact real-corpus evidence owned by the Perso-Arabic inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Perso-Arabic",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const persianDal = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "د",
    )!;
    expect(persianDal.strokeOrder).toEqual([
      "begin at the upper tip and descend through the folded shoulder",
      "without lifting, turn left along the baseline",
    ]);
    expect(persianDal.penLifts).toBe(0);
    expect(persianDal.strokeOrderSource?.url).toBe(
      "https://laits.utexas.edu/persian_grammar/video/gr/kooroshalphabet",
    );
    expect(persianDal.strokeOrderSource?.citation).toMatch(
      /Persian Online.*د.*01:04–01:06/i,
    );
    expect(persianDal.strokeOrderSource?.variation).toMatch(
      /continuous Naskh.*upper tip.*shoulder.*baseline.*without lifting.*non-connector.*Persian-scoped/i,
    );
    const persianPeh = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "پ",
    )!;
    expect(persianPeh.strokeOrder).toEqual([
      "sweep the shallow bowl from right to left",
      "lift, then place the left dot below",
      "lift again and place the right dot below",
      "lift again and place the lower-center dot",
    ]);
    expect(persianPeh.penLifts).toBe(3);
    expect(persianPeh.strokeOrderSource?.url).toBe(
      "https://laits.utexas.edu/persian_grammar/video/gr/kooroshalphabet",
    );
    expect(persianPeh.strokeOrderSource?.citation).toMatch(
      /Persian Online.*پ.*00:16–00:21/i,
    );
    expect(persianPeh.strokeOrderSource?.variation).toMatch(
      /right-to-left.*three separate dots below.*left, right, then lower-center.*Noto Naskh/i,
    );
    const persianFeh = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "ف",
    )!;
    expect(persianFeh.strokeOrder).toEqual([
      "loop clockwise around the small closed head",
      "continue left through the broad bowl without lifting",
      "lift once, then place the upper dot",
    ]);
    expect(persianFeh.penLifts).toBe(1);
    expect(persianFeh.strokeOrderSource?.citation).toMatch(
      /Persian Online.*ف.*02:09–02:13/i,
    );
    expect(persianFeh.strokeOrderSource?.variation).toMatch(
      /body-first.*clockwise.*closed head.*broad bowl.*lift once.*dot.*Persian-scoped/i,
    );
    const persianQaf = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "ق",
    )!;
    expect(persianQaf.penLifts).toBe(2);
    expect(persianQaf.strokeOrder).toEqual([
      "loop counterclockwise around the small closed head",
      "continue down and left through the deep bowl without lifting",
      "lift once, then place the upper-right dot",
      "lift again, then place the upper-left dot",
    ]);
    expect(persianQaf.strokeOrderSource?.citation).toMatch(
      /Persian Online.*ق.*02:14–02:18/i,
    );
    const persianHah = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "ح",
    )!;
    expect(persianHah.sound).toBe("h");
    expect(persianHah.penLifts).toBe(0);
    expect(persianHah.strokeOrder).toEqual([
      "draw the short upper head from left to right",
      "continue down and around the deep bowl without lifting",
    ]);
    expect(persianHah.strokeOrderSource?.citation).toMatch(
      /Persian Online.*ح.*00:42–00:46/i,
    );
    expect(persianHah.strokeOrderSource?.variation).toMatch(
      /body-first.*continuous Naskh.*left to right.*hooked descent.*deep lower bowl.*without lifting.*Persian-scoped/i,
    );
    const persianRa = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "ر",
    )!;
    expect(persianRa.strokeOrder).toEqual([
      "begin at the upper tip and descend through the short stroke",
      "without lifting, sweep left through the lower curve",
    ]);
    expect(persianRa.penLifts).toBe(0);
    expect(persianRa.strokeOrderSource?.url).toBe(
      "https://laits.utexas.edu/persian_grammar/video/gr/kooroshalphabet",
    );
    expect(persianRa.strokeOrderSource?.citation).toMatch(
      /Persian Online.*ر.*01:10–01:12/i,
    );
    expect(persianRa.strokeOrderSource?.variation).toMatch(
      /continuous Naskh.*upper tip.*short stroke.*lower curve.*without lifting.*non-connector.*Persian-scoped/i,
    );
    expect(missingByScript.get("perso-arabic.json")?.has("د")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("ر")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("پ")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("چ")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("ٓ")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("خ")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("ف")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("ح")).toBe(false);
    const persianShin = scripts["perso-arabic"]!.letters.find(
      (entry) => entry.glyph === "ش",
    )!;
    expect(persianShin.sound).toBe("sh");
    expect(persianShin.penLifts).toBe(3);
    expect(persianShin.strokeOrder).toEqual([
      "form the three teeth from right to left",
      "flow into the final bowl without lifting",
      "lift, then place the lower-left dot",
      "lift again and place the lower-right dot",
      "lift again and place the centered upper dot",
    ]);
    expect(persianShin.strokeOrderSource?.citation).toMatch(
      /Persian Online.*ش.*01:29–01:35/i,
    );
    expect(missingByScript.get("perso-arabic.json")?.has("ش")).toBe(false);
    expect(affected.get("ش") ?? 0).toBe(0);
    const persianYeh = scripts["perso-arabic"]!.letters.find(
      (entry) => entry.glyph === "ی",
    )!;
    expect(persianYeh.sound).toBe("y / i");
    expect(persianYeh.penLifts).toBe(0);
    expect(persianYeh.strokeOrder).toEqual([
      "sweep left from the upper right and descend through the S curve",
      "continue around the below-baseline bowl and finish at its rising tip without lifting",
    ]);
    expect(persianYeh.strokeOrderSource?.citation).toMatch(
      /Persian Online.*closing ی.*02:55–02:58/i,
    );
    expect(persianYeh.strokeOrderSource?.variation).toMatch(
      /uninterrupted dotless S-shaped run.*upper right.*below-baseline bowl.*without lifting.*Persian-scoped.*Urdu/i,
    );
    expect(missingByScript.get("perso-arabic.json")?.has("ی")).toBe(false);
    expect(affected.get("ی") ?? 0).toBe(0);
    expect(missingByScript.get("perso-arabic.json")?.has("ظ")).toBe(false);
    expect(missingByScript.get("perso-arabic.json")?.has("ک")).toBe(false);
    expect(affected.get("ک") ?? 0).toBe(0);
    expect(missingByScript.get("perso-arabic.json")?.has("ق")).toBe(false);
  },
};
