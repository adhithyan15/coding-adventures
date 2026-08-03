import { canonicalChapterHash } from "./hash.js";
import type { LessonBodyBlock } from "./types.js";
import type { ParsedLesson } from "./parse.js";

export interface BookGenerationTarget {
  language: string;
  chapter: number;
  title: string;
  label: string;
  output: string;
  /** Unicode Script property whose runs need the book's dedicated font command. */
  unicodeScript?: string;
  /** LaTeX command name, without the leading backslash, used for those runs. */
  scriptCommand?: string;
}

export interface InlineRenderOptions {
  unicodeScript: string;
  scriptCommand: string;
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
    "≈": "$\\approx$",
  };
  return escaped[character] ?? character;
}

function scriptMatcher(options: InlineRenderOptions | undefined): RegExp | undefined {
  if (!options) return undefined;
  if (!/^[A-Za-z_]+$/.test(options.unicodeScript)) {
    throw new Error(`invalid Unicode script '${options.unicodeScript}'`);
  }
  if (!/^[A-Za-z@]+$/.test(options.scriptCommand)) {
    throw new Error(`invalid LaTeX script command '${options.scriptCommand}'`);
  }
  return new RegExp(`^\\p{Script_Extensions=${options.unicodeScript}}$`, "u");
}

/** Render the deliberately small inline subset used by schema-v2 lessons. */
export function renderInlineMarkdown(
  markdown: string,
  options?: InlineRenderOptions,
): string {
  const output: string[] = [];
  const emphasis: Array<"italic" | "bold"> = [];
  const script = scriptMatcher(options);
  let cursor = 0;
  const open = (kind: "italic" | "bold"): void => {
    output.push(kind === "italic" ? "\\emph{" : "\\textbf{");
    emphasis.push(kind);
  };
  const close = (): void => {
    output.push("}");
    emphasis.pop();
  };
  while (cursor < markdown.length) {
    const codePoint = markdown.codePointAt(cursor);
    const character = codePoint === undefined ? "" : String.fromCodePoint(codePoint);
    if (script?.test(character)) {
      const run: string[] = [];
      while (cursor < markdown.length) {
        const nextCodePoint = markdown.codePointAt(cursor);
        const next = nextCodePoint === undefined ? "" : String.fromCodePoint(nextCodePoint);
        if (!script.test(next)) break;
        run.push(escapeLatexCharacter(next));
        cursor += next.length;
      }
      output.push(`\\${options!.scriptCommand}{${run.join("")}}`);
      continue;
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
        output.push(renderInlineMarkdown(markdown.slice(cursor + 1, labelEnd), options));
        cursor = destinationEnd + 1;
        continue;
      }
    }
    if (markdown.startsWith("***", cursor)) {
      const top = emphasis.at(-1);
      const below = emphasis.at(-2);
      if (top && below && top !== below) {
        close();
        close();
      } else {
        open("italic");
        open("bold");
      }
      cursor += 3;
      continue;
    }
    if (markdown.startsWith("**", cursor)) {
      if (emphasis.at(-1) === "bold") close();
      else open("bold");
      cursor += 2;
      continue;
    }
    if (markdown[cursor] === "*") {
      if (emphasis.at(-1) === "italic") close();
      else open("italic");
      cursor += 1;
      continue;
    }
    output.push(escapeLatexCharacter(markdown[cursor] ?? ""));
    cursor += 1;
  }
  while (emphasis.length > 0) close();
  return output.join("");
}

