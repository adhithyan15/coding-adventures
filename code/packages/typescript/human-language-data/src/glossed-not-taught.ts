import type { ParsedLesson } from "./parse.js";

export interface GlossedNotTaughtCandidate {
  token: string;
  occurrences: number;
  lessonIds: string[];
}

export interface GlossedNotTaughtReport {
  language: string;
  script: string;
  distinctScriptTokens: number;
  distinctHeadwordTokens: number;
  candidates: GlossedNotTaughtCandidate[];
}

function unicodeScriptNames(script: string): string[] {
  if (script === "chinese") return ["Han"];
  if (script === "japanese") return ["Han", "Hiragana", "Katakana"];
  if (script === "perso-arabic" || script === "urdu-nastaliq") return ["Arabic"];
  return [
    script
      .split(/[-_]/)
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
      .join("_"),
  ];
}

function scriptTokens(text: string, script: string): string[] {
  const properties = unicodeScriptNames(script);
  const scriptClasses = properties
    .map((property) => `\\p{Script_Extensions=${property}}`)
    .join("");
  let matcher: RegExp;
  try {
    // Marks are included explicitly because many Indic vowel signs have the
    // Unicode Script value Inherited even though they belong to the word.
    matcher = new RegExp(`[${scriptClasses}\\p{M}]+`, "gu");
  } catch {
    throw new Error(`cannot build a Unicode script matcher for '${script}'`);
  }
  const scriptLetter = new RegExp(`(?=\\p{L})[${scriptClasses}]`, "u");
  return (text.normalize("NFC").match(matcher) ?? []).filter((token) =>
    scriptLetter.test(token),
  );
}

/**
 * Narrow the human review queue described by HL-C214: native-script tokens
 * that occur in a track but are never a token in any lesson headword.
 *
 * This deliberately reports rather than rejects. A token can occur in an
 * English gloss, an etymology, or a genuine unannounced translation; only a
 * reader can distinguish those cases.
 */
export function measureGlossedNotTaught(
  lessons: ParsedLesson[],
  language: string,
): GlossedNotTaughtReport {
  const track = lessons.filter((lesson) => lesson.language === language);
  if (track.length === 0) throw new Error(`unknown or lessonless track '${language}'`);

  const scripts = new Set(track.map((lesson) => lesson.script));
  if (scripts.size !== 1) {
    throw new Error(`track '${language}' declares multiple scripts: ${[...scripts].join(", ")}`);
  }
  const script = track[0]!.script;
  const headwords = new Set(
    track.flatMap((lesson) => scriptTokens(lesson.realization.headword, script)),
  );
  const occurrences = new Map<string, { count: number; lessonIds: Set<string> }>();

  for (const lesson of track) {
    const searchable = `${JSON.stringify(lesson.frontmatter)}\n${lesson.body}`;
    for (const token of scriptTokens(searchable, script)) {
      const hit = occurrences.get(token) ?? { count: 0, lessonIds: new Set<string>() };
      hit.count += 1;
      hit.lessonIds.add(lesson.realization.lessonId);
      occurrences.set(token, hit);
    }
  }

  const candidates = [...occurrences.entries()]
    .filter(([token]) => !headwords.has(token))
    .map(([token, hit]) => ({
      token,
      occurrences: hit.count,
      lessonIds: [...hit.lessonIds].sort(),
    }))
    .sort((left, right) => left.token.localeCompare(right.token));

  return {
    language,
    script,
    distinctScriptTokens: occurrences.size,
    distinctHeadwordTokens: headwords.size,
    candidates,
  };
}

export function renderGlossedNotTaught(report: GlossedNotTaughtReport): string[] {
  return [
    `Glossed-but-never-taught candidate report: ${report.language}`,
    `script: ${report.script}`,
    `distinct script tokens: ${report.distinctScriptTokens}`,
    `distinct headword tokens: ${report.distinctHeadwordTokens}`,
    `candidates: ${report.candidates.length}`,
    ...report.candidates.map(
      (candidate) =>
        `${candidate.token}\t${candidate.occurrences}\t${candidate.lessonIds.join(",")}`,
    ),
  ];
}
