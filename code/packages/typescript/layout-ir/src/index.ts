/**
 * @coding-adventures/layout-ir
 *
 * Universal Layout Intermediate Representation — the shared vocabulary between
 * content producers (Mosaic IR, DocumentAST) and layout algorithms (flexbox,
 * block flow, grid).
 *
 * This package exports only types and pure builder helpers. It has zero runtime
 * dependencies and performs no I/O. Every other layout and paint package in the
 * coding-adventures stack depends on this one.
 *
 * Typical usage:
 *
 *   import {
 *     container, leaf_text, size_fill, size_wrap,
 *     font_spec, rgb, edges_all, constraints_width
 *   } from "@coding-adventures/layout-ir";
 *
 *   const tree = container([
 *     leaf_text({ kind: "text", value: "Hello",
 *                 font: font_spec("Arial", 16), color: rgb(0,0,0),
 *                 maxLines: null, textAlign: "start" }),
 *   ], {
 *     width: size_fill(),
 *     ext: { flex: { direction: "column", gap: 8 } }
 *   });
 *
 * See: code/specs/UI02-layout-ir.md
 */

// ─── Types ───────────────────────────────────────────────────────────────────
export type {
  SizeValue,
  Edges,
  Color,
  FontSpec,
  TextAlign,
  ImageFit,
  TextContent,
  ImageContent,
  NodeContent,
  LayoutNode,
  Constraints,
  PositionedNode,
  MeasureResult,
  TextMeasurer,
} from "./types.js";

// ─── Builders ────────────────────────────────────────────────────────────────
export {
  // SizeValue
  size_fixed,
  size_fill,
  size_wrap,

  // Edges
  edges_all,
  edges_xy,
  edges_zero,

  // Color
  rgba,
  rgb,
  color_transparent,

  // FontSpec
  font_spec,
  font_bold,
  font_italic,

  // Constraints
  constraints_fixed,
  constraints_width,
  constraints_unconstrained,
  constraints_shrink,

  // LayoutNode
  node,
  leaf_text,
  leaf_image,
  container,
  type NodeOpts,

  // PositionedNode
  positioned,
} from "./builders.js";
