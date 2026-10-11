// Saying a typed prompt and a heading for the ear — `src/for-the-ear.ts`.
//
// The module has two small rewrites, and each is held here case by case: what
// it changes, what it must leave alone, and that it stays linear on adversarial
// input. The corpus-wide gates (no spoken prompt or heading in a drivable part of
// a core-drivable lesson asks for an eye or a hand) are in
// `drivable-instructions.test.ts`, beside the detectors they rely on.

import { describe, expect, it } from "vitest";
import {
  activityPromptForTheEar,
  headingAsksForEyesOrHands,
  headingForTheEar,
} from "../src/for-the-ear.js";
import { asksToType } from "../src/drivable-instructions.js";

describe("activityPromptForTheEar: the typing verb becomes the speaking verb", () => {
  it.each([
    ["Type the Japanese for 'a stick, a pole'.", "Say the Japanese for 'a stick, a pole'."],
    ["Type the Persian word for 'tea'.", "Say the Persian word for 'tea'."],
    ["You hear nahī̃. Type yes or no.", "You hear nahī̃. Say yes or no."],
    [
      "After someone asks اسم شما چیست؟, type the careful Persian answer using Sara.",
      "After someone asks اسم شما چیست؟, say the careful Persian answer using Sara.",
    ],
    [
      "From meaning cues only, say my mother and child. Then type the Japanese forms you said.",
      "From meaning cues only, say my mother and child. Then say the Japanese forms you said.",
    ],
    ["TYPE IT.", "SAY IT."],
  ])("%j → %j", (prompt, spoken) => {
    expect(activityPromptForTheEar(prompt)).toBe(spoken);
    expect(asksToType(activityPromptForTheEar(prompt))).toBe(false);
  });

  it.each([
    "Give the word for friend, the old root meaning, and its borrowing type.",
    "Which type of greeting is this?",
    "A typed answer is fine.",
    "Which one points at a day we both already know about: el dia or un dia?",
    // An authored lower-case opening is never re-capitalised.
    "kokonotsu begins with the same two signs as koko, here. What is the relationship?",
  ])("leaves %j alone", (prompt) => {
    expect(activityPromptForTheEar(prompt)).toBe(prompt);
  });
});

describe("activityPromptForTheEar: a typist's spelling instruction goes", () => {
  it.each([
    ["Type the Japanese for 'a dog', in rōmaji.", "Say the Japanese for 'a dog'."],
    ["Ask for tea politely, in rōmaji.", "Ask for tea politely."],
    [
      "Type the Mandarin word, in pinyin with its tone mark, that means 'you' (one person).",
      "Say the Mandarin word that means 'you' (one person).",
    ],
    ["Type the pinyin, with its tone mark, for the number two.", "Say the pinyin for the number two."],
    [
      "A dictionary prints the greeting as nǐ hǎo. Type it in pinyin the way it is actually spoken.",
      "A dictionary prints the greeting as nǐ hǎo. Say it the way it is actually spoken.",
    ],
    [
      "You did not catch what someone just said. In pinyin, what one word gets it repeated?",
      "You did not catch what someone just said. What one word gets it repeated?",
    ],
    ["Which line closes it, in pinyin?", "Which line closes it?"],
    [
      "Say to speak aloud, then type it romanized and the stem it leaves behind.",
      "Say to speak aloud, then say it and the stem it leaves behind.",
    ],
    [
      "Ask how someone is respectfully aloud, then type the romanized words you said.",
      "Ask how someone is respectfully aloud, then say the words you said.",
    ],
    [
      "Someone says dhannavād. Give the gracious Punjabi reply aloud, then type its romanization.",
      "Someone says dhannavād. Give the gracious Punjabi reply aloud, then say it.",
    ],
  ])("%j → %j", (prompt, spoken) => {
    expect(activityPromptForTheEar(prompt)).toBe(spoken);
  });

  it("leaves a question ABOUT the writing alone", () => {
    const prompt = "In the pinyin for 'morning', why does the second syllable have no tone mark?";
    expect(activityPromptForTheEar(prompt)).toBe(prompt);
  });

  it("stays linear on a long run of near-misses", () => {
    // Every pattern is a fixed literal; 40,000 near-misses finish at once. Only
    // the answer is asserted, because timing bounds flake on a loaded runner.
    const prompt = `${"then typ ".repeat(5_000)}in pinyi, romanize`;
    expect(prompt.length).toBeGreaterThan(40_000);
    expect(activityPromptForTheEar(prompt)).toBe(prompt);
  });
});

