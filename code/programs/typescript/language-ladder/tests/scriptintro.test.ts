import { describe, it, expect } from "vitest";
import {
  scriptOf,
  scriptsById,
  firstIntroductionByScript,
  scriptIntroFor,
  LANGUAGE_SCRIPT,
} from "../src/scriptintro";
import type { ScriptData } from "../src/types";

function scriptData(script: string, name: string, signature?: string): ScriptData {
  return {
    script,
    name,
    font: "",
    direction: "ltr",
    system: "abugida",
    signature,
    letters: [],
  };
}

describe("scriptOf / LANGUAGE_SCRIPT", () => {
  it("maps chain languages to their writing systems, Latin languages to 'latin'", () => {
    expect(scriptOf("hindi")).toBe("devanagari");
    expect(scriptOf("arabic")).toBe("arabic");
    expect(scriptOf("tamil")).toBe("tamil");
    expect(scriptOf("spanish")).toBe("latin");
    expect(scriptOf("german")).toBe("latin");
  });

  it("an unknown / off-chain language is treated as Latin (no intro)", () => {
    expect(scriptOf("klingon")).toBe("latin");
  });

  it("every chain language has a mapping", () => {
    // all ten chain entries present
    expect(Object.keys(LANGUAGE_SCRIPT).length).toBe(10);
  });
});

describe("firstIntroductionByScript — first occurrence in book order", () => {
  // spine in book order; the same script recurs across concepts.
  const spine = ["THANKS", "HELLO", "NAME"];
  const lessons = [
    { concept: "THANKS", language: "spanish" }, // latin — ignored
    { concept: "THANKS", language: "hindi" }, // devanagari FIRST here
    { concept: "THANKS", language: "arabic" }, // arabic FIRST here
    { concept: "HELLO", language: "hindi" }, // devanagari again — NOT an intro
    { concept: "NAME", language: "tamil" }, // tamil FIRST here (but no data below)
  ];
  const available = new Set(["devanagari", "arabic", "tamil"]);

  it("returns the earliest concept for each non-Latin script we have data for", () => {
    const introAt = firstIntroductionByScript(spine, lessons, available);
    expect(introAt.get("devanagari")).toBe("THANKS");
    expect(introAt.get("arabic")).toBe("THANKS");
    expect(introAt.get("tamil")).toBe("NAME");
    expect(introAt.has("latin")).toBe(false); // base script never introduced
  });

  it("CONTROL: the SECOND appearance of a script is not its intro concept", () => {
    const introAt = firstIntroductionByScript(spine, lessons, available);
    // Devanagari appears in THANKS (index 0) and HELLO (index 1); the intro is
    // THANKS. If the helper tracked the last occurrence instead of the first,
    // this would be "HELLO" and fail.
    expect(introAt.get("devanagari")).not.toBe("HELLO");
    expect(introAt.get("devanagari")).toBe("THANKS");
  });

  it("omits scripts with no data — we never fabricate a note", () => {
    const introAt = firstIntroductionByScript(spine, lessons, new Set(["arabic"]));
    expect(introAt.has("devanagari")).toBe(false); // not in `available`
    expect(introAt.has("tamil")).toBe(false);
    expect(introAt.get("arabic")).toBe("THANKS");
  });

  it("ignores lessons whose concept is off the spine (e.g. writing lessons)", () => {
    const withWriting = [...lessons, { concept: "", language: "tamil" }];
    const introAt = firstIntroductionByScript(spine, withWriting, available);
    expect(introAt.get("tamil")).toBe("NAME"); // the "" concept contributes nothing
  });
});

describe("scriptIntroFor — the note for one step", () => {
  const introAt = new Map([
    ["devanagari", "THANKS"],
    ["arabic", "THANKS"],
  ]);
  const byId = new Map([
    ["devanagari", scriptData("devanagari", "Devanagari", "A head-line runs across the top.")],
    ["arabic", scriptData("arabic", "Arabic", "Right-to-left cursive with dots.")],
  ]);

  it("returns the note exactly at the introducing concept", () => {
    expect(scriptIntroFor("THANKS", "hindi", introAt, byId)).toEqual({
      name: "Devanagari",
      system: "abugida",
      signature: "A head-line runs across the top.",
    });
  });

  it("returns null at a LATER concept (the script is old news by then)", () => {
    expect(scriptIntroFor("HELLO", "hindi", introAt, byId)).toBeNull();
  });

  it("returns null for a Latin-script language", () => {
    expect(scriptIntroFor("THANKS", "spanish", introAt, byId)).toBeNull();
  });

  it("returns null when there is no data for the script, even if introAt names it", () => {
    const introAtTamil = new Map([["tamil", "THANKS"]]);
    expect(scriptIntroFor("THANKS", "tamil", introAtTamil, byId)).toBeNull();
  });

  it("tolerates a missing signature (empty string, not undefined)", () => {
    const byIdNoSig = new Map([["devanagari", scriptData("devanagari", "Devanagari")]]);
    expect(scriptIntroFor("THANKS", "hindi", introAt, byIdNoSig)).toEqual({
      name: "Devanagari",
      system: "abugida",
      signature: "",
    });
  });
});

describe("scriptsById", () => {
  it("indexes script data by its script id", () => {
    const byId = scriptsById([scriptData("arabic", "Arabic"), scriptData("tamil", "Tamil")]);
    expect(byId.get("arabic")?.name).toBe("Arabic");
    expect(byId.get("tamil")?.name).toBe("Tamil");
    expect(byId.get("missing")).toBeUndefined();
  });
});
