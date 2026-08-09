// A deliberately small Markdown-to-view-model parser for authored lesson bodies.
// We preserve the source in `Lesson.body`; this layer only extracts readable
// sections for the DOM without injecting HTML.

export interface LessonSection {
  title: string;
  blocks: LessonViewBlock[];
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
