import type {
  BlockNode,
  DocumentNode,
  InlineNode,
} from "@coding-adventures/document-ast";
import {
  Kinds,
  streamOf,
  type ContentNode,
  type ContentSource,
  type JsonValue,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { parseLandingModel, type LandingModel } from "./model.ts";

const decoder = new TextDecoder("utf-8", { fatal: true });

const parseLanding = defineStage({
  name: "@coding-adventures/site-landing-parse",
  version: "0.1.0",
  apiVersion: 1,
  description: "Parse the declarative landing-page model into Content IR.",
  consumes: streamOf(Kinds.ContentSource),
  produces: streamOf(Kinds.ContentNode),
  capabilities: [],
  configSchema: { type: "object", properties: {} },
  async *run(rawInput, _config, ctx) {
    for await (const source of rawInput as AsyncIterable<ContentSource>) {
      ctx.cancellation.throwIfCancelled();
      let raw: unknown;
      try {
        raw = JSON.parse(decoder.decode(source.bytes));
      } catch (error) {
        throw new Error(`site-landing-parse: ${source.path} is not valid UTF-8 JSON: ${String(error)}`);
      }
      const model = parseLandingModel(raw);
      const node: ContentNode = {
        identity: source.identity,
        revision: source.revision,
        document: modelToDocument(model),
        frontmatter: {
          title: model.site.title,
          excerpt: model.site.description,
          slug: "index",
          landing: raw as JsonValue,
        },
        route: null,
        assetRefs: [],
        sourcePath: source.path,
      };
      yield node as never;
    }
  },
});

function modelToDocument(model: LandingModel): DocumentNode {
  const children: BlockNode[] = [
    heading(1, `${model.hero.title} ${model.hero.accent}`),
    paragraph(model.hero.intro),
    heading(2, "Learning paths"),
    ...model.paths.flatMap(item => [heading(3, item.title), paragraph(item.description)]),
    heading(2, "Live labs"),
    ...model.labs.flatMap(item => [heading(3, item.title), paragraph(item.description)]),
    heading(2, model.forme.title),
    paragraph(model.forme.description),
    heading(2, "What the lab is building now"),
    ...model.workshop.flatMap(item => [heading(3, item.title), paragraph(item.description)]),
    {
      type: "paragraph",
      children: [{
        type: "image",
        destination: model.site.ogImage,
        title: "Coding Adventures social preview",
        alt: "Coding Adventures — Build the stack.",
      }],
    },
  ];
  return { type: "document", children };
}

function heading(level: 1 | 2 | 3, value: string): BlockNode {
  return { type: "heading", level, children: [text(value)] };
}

function paragraph(value: string): BlockNode {
  return { type: "paragraph", children: [text(value)] };
}

function text(value: string): InlineNode {
  return { type: "text", value };
}

export default parseLanding;
export { parseLanding, modelToDocument };
