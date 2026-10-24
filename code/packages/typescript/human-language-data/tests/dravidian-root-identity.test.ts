import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// DEDR 3103 names the head etymon; DEDR 2023 names the hand etymon.
// A combined lesson needs both slugs, never one slug standing for both entries.
const HEAD = "dravidian-talai-head";
const HAND = "dravidian-kai-hand";
const LEGACY = ["dravidian-tala-kai", "dravidian-talai-kai", "dravidian-tale-kai"];
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../learning/human-languages");

/** Read only the authored roots line; generated books cannot change identity. */
function rootsOf(source: string): string[] {
  const frontmatter = source.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  const roots = frontmatter?.[1].match(/^roots:\s*\[([^\]]*)\]/m);
  return roots?.[1].split(",").map((root) => root.trim()) ?? [];
}

const CASES: Record<string, Record<string, readonly string[]>> = {
  malayalam: {
    "ML-C13-shareera-bhaagangal": [HEAD, HAND],
    "ML-C105-thalasthaanam": [HEAD],
    "ML-C115-dizzy": [HEAD],
    "ML-C115-headache": [HEAD],
    "ML-C154-kaimuttu": [HAND],
    "ML-C158-talayana": [HEAD],
    "ML-C163-kaivella": [HAND],
    "ML-C235-kanankai": [HAND],
    "ML-C237-talennu": [HEAD],
    "ML-C252-kainittam": [HAND],
    "ML-C267-kaimaruka": [HAND],
  },
  tamil: {
    "TA-C13-udal-uruppugal": [HEAD, HAND],
    "TA-C88-headache": [HEAD],
    "TA-C105-signature": [HAND],
    "TA-C119-talaiyanai": [HEAD],
    "TA-C134-kaippeci": [HAND],
    "TA-C137-kaiyurai": [HAND],
    "TA-C142-mulankai": [HAND],
    "TA-C201-ullankai": [HAND],
    "TA-C221-talaikkavacam": [HEAD],
    "TA-C234-talainakaram": [HEAD],
    "TA-C244-ancal-talai": [HEAD],
    "TA-C258-kai-kulukku": [HAND],
    "TA-C261-talaiccurral": [HEAD],
    "TA-C270-kaikkuttai": [HAND],
  },
  kannada: {
    "KA-C13-dehada-bhagagalu": [HEAD, HAND],
  },
};

describe("DEDR 3103 head and DEDR 2023 hand root identities", () => {
  for (const [language, cases] of Object.entries(CASES)) {
    it(`keeps ${language} lesson roots distinct`, () => {
      const lessonsDir = join(ROOT, language, "lessons");
      for (const [id, expected] of Object.entries(cases)) {
        const roots = rootsOf(readFileSync(join(lessonsDir, `${id}.md`), "utf8"));
        expect(roots.filter((root) => root === HEAD || root === HAND), id).toEqual(expected);
      }
      // The old pair-slugs joined unrelated head and hand derivatives. Scan the
      // whole track so a newly authored lesson cannot quietly revive that join.
      const collisions = readdirSync(lessonsDir)
        .filter((file) => file.endsWith(".md"))
        .flatMap((file) => rootsOf(readFileSync(join(lessonsDir, file), "utf8"))
          .filter((root) => LEGACY.includes(root))
          .map((root) => `${file}: ${root}`));
      expect(collisions).toEqual([]);
    });
  }
});