function renderMarkdown(markdown: string, options?: InlineRenderOptions): string {
  const output: string[] = [];
  const paragraph: string[] = [];
  const quote: string[] = [];
  let listOpen = false;
  let listItem: string[] = [];
  const tableRows: string[][] = [];

  const flushParagraph = (): void => {
    if (paragraph.length === 0) return;
    output.push(renderInlineMarkdown(paragraph.join(" "), options), "");
    paragraph.length = 0;
  };
  const flushQuote = (): void => {
    if (quote.length === 0) return;
    output.push(
      "\\begin{quote}",
      renderInlineMarkdown(quote.join(" "), options),
      "\\end{quote}",
      "",
    );
    quote.length = 0;
  };
  const flushListItem = (): void => {
    if (listItem.length === 0) return;
    output.push(`  \\item ${renderInlineMarkdown(listItem.join(" "), options)}`);
    listItem = [];
  };
  const closeList = (): void => {
    if (!listOpen) return;
    flushListItem();
    output.push("\\end{itemize}", "");
    listOpen = false;
  };

  const flushTable = (): void => {
    if (tableRows.length === 0) return;
    const [header = [], separator = [], ...body] = tableRows;
    const isSeparator =
      separator.length === header.length &&
      separator.every((cell) => /^:?-{3,}:?$/.test(cell));
    if (!isSeparator || header.length === 0) {
      output.push(
        ...tableRows.flatMap((row) => [
          renderInlineMarkdown(`| ${row.join(" | ")} |`, options),
          "",
        ]),
      );
      tableRows.length = 0;
      return;
    }
    const columns = Array.from(
      { length: header.length },
      () => ">{\\raggedright\\arraybackslash}X",
    ).join("");
    const cells = (row: string[]): string[] =>
      Array.from({ length: header.length }, (_, index) =>
        renderInlineMarkdown(row[index] ?? "", options),
      );
    output.push(
      `\\begin{tabularx}{\\linewidth}{@{}${columns}@{}}`,
      "\\toprule",
      `${cells(header)
        .map((cell) => `\\textbf{${cell}}`)
        .join(" & ")} \\\\`,
      "\\midrule",
      ...body.map((row) => `${cells(row).join(" & ")} \\\\`),
      "\\bottomrule",
      "\\end{tabularx}",
      "",
    );
    tableRows.length = 0;
  };

  for (const rawLine of markdown.split(/\r?\n/)) {
    const line = rawLine.trimEnd();
    if (line.trim() === "") {
      flushParagraph();
      flushQuote();
      closeList();
      flushTable();
      continue;
    }
    if (/^\s*\|.*\|\s*$/.test(line)) {
      flushParagraph();
      flushQuote();
      closeList();
      tableRows.push(
        line
          .trim()
          .replace(/^\|/, "")
          .replace(/\|$/, "")
          .split("|")
          .map((cell) => cell.trim()),
      );
      continue;
    }
    flushTable();
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
        output.push("\\begin{itemize}", "\\raggedright");
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
  flushTable();
  return output.join("\n").trimEnd();
}

function renderBlock(block: LessonBodyBlock, options?: InlineRenderOptions): string {
  const content = renderMarkdown(block.markdown, options);
  const title = renderInlineMarkdown(block.title, options);
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

function sectionShortTitle(lesson: ParsedLesson, options?: InlineRenderOptions): string {
  const title = lesson.realization.type.startsWith("practice")
    ? "Practice"
    : options && stringValue(lesson.frontmatter.romanization).trim() !== ""
      ? stringValue(lesson.frontmatter.romanization)
      : lesson.realization.headword;
  return renderInlineMarkdown(
    title.replaceAll("←", " from ").replaceAll("→", " to ").replace(/\s+/g, " ").trim(),
    options,
  );
}

function targetRenderOptions(target: BookGenerationTarget): InlineRenderOptions | undefined {
  if (target.unicodeScript === undefined && target.scriptCommand === undefined) return undefined;
  if (target.unicodeScript === undefined || target.scriptCommand === undefined) {
    throw new Error(
      `${target.language} chapter ${target.chapter}: unicodeScript and scriptCommand must be declared together`,
    );
  }
  return { unicodeScript: target.unicodeScript, scriptCommand: target.scriptCommand };
}

/** Render one configured chapter from the same typed lesson AST the app receives. */
export function renderBookChapter(
  target: BookGenerationTarget,
  allLessons: ParsedLesson[],
): GeneratedBookChapter {
  const renderOptions = targetRenderOptions(target);
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
    return [
      `\\section[${sectionShortTitle(lesson, renderOptions)}]{${renderInlineMarkdown(lessonTitle(lesson), renderOptions)}}`,
      `\\label{lesson:${id}}`,
      "",
      ...lesson.blocks.map((block) => renderBlock(block, renderOptions)),
    ].join("\n\n");
  });
  const tex = [
    "% GENERATED FILE. Edit canonical lessons, then run npm run generate:books.",
    `% canonical-source-hash: ${sourceHash}`,
    `% canonical-lessons: ${lessons.map((lesson) => lesson.realization.lessonId).join(", ")}`,
    "",
    `\\chapter{${renderInlineMarkdown(target.title, renderOptions)}}`,
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
