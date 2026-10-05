/**
 * @coding-adventures/forme-authoring-editor
 *
 * Accessible declarative block editor for the Forme authoring shell
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 *
 */

export { AUTHORING_EDITOR_CSS, AuthoringEditor } from "./editor.js";
export type { AuthoringEditorProps } from "./editor.js";
export {
  BlockEditError,
  createDefaultBlock,
  insertBlock,
  moveBlock,
  removeBlock,
  replaceBlock,
} from "./blocks.js";
export type { DefaultBlockKind } from "./blocks.js";
export {
  EditorBoundaryError,
  MAX_EDITOR_CONTRIBUTIONS,
  createEditorPluginRequest,
  validateEditorContributions,
  validateThemeOptions,
} from "./plugin.js";
export type {
  EditorContribution,
  EditorPluginBridge,
  EditorPluginRequest,
  EditorPluginTarget,
  EditorSlot,
  EditorThemeOption,
} from "./plugin.js";

export const VERSION = "1.0.0";
