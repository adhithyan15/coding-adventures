import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";

it("keeps recurring Malayalam forms on one letter-by-letter romanization", () => {
  const lessons = loadTrackLessons("malayalam");
  const byId = new Map(lessons.map((lesson) => [lesson.realization.lessonId, lesson]));
  const expected = new Map<string, string>([
    ["ML-C01-sari", "śari"],
    ["ML-C02-peru", "pēr"],
    ["ML-C06-dative-ikku", "-ikkŭ"],
    ["ML-C06-dative-subject", "enikkŭ malayāḷaṁ aṟiyāṁ"],
    ["ML-C10-azhcha", "tiṅkaḷ covva budhan vyāzham veḷḷi śani ñāyar"],
    ["ML-C13-shareera-bhaagangal", "tala kai"],
    ["ML-C17-paathira", "pātirā"],
    ["ML-C18-mani", "maṇi"],
    ["ML-C38-nenchu", "neñcŭ"],
    ["ML-C39-chaaya", "cāya"],
    ["ML-C39-kaapi", "kāppi"],
    ["ML-C39-paal", "pāl"],
    ["ML-C81-dooram", "etra dūraṁ"],
    ["ML-C89-nallathu", "nallatŭ"],
  ]);

  for (const [lessonId, romanization] of expected) {
    expect(byId.get(lessonId)?.realization.romanization, lessonId).toBe(romanization);
  }

  // These are the three concrete collisions that exposed the drift: the same
  // dative ending, the same hour word, and the same weekday phrase used once
  // early and once late. Pin the body sites too, not only their owner fields.
  expect(byId.get("ML-C06-dative-ikku")?.body).toContain("*jōli**kkŭ***");
  expect(byId.get("ML-C18-mani")?.body).toContain("**മണി** (*maṇi*)");
  expect(byId.get("ML-C102-azhcha")?.body).toContain("*tiṅkaḷāḻca ñān pōkuṁ*");
});
