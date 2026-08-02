import { canonicalChapterHash } from "./hash.js";
import type { LessonBodyBlock } from "./types.js";
import type { ParsedLesson } from "./parse.js";

export interface BookGenerationTarget {
  language: string;
  chapter: number;
  title: string;
  label: string;
  output: string;
}

export interface GeneratedBookChapter {
  tex: string;
  sourceHash: string;
  lessonIds: string[];
}

function stringValue(value: ParsedLesson["frontmatter"][string] | undefined): string {
  return typeof value === "string" ? value : "";
}

function escapeLatexCharacter(character: string): string {
  const escaped: Record<string, string> = {
    "\\": "\\textbackslash{}",
    "{": "\\{",
    "}": "\\}",
    "%": "\\%",
    "$": "\\$",
    "#": "\\#",
    "_": "\\_",
    "&": "\\&",
    "~": "\\textasciitilde{}",
    "^": "\\textasciicircum{}",
    "[": "{[}",
    "]": "{]}",
    "←": "$\\leftarrow$",
    "→": "$\\to$",
  };
  return escaped[character] ?? character;
}

/** Render the deliberately small inline subset used by schema-v2 lessons. */
export function renderInlineMarkdown(markdown: string): string {
  const output: string[] = [];
  let cursor = 0;
  while (cursor < markdown.length) {
    if (markdown.startsWith("**", cursor)) {
      const end = markdown.indexOf("**", cursor + 2);
      if (end !== -1) {
        output.push(`\\textbf{${renderInlineMarkdown(markdown.slice(cursor + 2, end))}}`);
        cursor = end + 2;
        continue;
      }
    }
    if (markdown[cursor] === "*") {
      const end = markdown.indexOf("*", cursor + 1);
      if (end !== -1) {
        output.push(`\\emph{${renderInlineMarkdown(markdown.slice(cursor + 1, end))}}`);
        cursor = end + 1;
        continue;
      }
    }
    if (markdown[cursor] === "`") {
      const end = markdown.indexOf("`", cursor + 1);
      if (end !== -1) {
        const literal = markdown
          .slice(cursor + 1, end)
          .split("")
          .map(escapeLatexCharacter)
          .join("");
        output.push(`\\texttt{${literal}}`);
        cursor = end + 1;
        continue;
      }
    }
    if (markdown[cursor] === "[") {
      const labelEnd = markdown.indexOf("](", cursor + 1);
      const destinationEnd = labelEnd === -1 ? -1 : markdown.indexOf(")", labelEnd + 2);
      if (labelEnd !== -1 && destinationEnd !== -1) {
        output.push(renderInlineMarkdown(markdown.slice(cursor + 1, labelEnd)));
        cursor = destinationEnd + 1;
        continue;
      }
    }
    output.push(escapeLatexCharacter(markdown[cursor] ?? ""));
    cursor += 1;
  }
  return output.join("");
}

function renderMarkdown(markdown: string): string {
  const output: string[] = [];
  const paragraph: string[] = [];
  const quote: string[] = [];
  let listOpen = false;
  let listItem: string[] = [];

  const flushParagraph = (): void => {
    if (paragraph.length === 0) return;
    output.push(renderInlineMarkdown(paragraph.join(" ")), "");
    paragraph.length = 0;
  };
  const flushQuote = (): void => {
    if (quote.length === 0) return;
    output.push("\\begin{quote}", renderInlineMarkdown(quote.join(" ")), "\\end{quote}", "");
    quote.length = 0;
  };
  const flushListItem = (): void => {
    if (listItem.length === 0) return;
    output.push(`  \\item ${renderInlineMarkdown(listItem.join(" "))}`);
    listItem = [];
  };
  const closeList = (): void => {
    if (!listOpen) return;
    flushListItem();
    output.push("\\end{itemize}", "");
    listOpen = false;
  };

  for (const rawLine of markdown.split(/\r?\n/)) {
    const line = rawLine.trimEnd();
    if (line.trim() === "") {
      flushParagraph();
      flushQuote();
      closeList();
      continue;
    }
    if (line.startsWith("> ")) {
      flushParagraph();
      closeList();
      quote.push(line.slice(2));
      continue;
    }
    if (line.startsWith("- ")) {
      flushParagraph();
      flushQuote();
      if (!listOpen) {
        output.push("\\begin{itemize}");
        listOpen = true;
      }
      flushListItem();
      listItem = [line.slice(2)];
      continue;
    }
    if (listOpen && /^\s+/.test(line)) {
      listItem.push(line.trim());
      continue;
    }
    if (quote.length > 0) flushQuote();
    if (listOpen) closeList();
    paragraph.push(line.trim());
  }
  flushParagraph();
  flushQuote();
  closeList();
  return output.join("\n").trimEnd();
}

