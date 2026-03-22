/**
 * Generic hook for managing tab state with WAI-ARIA keyboard navigation.
 *
 * Implements the full WAI-ARIA Tabs pattern:
 *   - ArrowRight/ArrowDown: move to next tab (wraps around)
 *   - ArrowLeft/ArrowUp: move to previous tab (wraps around)
 *   - Home: move to first tab
 *   - End: move to last tab
 *
 * Uses roving tabindex — only the active tab has tabIndex=0,
 * all others have tabIndex=-1. This means Tab key moves focus
 * out of the tab list, not between tabs.
 *
 * @example
 * ```tsx
 * const { tabListRef, handleKeyDown, getTabProps } = useTabs({
 *   items: [{ id: "a", label: "Alpha" }, { id: "b", label: "Beta" }],
 *   activeTab: "a",
 *   onActiveChange: setActive,
 * });
 * ```
 */

import { useCallback, useRef } from "react";
import type { TabItem } from "../types.js";

export interface UseTabsOptions<T extends string> {
  items: TabItem<T>[];
  activeTab: T;
  onActiveChange: (id: T) => void;
}

export interface TabProps {
  role: "tab";
  "aria-selected": boolean;
  "aria-controls": string;
  tabIndex: 0 | -1;
  onClick: () => void;
}

export function useTabs<T extends string>(options: UseTabsOptions<T>) {
  const { items, activeTab, onActiveChange } = options;
  const tabListRef = useRef<HTMLElement>(null);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const currentIndex = items.findIndex((item) => item.id === activeTab);
      let nextIndex = currentIndex;

      if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        nextIndex = (currentIndex + 1) % items.length;
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        nextIndex = (currentIndex - 1 + items.length) % items.length;
      } else if (e.key === "Home") {
        nextIndex = 0;
      } else if (e.key === "End") {
        nextIndex = items.length - 1;
      } else {
        return;
      }

      e.preventDefault();
      onActiveChange(items[nextIndex]!.id);

      // Focus the newly active tab button
      const tabList = tabListRef.current;
      if (tabList) {
        const buttons = tabList.querySelectorAll<HTMLButtonElement>("[role=tab]");
        buttons[nextIndex]?.focus();
      }
    },
    [activeTab, items, onActiveChange],
  );

  const getTabProps = useCallback(
    (id: T): TabProps => ({
      role: "tab" as const,
      "aria-selected": activeTab === id,
      "aria-controls": `panel-${id}`,
      tabIndex: activeTab === id ? 0 : -1,
      onClick: () => onActiveChange(id),
    }),
    [activeTab, onActiveChange],
  );

  return { tabListRef, handleKeyDown, getTabProps };
}
