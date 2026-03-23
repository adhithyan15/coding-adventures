/**
 * @coding-adventures/ui-components
 *
 * Shared React UI components for interactive visualizations.
 * Provides accessible tabs, i18n, animation hooks, and dark theme CSS.
 *
 * === Usage ===
 *
 * ```typescript
 * import {
 *   TabList,
 *   SliderControl,
 *   useTranslation,
 *   initI18n,
 *   useAnimationFrame,
 *   useReducedMotion,
 * } from "@coding-adventures/ui-components";
 * ```
 *
 * === CSS ===
 *
 * Import the shared styles in your app's CSS:
 * ```css
 * @import "@coding-adventures/ui-components/src/styles/theme.css";
 * @import "@coding-adventures/ui-components/src/styles/accessibility.css";
 * @import "@coding-adventures/ui-components/src/styles/tree.css";
 * @import "@coding-adventures/ui-components/src/styles/date-picker.css";
 * ```
 */

// Types
export type { TabItem } from "./types.js";
export type { TabListProps } from "./components/TabList.js";
export type { SliderControlProps } from "./components/SliderControl.js";
export type { TreeViewProps, TreeViewNode } from "./components/TreeView.js";
export type { BranchGroupProps } from "./components/BranchGroup.js";
export type { DatePickerProps } from "./components/DatePicker.js";
export type { UseTabsOptions, TabProps } from "./hooks/useTabs.js";
export type { LocaleMap } from "./i18n/index.js";

// Components
export { TabList } from "./components/TabList.js";
export { SliderControl } from "./components/SliderControl.js";
export { TreeView } from "./components/TreeView.js";
export { BranchGroup } from "./components/BranchGroup.js";
export { DatePicker } from "./components/DatePicker.js";

// Hooks
export { useTabs } from "./hooks/useTabs.js";
export { useAnimationFrame, useAutoStep } from "./hooks/useAnimationFrame.js";
export { useReducedMotion } from "./hooks/useReducedMotion.js";

// i18n
export {
  initI18n,
  translate,
  setLocale,
  getAvailableLocales,
  useTranslation,
} from "./i18n/index.js";
