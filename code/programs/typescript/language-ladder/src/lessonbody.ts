// A deliberately small Markdown-to-view-model parser for authored lesson bodies.
// We preserve the source in `Lesson.body`; this layer only extracts readable
// sections for the DOM without injecting HTML.

export interface LessonSection {
  title: string;
  blocks: LessonViewBlock[];
  /**
   * The section's writing stage, from its `<!-- hl-writing-stage: … -->`
   * directive (HL19), when it declares one. The directive itself is metadata,
   * never learner copy.
   */
  writingStage?: string;
}

export type LessonViewBlock =
  | { kind: "text"; text: string }
  | { kind: "image"; alt: string; source: string };

function plainInline(markdown: string): string {
  return markdown
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[*_`]/g, "")
    .replace(/^>\s?/, "")
    .trim();
}

export function lessonSections(markdown: string): LessonSection[] {
  const sections: LessonSection[] = [];
  let current: LessonSection = { title: "Lesson", blocks: [] };
  let paragraph: string[] = [];

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    current.blocks.push({ kind: "text", text: plainInline(paragraph.join(" ")) });
    paragraph = [];
  };
  const flushSection = () => {
    flushParagraph();
    if (current.blocks.length > 0) sections.push(current);
  };

  for (const raw of markdown.replace(/\r\n/g, "\n").split("\n")) {
    const line = raw.trim();
    if (/^#\s+/.test(line)) continue; // card already displays the lesson title
    if (/^<!--\s*hl-(?:knowledge|activity):/.test(line)) continue; // canonical AST metadata, not learner copy
    const stage = /^<!--\s*hl-writing-stage:\s*([a-z][a-z0-9-]*)\s*-->$/.exec(line);
    if (stage) {
      current.writingStage = stage[1]!;
      continue;
    }
    if (/^<!--\s*hl-writing-stage:/.test(line)) continue; // malformed: the validator reports it
    const heading = /^##\s+(.+)$/.exec(line);
    const image = /^!\[([^\]]+)\]\(([^)]+)\)$/.exec(line);
    if (heading) {
      flushSection();
      current = { title: plainInline(heading[1]!), blocks: [] };
    } else if (image) {
      flushParagraph();
      current.blocks.push({ kind: "image", alt: plainInline(image[1]!), source: image[2]! });
    } else if (line === "") {
      flushParagraph();
    } else if (/^[-*]\s+/.test(line)) {
      flushParagraph();
      current.blocks.push({
        kind: "text",
        text: `• ${plainInline(line.replace(/^[-*]\s+/, ""))}`,
      });
    } else {
      paragraph.push(line);
    }
  }
  flushSection();
  return sections;
}

// ---------------------------------------------------------------------------
// Which section a lesson's stroke-order filmstrip goes in
// ---------------------------------------------------------------------------
//
// The same rule as the book (`stripBlockIndex` in human-language-data's
// figure-targets.ts): the first Writing section, else the first Script
// section, else the first section whose writing stage SHOWS the learner a
// model. A dictation, a composition or a timed task never gets a strip, and
// neither does a section with no stage: a strip there would hand over the
// answer the lesson is testing. A test holds this list equal to the book's.

/** Writing stages whose section shows the learner a model of what to write. */
export const MODELLED_WRITING_STAGES: ReadonlySet<string> = new Set([
  "observe-trace",
  "guided-copy",
  "delayed-copy",
]);

/** The index of the section a filmstrip belongs in, or -1 when none fits. */
export function filmstripSectionIndex(sections: readonly LessonSection[]): number {
  const letter = sections.findIndex((section) => /^(?:Writing|Script)\b/.test(section.title.trim()));
  if (letter !== -1) return letter;
  return sections.findIndex(
    (section) => section.writingStage !== undefined && MODELLED_WRITING_STAGES.has(section.writingStage),
  );
}
