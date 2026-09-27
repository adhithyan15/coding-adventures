import type { ParsedLesson } from "../../src/parse.js";

export function lesson(
  id: string,
  options: { language?: string; type?: string; headword?: string; blocks?: string[] } = {},
): ParsedLesson {
  const blocks = (options.blocks ?? ["Warm-up", "Writing: x", "Wrap-up Recall"]).map((title) => ({
    type: "explanation",
    title,
    markdown: `body of ${title}\n`,
  }));
  return {
    language: options.language ?? "tamil",
    realization: {
      lessonId: id,
      type: options.type ?? "writing",
      headword: options.headword ?? "அ",
    },
    blocks,
  } as unknown as ParsedLesson;
}
