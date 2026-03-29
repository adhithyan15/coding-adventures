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
 *   FlashCard,
 *   RatingButtons,
 *   ProgressBar,
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
 * @import "@coding-adventures/ui-components/src/styles/flash-card.css";
 * @import "@coding-adventures/ui-components/src/styles/rating-buttons.css";
 * @import "@coding-adventures/ui-components/src/styles/progress-bar.css";
 * @import "@coding-adventures/ui-components/src/styles/table.css";
 * ```
 */

// Types
export type { TabItem } from "./types.js";
export type { TabListProps } from "./components/TabList.js";
export type { SliderControlProps } from "./components/SliderControl.js";
export type { TreeViewProps, TreeViewNode } from "./components/TreeView.js";
export type { BranchGroupProps } from "./components/BranchGroup.js";
export type { DatePickerProps } from "./components/DatePicker.js";
export type { FlashCardProps } from "./components/FlashCard.js";
export type { RatingButtonsProps, Rating } from "./components/RatingButtons.js";
export type { ProgressBarProps } from "./components/ProgressBar.js";
export type { CalendarViewProps, CalendarItem, CalendarViewGranularity, DayOfWeek } from "./components/CalendarView.js";
export type { TableProps, TableBaseProps, ColumnDef, CellAlignment, RowKeyFn } from "./components/Table.js";
export type { CanvasTheme } from "./hooks/useCanvasTheme.js";
export type { CellPosition, UseGridKeyboardOptions, UseGridKeyboardReturn } from "./hooks/useGridKeyboard.js";
export type { UseTabsOptions, TabProps } from "./hooks/useTabs.js";
export type { LocaleMap } from "./i18n/index.js";

// Components
export { TabList } from "./components/TabList.js";
export { SliderControl } from "./components/SliderControl.js";
export { TreeView } from "./components/TreeView.js";
export { BranchGroup } from "./components/BranchGroup.js";
export { DatePicker } from "./components/DatePicker.js";
export { FlashCard } from "./components/FlashCard.js";
export { RatingButtons } from "./components/RatingButtons.js";
export { ProgressBar } from "./components/ProgressBar.js";
export { CalendarView } from "./components/CalendarView.js";
export { Table } from "./components/Table.js";
export { DataTable } from "./components/DataTable.js";
export { CanvasTable } from "./components/CanvasTable.js";
export { resolveCellValue } from "./components/Table.js";

// Hooks
export { useTabs } from "./hooks/useTabs.js";
export { useAnimationFrame, useAutoStep } from "./hooks/useAnimationFrame.js";
export { useAnimatedNumber } from "./hooks/useAnimatedNumber.js";
export { useReducedMotion } from "./hooks/useReducedMotion.js";
export { useCanvasTheme } from "./hooks/useCanvasTheme.js";
export { useGridKeyboard } from "./hooks/useGridKeyboard.js";

// i18n
export {
  initI18n,
  translate,
  setLocale,
  getAvailableLocales,
  useTranslation,
} from "./i18n/index.js";