describe("headingForTheEar: a label that asks keeps the label", () => {
  it.each([
    ["Guided Practice — read, copy, recall", "Guided Practice"],
    ["Guided Practice — write", "Guided Practice"],
    ["Guided Practice — read the seven words", "Guided Practice"],
    ["Guided Practice: Read them back", "Guided Practice"],
    ["Sounds you'll need: Read the cedilla", "Sounds you'll need"],
    ["Practice — sort the evening words, then read the Latin", "Practice"],
    ["Practice — hear, say, read, and write water", "Practice"],
    ["Chapter 1 checkpoint — say it, know the letters, write it", "Chapter 1 checkpoint"],
    ["(el mensaje del vecino) — read it, then answer", "(el mensaje del vecino)"],
    // Heard aloud, "read *wa*" loses its italics and sounds like an instruction.
    ["Grammar Lens — the sign は, read *wa*", "Grammar Lens"],
  ])("%j → %j", (heading, spoken) => {
    expect(headingForTheEar(heading)).toBe(spoken);
    expect(headingAsksForEyesOrHands(spoken)).toBe(false);
  });

  it.each(["Read two, say one", "Hide, then mix the familiar descriptions", "Understand and read return at R4"])(
    "has no label to keep in %j, and says nothing",
    (heading) => {
      expect(headingForTheEar(heading)).toBe("");
    },
  );

  it.each([
    "Guided Practice",
    "You'll want to know: tapēte, tapētis",
    "Guided Practice: point every verb at somebody",
    "Why it's said this way: Register first",
    "Writing is how the page remembers",
  ])("leaves %j alone", (heading) => {
    expect(headingForTheEar(heading)).toBe(heading);
    expect(headingAsksForEyesOrHands(heading)).toBe(false);
  });
});

describe("headingForTheEar: a lesson title's gloss is the word's meaning", () => {
  it("keeps 'write' when it is the gloss of the headword the title names", () => {
    const context = { headword: "ಬರೆ", gloss: "write" };
    expect(headingForTheEar("ಬರೆ (bare) — write", context)).toBe("ಬರೆ (bare) — write");
    expect(headingAsksForEyesOrHands("ಬರೆ (bare) — write", context)).toBe(false);
  });

  it("needs BOTH the headword in the head and the gloss in the tail", () => {
    // The headword is there, but the tail is not the gloss: an instruction.
    const sintesis = {
      headword: "(el mensaje del vecino)",
      gloss: "the payoff — read a message to the building group, then answer it",
    };
    expect(headingForTheEar("(el mensaje del vecino) — read it, then answer", sintesis)).toBe(
      "(el mensaje del vecino)",
    );
    // The tail is the gloss, but the head does not hold the headword.
    expect(headingAsksForEyesOrHands("Practice — write", { headword: "ಬರೆ", gloss: "write" })).toBe(true);
  });

  it("cuts a joined title-and-gloss back to the part before the asking", () => {
    // How the narration opens a lesson whose title has no dash: title — gloss.
    const context = {
      headword: "(una oferta y una despedida)",
      gloss: "the payoff — read a job advert and a leaving note, then write your own answer",
    };
    expect(
      headingForTheEar(`(una oferta y una despedida) — ${context.gloss}`, context),
    ).toBe("(una oferta y una despedida) — the payoff");
  });

  it("stays linear on a long heading of near-misses", () => {
    const heading = `${"Guided Practice — reading, ".repeat(2_000)}done`;
    expect(heading.length).toBeGreaterThan(40_000);
    expect(headingForTheEar(heading)).toBe(heading);
  });

  it("trims a label's trailing joiners in one walk, however long the run before it", () => {
    // The trim used to be `replace(/[\s,;:—–-]+$/u, "")`. An end-anchored run is
    // retried from every start position, so a long run of commas that stops short
    // of the end cost a quadratic scan: about six seconds at 100,000 commas on the
    // machine where this was written, past vitest's five-second budget. 200,000
    // leaves that margin on a faster machine, and only the answers are asserted
    // here: the budget is the bound, because a wall-clock bound measures the runner.
    const run = ",".repeat(200_000);
    expect(headingForTheEar(`Practice${run}x — write`)).toBe(`Practice${run}x`);
    // The same walk still trims what a label really trails.
    expect(headingForTheEar(`Practice${", ;".repeat(70_000)} — write`)).toBe("Practice");
    expect(headingForTheEar("Guided Practice: — read, copy, recall")).toBe("Guided Practice");
  });
});

describe("activityPromptForTheEar keeps the Markdown around the verb", () => {
  it("rewrites a bold typing verb and keeps the bold", () => {
    expect(activityPromptForTheEar("**Type** the word for 'water'.")).toBe("**Say** the word for 'water'.");
  });
});
