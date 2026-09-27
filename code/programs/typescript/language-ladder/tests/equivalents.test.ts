import { describe, expect, it } from "vitest";
import { equivalentsView } from "../src/equivalents.ts";
import { equivalentsPanel, loadEquivalents, renderEquivalentsPanel } from "../src/equivalents-sources.ts";

const NAMES: Record<string, string> = { kannada: "Kannada", telugu: "Telugu", hindi: "Hindi" };
const nameOf = (id: string) => NAMES[id] ?? id;

const EYE = {
  lesson: "TA-TEST-kan",
  english: "an eye",
  equivalents: [
    { language: "kannada", form: "ಕಣ್ಣು", romanization: "kaṇṇu", sameRoot: true },
    { language: "telugu", form: "కన్ను", romanization: "kannu", sameRoot: true },
    { language: "hindi", form: "आँख", romanization: "ā̃kh" },
  ],
  source: "test",
};

describe("equivalentsView", () => {
  it("lists the family, labels the neighbour, and puts English last", () => {
    const view = equivalentsView(EYE, ["hindi"], nameOf);
    expect(view).not.toBeNull();
    expect(view!.marked).toBe(true);
    expect(view!.rows.map((row) => row.label)).toEqual([
      "Kannada",
      "Telugu",
      "Hindi (neighbour)",
      "English",
    ]);
    expect(view!.rows[0]).toEqual({ label: "Kannada", form: "ಕಣ್ಣು", romanization: "kaṇṇu", sameRoot: true });
    expect(view!.rows[2]!.sameRoot).toBe(false);
    expect(view!.rows[3]).toEqual({ label: "English", form: "an eye", romanization: "", sameRoot: false });
  });

  it("leaves the same-root column off when nothing is marked", () => {
    const plain = { ...EYE, equivalents: EYE.equivalents.map(({ sameRoot: _unused, ...rest }) => rest) };
    expect(equivalentsView(plain, ["hindi"], nameOf)!.marked).toBe(false);
  });

  it("falls back to the language id when the registry has no name", () => {
    const other = { ...EYE, equivalents: [{ language: "tulu", form: "ಕಣ್ಣ್", romanization: "kaṇṇŭ" }] };
    expect(equivalentsView(other, [], nameOf)!.rows[0]!.label).toBe("tulu");
  });

  it("shows no panel at all, rather than half of one, for a malformed owner file", () => {
    const broken: unknown[] = [
      null,
      "an eye",
      { ...EYE, english: "" },
      { ...EYE, english: 3 },
      { ...EYE, equivalents: [] },
      { ...EYE, equivalents: "ಕಣ್ಣು" },
      { ...EYE, equivalents: [null] },
      { ...EYE, equivalents: [{ language: "kannada", form: "" }] },
      { ...EYE, equivalents: [{ language: 7, form: "ಕಣ್ಣು" }] },
      { ...EYE, equivalents: [{ language: "kannada", form: "ಕಣ್ಣು", romanization: 1 }] },
      { ...EYE, equivalents: [{ language: "kannada", form: "ಕಣ್ಣು", sameRoot: "yes" }] },
      { ...EYE, equivalents: [...EYE.equivalents, { language: "telugu" }] },
    ];
    for (const raw of broken) expect(equivalentsView(raw, ["hindi"], nameOf)).toBeNull();
  });
});

describe("loadEquivalents", () => {
  it("loads a committed Tamil owner file with Tamil's neighbour list", async () => {
    const loaded = await loadEquivalents("tamil", "TA-C01-aam");
    expect(loaded).not.toBeNull();
    expect(loaded!.neighbours).toEqual(["hindi"]);
    const view = equivalentsView(loaded!.raw, loaded!.neighbours, nameOf);
    expect(view!.rows.at(-1)).toMatchObject({ label: "English", form: "yes" });
    expect(view!.rows.map((row) => row.label)).toContain("Hindi (neighbour)");
  });

  it("returns null for a lesson with no owner file", async () => {
    expect(await loadEquivalents("tamil", "TA-NO-SUCH-LESSON")).toBeNull();
    expect(await loadEquivalents("spanish", "ES-C01-hola")).toBeNull();
  });

  it("refuses ids that could reach outside the owner directory", async () => {
    expect(await loadEquivalents("../tamil", "TA-C01-aam")).toBeNull();
    expect(await loadEquivalents("tamil", "../TA-C01-aam")).toBeNull();
    expect(await loadEquivalents("Tamil", "TA-C01-aam")).toBeNull();
  });
});

describe("renderEquivalentsPanel", () => {
  it("builds a table of text nodes, one row per language and English last", () => {
    const panel = renderEquivalentsPanel(equivalentsView(EYE, ["hindi"], nameOf)!);
    expect(panel.querySelector("figcaption")!.textContent).toBe("In the family, and next door");
    const rows = [...panel.querySelectorAll("tr")];
    expect(rows.map((row) => row.querySelector("th")!.textContent)).toEqual([
      "Kannada",
      "Telugu",
      "Hindi (neighbour)",
      "English",
    ]);
    expect(rows[0]!.querySelector(".lesson-body__equivalents-said")!.textContent).toBe("kaṇṇu");
    expect(rows[0]!.querySelector(".lesson-body__equivalents-root")!.textContent).toBe("same root");
    expect(rows[3]!.querySelector(".lesson-body__equivalents-said")).toBeNull();
  });

  it("drops the same-root column when nothing is marked", () => {
    const plain = { ...EYE, equivalents: [{ language: "kannada", form: "ಕಣ್ಣು", romanization: "kaṇṇu" }] };
    const panel = renderEquivalentsPanel(equivalentsView(plain, [], nameOf)!);
    expect(panel.querySelector(".lesson-body__equivalents-root")).toBeNull();
  });

  it("prints markup in a form as text, never as HTML", () => {
    const hostile = { ...EYE, equivalents: [{ language: "kannada", form: "<img src=x onerror=alert(1)>" }] };
    const panel = renderEquivalentsPanel(equivalentsView(hostile, [], nameOf)!);
    expect(panel.querySelector("img")).toBeNull();
    expect(panel.querySelector(".lesson-body__equivalents-form")!.textContent).toBe("<img src=x onerror=alert(1)>");
  });

  it("builds the committed Tamil panel end to end, and nothing for a lesson without one", async () => {
    const panel = await equivalentsPanel("tamil", "TA-C01-aam", nameOf);
    expect(panel!.querySelectorAll("tr").length).toBeGreaterThanOrEqual(2);
    expect(await equivalentsPanel("tamil", "TA-NO-SUCH-LESSON", nameOf)).toBeNull();
  });
});
