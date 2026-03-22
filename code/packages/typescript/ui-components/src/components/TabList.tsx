/**
 * Accessible tab list component (WAI-ARIA Tabs pattern).
 *
 * Implements the full keyboard navigation spec:
 *   - ArrowRight/ArrowDown: next tab
 *   - ArrowLeft/ArrowUp: previous tab
 *   - Home: first tab
 *   - End: last tab
 *
 * Each tab uses roving tabindex — only the active tab is focusable.
 * The tab list wraps around at both ends.
 *
 * @example
 * ```tsx
 * const tabs = [
 *   { id: "vacuum-tube", label: t("tabs.vacuumTube") },
 *   { id: "bjt", label: t("tabs.bjt") },
 * ];
 *
 * <TabList
 *   items={tabs}
 *   activeTab={active}
 *   onActiveChange={setActive}
 *   ariaLabel="Historical eras"
 * />
 * ```
 */

import type { TabItem } from "../types.js";
import { useTabs } from "../hooks/useTabs.js";

export interface TabListProps<T extends string = string> {
  /** The tab items to render. Labels should already be translated. */
  items: TabItem<T>[];
  /** Currently active tab id. */
  activeTab: T;
  /** Called when the user selects a different tab. */
  onActiveChange: (id: T) => void;
  /** Accessible label for the tab list (e.g., "Visualization layers"). */
  ariaLabel: string;
  /** Optional CSS class for the nav element. */
  className?: string;
  /** Optional CSS class for individual tab buttons. */
  tabClassName?: string;
  /** Optional CSS class for the active tab button. */
  activeTabClassName?: string;
}

export function TabList<T extends string>({
  items,
  activeTab,
  onActiveChange,
  ariaLabel,
  className = "tab-list",
  tabClassName = "tab-list__tab",
  activeTabClassName = "tab-list__tab--active",
}: TabListProps<T>) {
  const { tabListRef, handleKeyDown, getTabProps } = useTabs({
    items,
    activeTab,
    onActiveChange,
  });

  return (
    <nav
      className={className}
      role="tablist"
      aria-label={ariaLabel}
      ref={tabListRef}
      onKeyDown={handleKeyDown}
    >
      {items.map((item) => {
        const props = getTabProps(item.id);
        const isActive = props["aria-selected"];
        return (
          <button
            key={item.id}
            className={`${tabClassName} ${isActive ? activeTabClassName : ""}`}
            {...props}
          >
            {item.label}
          </button>
        );
      })}
    </nav>
  );
}
