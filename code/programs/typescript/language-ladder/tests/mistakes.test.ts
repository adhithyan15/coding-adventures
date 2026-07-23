import { describe, it, expect } from "vitest";
import { recordAnswer, demote, confusions, type AnswerRecord } from "../src/mistakes";
import { cellWeight, type QuizState } from "../src/quiz";

describe("recordAnswer", () => {
  it("appends without mutating the previous log", () => {
    const a = recordAnswer([], "k1", true);
    const b = recordAnswer(a, "k2", false, "picked");
    expect(a).toEqual([{ cellKey: "k1", correct: true }]); // a is untouched
    expect(b).toEqual([
      { cellKey: "k1", correct: true },
      { cellKey: "k2", correct: false, chosenKey: "picked" },
    ]);
  });

  it("drops chosenKey for a correct answer (a right answer is not a confusion)", () => {
    // even if a chosenKey is passed on a correct answer, it isn't a confusion.
    expect(recordAnswer([], "k", true, "whatever")).toEqual([{ cellKey: "k", correct: true }]);
  });
});

describe("demote — feeding a miss back into the SRS", () => {
  const session = 20;
  const mastered: QuizState = { box: 5, dueAtSession: 999, lapses: 0, reps: 8 };

  it("a missed cell resurfaces sooner: its draw weight jumps above its mastered weight", () => {
    const before = cellWeight(mastered, session); // mastered, not due → floor
    const after = cellWeight(demote(mastered, session), session); // box 0, due now
    expect(after).toBeGreaterThan(before); // CONTROL: a no-op demote leaves after == before → fails
  });

  it("resets the box, makes it due now, and counts the lapse", () => {
    const d = demote(mastered, session);
    expect(d.box).toBe(0);
    expect(d.dueAtSession).toBe(session); // due at (not after) the current session
    expect(d.lapses).toBe(1);
    expect(d.reps).toBe(9);
  });

  it("does not mutate the input state", () => {
    demote(mastered, session);
    expect(mastered).toEqual({ box: 5, dueAtSession: 999, lapses: 0, reps: 8 });
  });
});

describe("confusions — what the learner keeps mixing up", () => {
  it("ranks the (chosen instead of correct) pairs by frequency", () => {
    let log: AnswerRecord[] = [];
    log = recordAnswer(log, "es:gato", false, "fr:chat"); // mixed es cat with fr cat
    log = recordAnswer(log, "es:gato", false, "fr:chat"); // again
    log = recordAnswer(log, "de:hund", false, "en:hound");
    log = recordAnswer(log, "es:gato", true); // a correct answer — ignored
    const c = confusions(log);
    expect(c).toEqual([
      { correct: "es:gato", chosen: "fr:chat", count: 2 },
      { correct: "de:hund", chosen: "en:hound", count: 1 },
    ]);
  });

  it("CONTROL: only surfaces pairs actually recorded wrong — nothing invented", () => {
    let log: AnswerRecord[] = [];
    log = recordAnswer(log, "a", true); // correct — no confusion
    log = recordAnswer(log, "b", false); // wrong but no chosenKey — no pair
    const c = confusions(log);
    expect(c).toEqual([]); // a fabricated pair would make this non-empty and fail
  });

  it("separates distinct chosen-answers for the same asked item", () => {
    let log: AnswerRecord[] = [];
    log = recordAnswer(log, "es:gato", false, "fr:chat");
    log = recordAnswer(log, "es:gato", false, "it:gatto");
    const c = confusions(log);
    expect(c.map((x) => x.chosen).sort()).toEqual(["fr:chat", "it:gatto"]);
    expect(c.every((x) => x.count === 1)).toBe(true);
  });

  it("keys are collision-safe even if an id contains the delimiter characters", () => {
    // JSON encoding, not delimiter-joining: these two must stay distinct.
    let log: AnswerRecord[] = [];
    log = recordAnswer(log, "a", false, "b,c");
    log = recordAnswer(log, "a,b", false, "c");
    expect(confusions(log).length).toBe(2); // would collapse to 1 under a naive "a,b,c" join
  });
});
