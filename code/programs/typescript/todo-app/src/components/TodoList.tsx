/**
 * TodoList.tsx — Main list view with filtering, sorting, and summary stats.
 *
 * This is the primary screen of the app. It displays:
 *   1. A summary bar showing counts (total, active, completed)
 *   2. A FilterBar for search, status, priority, category, and sort
 *   3. The list of TodoCards matching the current filters
 *   4. An EmptyState when no items match
 *
 * === Filtering logic ===
 *
 * Filters are applied in this order:
 *   1. Status filter (exact match)
 *   2. Priority filter (exact match)
 *   3. Category filter (case-insensitive match)
 *   4. Search filter (case-insensitive substring match on title + description)
 *
 * All filters are AND-combined: an item must pass ALL active filters to appear.
 *
 * === Sorting logic ===
 *
 * After filtering, items are sorted by the selected field. Null values
 * (e.g., no due date) are pushed to the end regardless of sort direction.
 */

import { useState, useMemo, useCallback } from "react";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import {
  toggleStatusAction,
  deleteTaskAction,
  clearCompletedAction,
} from "../actions.js";
import type { FilterState, Task } from "../types.js";
import { getUniqueCategories, PRIORITY_WEIGHT } from "../types.js";
import { TodoCard } from "./TodoCard.js";
import { FilterBar } from "./FilterBar.js";
import { EmptyState } from "./EmptyState.js";

interface TodoListProps {
  onNavigate: (path: string) => void;
}

/**
 * defaultFilters — the initial filter state when the page loads.
 *
 * Shows all statuses, all priorities, all categories, sorted by
 * creation date (newest first).
 */
const defaultFilters: FilterState = {
  search: "",
  status: null,
  priority: null,
  category: "",
  sortField: "createdAt",
  sortDirection: "desc",
};

/**
 * applyFilters — filters the todo list based on the current filter state.
 *
 * Returns a new array (never mutates the input). Each filter is applied
 * as a boolean predicate. An item must pass ALL predicates to be included.
 */
function applyFilters(todos: Task[], filters: FilterState): Task[] {
  return todos.filter((todo) => {
    // Status filter
    if (filters.status !== null && todo.status !== filters.status) {
      return false;
    }

    // Priority filter
    if (filters.priority !== null && todo.priority !== filters.priority) {
      return false;
    }

    // Category filter (case-insensitive)
    if (
      filters.category !== "" &&
      todo.category.toLowerCase() !== filters.category.toLowerCase()
    ) {
      return false;
    }

    // Search filter (case-insensitive substring match)
    if (filters.search !== "") {
      const query = filters.search.toLowerCase();
      const matchesTitle = todo.title.toLowerCase().includes(query);
      const matchesDescription = todo.description.toLowerCase().includes(query);
      if (!matchesTitle && !matchesDescription) {
        return false;
      }
    }

    return true;
  });
}

/**
 * applySorting — sorts the filtered list by the selected field.
 *
 * Creates a new sorted array (never mutates). Null values (e.g., no due date)
 * are pushed to the end regardless of sort direction.
 */
function applySorting(todos: Task[], filters: FilterState): Task[] {
  const sorted = [...todos];
  const dir = filters.sortDirection === "asc" ? 1 : -1;

  sorted.sort((a, b) => {
    switch (filters.sortField) {
      case "title":
        return dir * a.title.localeCompare(b.title);

      case "priority":
        return dir * (PRIORITY_WEIGHT[a.priority] - PRIORITY_WEIGHT[b.priority]);

      case "dueDate": {
        // Null due dates go to the end
        if (!a.dueDate && !b.dueDate) return 0;
        if (!a.dueDate) return 1;
        if (!b.dueDate) return -1;
        return dir * a.dueDate.localeCompare(b.dueDate);
      }

      case "updatedAt":
        return dir * (a.updatedAt - b.updatedAt);

      case "createdAt":
      default:
        return dir * (a.createdAt - b.createdAt);
    }
  });

  return sorted;
}

export function TodoList({ onNavigate }: TodoListProps) {
  const state = useStore(store);
  const [filters, setFilters] = useState<FilterState>(defaultFilters);

  // ── Derived data (memoized for performance) ─────────────────────────────
  const categories = useMemo(
    () => getUniqueCategories(state.tasks),
    [state.tasks],
  );

  const filteredTodos = useMemo(
    () => applySorting(applyFilters(state.tasks, filters), filters),
    [state.tasks, filters],
  );

  const todoCount = state.tasks.filter((t) => t.status === "todo").length;
  const inProgressCount = state.tasks.filter((t) => t.status === "in-progress").length;
  const doneCount = state.tasks.filter((t) => t.status === "done").length;

  // ── Handlers ────────────────────────────────────────────────────────────
  const handleToggleStatus = useCallback((id: string) => {
    store.dispatch(toggleStatusAction(id));
  }, []);

  const handleEdit = useCallback(
    (id: string) => {
      onNavigate(`/edit/${id}`);
    },
    [onNavigate],
  );

  const handleDelete = useCallback((id: string) => {
    store.dispatch(deleteTaskAction(id));
  }, []);

  const handleClearCompleted = useCallback(() => {
    store.dispatch(clearCompletedAction());
  }, []);

  return (
    <div className="todo-list" id="todo-list">
      {/* ── Summary stats ──────────────────────────────────────────────── */}
      <div className="todo-list__summary" id="summary-bar">
        <div className="todo-list__stat">
          <span className="todo-list__stat-value" id="stat-total">{state.tasks.length}</span>
          <span className="todo-list__stat-label">Total</span>
        </div>
        <div className="todo-list__stat">
          <span className="todo-list__stat-value todo-list__stat-value--todo" id="stat-todo">{todoCount}</span>
          <span className="todo-list__stat-label">Todo</span>
        </div>
        <div className="todo-list__stat">
          <span className="todo-list__stat-value todo-list__stat-value--progress" id="stat-in-progress">{inProgressCount}</span>
          <span className="todo-list__stat-label">In Progress</span>
        </div>
        <div className="todo-list__stat">
          <span className="todo-list__stat-value todo-list__stat-value--done" id="stat-done">{doneCount}</span>
          <span className="todo-list__stat-label">Done</span>
        </div>
      </div>

      {/* ── Filter bar ─────────────────────────────────────────────────── */}
      <FilterBar
        filters={filters}
        categories={categories}
        onFilterChange={setFilters}
        todoCount={state.tasks.length}
        filteredCount={filteredTodos.length}
      />

      {/* ── Action bar ─────────────────────────────────────────────────── */}
      <div className="todo-list__action-bar">
        <button
          className="btn btn--primary"
          onClick={() => onNavigate("/new")}
          type="button"
          id="create-todo-btn"
        >
          + New Todo
        </button>
        {doneCount > 0 && (
          <button
            className="btn btn--ghost"
            onClick={handleClearCompleted}
            type="button"
            id="clear-completed-btn"
          >
            Clear completed ({doneCount})
          </button>
        )}
      </div>

      {/* ── Todo cards ─────────────────────────────────────────────────── */}
      {filteredTodos.length === 0 ? (
        <EmptyState
          hasNoTodos={state.tasks.length === 0}
          onCreateClick={() => onNavigate("/new")}
        />
      ) : (
        <div className="todo-list__cards" id="todo-cards">
          {filteredTodos.map((todo) => (
            <TodoCard
              key={todo.id}
              todo={todo}
              onToggleStatus={handleToggleStatus}
              onEdit={handleEdit}
              onDelete={handleDelete}
            />
          ))}
        </div>
      )}
    </div>
  );
}