function renderBlock(block: LessonBodyBlock): string {
  const content = renderMarkdown(block.markdown);
  const title = renderInlineMarkdown(block.title);
  if (block.type === "pronunciation") return `\\begin{sounds}\n${content}\n\\end{sounds}`;
  if (block.type === "etymology") return `\\begin{cousinweb}\n${content}\n\\end{cousinweb}`;
  if (block.type === "grammar" || block.type === "notice") {
    return `\\begin{grammarlens}[title={${title}}]\n${content}\n\\end{grammarlens}`;
  }
  if (block.type === "culture-pragmatics") return `\\begin{culture}\n${content}\n\\end{culture}`;
  if (block.type === "warmup") {
    return `\\begin{quote}\n\\textbf{Warm-up.} ${content}\n\\end{quote}`;
  }
  if (block.type === "recall") {
    return [
      "\\begin{tcolorbox}[breakable,colback=teal!4,colframe=teal!35!black,title={Wrap-up recall}]",
      content,
      "\\end{tcolorbox}",
    ].join("\n");
  }
  return `\\subsection*{${title}}\n${content}`;
}

function lessonTitle(lesson: ParsedLesson): string {
  const firstLine = lesson.preamble.split(/\r?\n/).find((line) => line.trim() !== "") ?? "";
  return firstLine.startsWith("# ") ? firstLine.slice(2).trim() : lesson.realization.headword;
}

function lessonSequence(lesson: ParsedLesson): number {
  return Number(stringValue(lesson.frontmatter.sequence));
}

/** Render one configured chapter from the same typed lesson AST the app receives. */
export function renderBookChapter(
  target: BookGenerationTarget,
  allLessons: ParsedLesson[],
): GeneratedBookChapter {
  const lessons = allLessons
    .filter(
      (lesson) =>
        lesson.language === target.language && lesson.realization.chapter === target.chapter,
    )
    .sort(
      (left, right) =>
        lessonSequence(left) - lessonSequence(right) ||
        left.realization.lessonId.localeCompare(right.realization.lessonId),
    );
  if (lessons.length === 0) throw new Error(`${target.language} chapter ${target.chapter}: no lessons`);
  for (const lesson of lessons) {
    if (stringValue(lesson.frontmatter.schema_version) !== "2") {
      throw new Error(`${lesson.realization.lessonId}: generated books require schema version 2`);
    }
    if (!Number.isInteger(lessonSequence(lesson))) {
      throw new Error(`${lesson.realization.lessonId}: generated books require an integer sequence`);
    }
    if (lesson.blocks.some((block) => block.type === "unknown")) {
      throw new Error(`${lesson.realization.lessonId}: generated books require known body blocks`);
    }
  }

  const sourceHash = canonicalChapterHash(lessons);
  const sections = lessons.map((lesson) => {
    const id = lesson.realization.lessonId;
    const shortTitle = lesson.realization.type.startsWith("practice")
      ? "Practice"
      : lesson.realization.headword;
    return [
      `\\section[${renderInlineMarkdown(shortTitle)}]{${renderInlineMarkdown(lessonTitle(lesson))}}`,
      `\\label{lesson:${id}}`,
      "",
      ...lesson.blocks.map(renderBlock),
    ].join("\n\n");
  });
  const tex = [
    "% GENERATED FILE. Edit canonical lessons, then run npm run generate:books.",
    `% canonical-source-hash: ${sourceHash}`,
    `% canonical-lessons: ${lessons.map((lesson) => lesson.realization.lessonId).join(", ")}`,
    "",
    `\\chapter{${renderInlineMarkdown(target.title)}}`,
    `\\label{${target.label}}`,
    "",
    "This chapter is generated from the canonical micro-lessons used by Language Ladder.",
    "Each section stays independently resumable and preserves the authored prerequisite order.",
    "",
    ...sections,
    "",
  ].join("\n");
  return {
    tex,
    sourceHash,
    lessonIds: lessons.map((lesson) => lesson.realization.lessonId),
  };
}
