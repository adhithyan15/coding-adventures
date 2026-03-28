/**
 * FilterBar.tsx — Filter, search, and sort controls.
 *
 * Sits above the todo list and lets users narrow down what they see.
 * All filter state is local to this component's parent (TodoList) —
 * it's NOT persisted in the store. When the user reloads, filters
 * reset to defaults.
 *
 * === Controls ===
 *
 * 1. SEARCH — free-text input that matches against title and description
 * 2. STATUS — dropdown: All / Todo / In Progress / Done
 * 3. PRIORITY — dropdown: All / Low / Medium / High / Urgent
 * 4. CATEGORY — dropdown built from unique categories in the data
 * 5. SORT — dropdown + direction toggle
 */

import type { FilterState, SortField, SortDirection, TodoStatus, Priority } from "../types.js";
import { t } from "../strings.js";

interface FilterBarProps {
  filters: FilterState;
  categories: string[];
  onFilterChange: (filters: FilterState) => void;
  todoCount: number;
  filteredCount: number;
}

export function FilterBar({
  filters,
  categories,
  onFilterChange,
  todoCount,
  filteredCount,
}: FilterBarProps) {
  /**
   * updateFilter — helper that merges a partial update into the current filters.
   *
   * Uses the spread operator to create a new FilterState with the changed field.
   * This triggers a re-render in the parent (TodoList) which re-filters the list.
   */
  function updateFilter(patch: Partial<FilterState>) {
    onFilterChange({ ...filters, ...patch });
  }

  /** resetFilters — clears all filters back to defaults. */
  function resetFilters() {
    onFilterChange({
      search: "",
      status: null,
      priority: null,
      category: "",
      sortField: "createdAt",
      sortDirection: "desc",
    });
  }

  const hasActiveFilters =
    filters.search !== "" ||
    filters.status !== null ||
    filters.priority !== null ||
    filters.category !== "";

  return (
    <div className="filter-bar" id="filter-bar">
      {/* ── Search ─────────────────────────────────────────────────────── */}
      <div className="filter-bar__search">
        <span className="filter-bar__search-icon">🔍</span>
        <input
          type="text"
          className="filter-bar__search-input"
          placeholder={t("filter.searchPlaceholder")}
          value={filters.search}
          onChange={(e) => updateFilter({ search: e.target.value })}
          id="search-input"
          aria-label={t("filter.searchPlaceholder")}
        />
        {filters.search && (
          <button
            className="filter-bar__clear-search"
            onClick={() => updateFilter({ search: "" })}
            type="button"
            aria-label="Clear search"
            id="clear-search-btn"
          >
            ✕
          </button>
        )}
      </div>

      {/* ── Filter dropdowns ───────────────────────────────────────────── */}
      <div className="filter-bar__filters">
        <select
          className="filter-bar__select"
          value={filters.status ?? "all"}
          onChange={(e) =>
            updateFilter({
              status: e.target.value === "all" ? null : (e.target.value as TodoStatus),
            })
          }
          id="status-filter"
          aria-label="Filter by status"
        >
          <option value="all">{t("filter.statusAll")}</option>
          <option value="todo">{t("filter.statusTodo")}</option>
          <option value="in-progress">{t("filter.statusProgress")}</option>
          <option value="done">{t("filter.statusDone")}</option>
        </select>

        <select
          className="filter-bar__select"
          value={filters.priority ?? "all"}
          onChange={(e) =>
            updateFilter({
              priority: e.target.value === "all" ? null : (e.target.value as Priority),
            })
          }
          id="priority-filter"
          aria-label="Filter by priority"
        >
          <option value="all">{t("filter.priorityAll")}</option>
          <option value="low">{t("filter.priorityLow")}</option>
          <option value="medium">{t("filter.priorityMedium")}</option>
          <option value="high">{t("filter.priorityHigh")}</option>
          <option value="urgent">{t("filter.priorityUrgent")}</option>
        </select>

        {categories.length > 0 && (
          <select
            className="filter-bar__select"
            value={filters.category}
            onChange={(e) => updateFilter({ category: e.target.value })}
            id="category-filter"
            aria-label="Filter by category"
          >
            <option value="">{t("filter.categoryAll")}</option>
            {categories.map((cat) => (
              <option key={cat} value={cat}>
                {cat.charAt(0).toUpperCase() + cat.slice(1)}
              </option>
            ))}
          </select>
        )}

        {/* ── Sort ───────────────────────────────────────────────────────── */}
        <select
          className="filter-bar__select"
          value={filters.sortField}
          onChange={(e) => updateFilter({ sortField: e.target.value as SortField })}
          id="sort-field"
          aria-label="Sort by"
        >
          <option value="createdAt">{t("filter.sortCreated")}</option>
          <option value="updatedAt">{t("filter.sortUpdated")}</option>
          <option value="dueDate">{t("filter.sortDueDate")}</option>
          <option value="priority">{t("filter.sortPriority")}</option>
          <option value="title">{t("filter.sortTitle")}</option>
        </select>

        <button
          className="filter-bar__sort-dir"
          onClick={() =>
            updateFilter({
              sortDirection: filters.sortDirection === "asc" ? "desc" : "asc" as SortDirection,
            })
          }
          type="button"
          title={filters.sortDirection === "asc" ? t("filter.sortAscending") : t("filter.sortDescending")}
          id="sort-direction-btn"
          aria-label={`Sort ${filters.sortDirection === "asc" ? "ascending" : "descending"}`}
        >
          {filters.sortDirection === "asc" ? "↑" : "↓"}
        </button>

        {hasActiveFilters && (
          <button
            className="filter-bar__clear"
            onClick={resetFilters}
            type="button"
            id="clear-filters-btn"
          >
            {t("filter.clearFilters")}
          </button>
        )}
      </div>

      {/* ── Count ────────────────────────────────────────────────────────── */}
      <div className="filter-bar__count" id="todo-count">
        {filteredCount === todoCount
          ? `${todoCount} ${todoCount === 1 ? "todo" : "todos"}`
          : `${filteredCount} of ${todoCount} todos`}
      </div>
    </div>
  );
}
