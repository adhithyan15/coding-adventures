import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import { stateLoadAction, createTodoAction, toggleTodoAction } from "../actions.js";
import { TodoList } from "./TodoList.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  store.dispatch(stateLoadAction([], [], []));
});

describe("TodoList", () => {
  it("shows empty state when no todos", () => {
    render(<TodoList onNavigate={() => {}} />);
    expect(screen.getByText(/no todos yet/i)).toBeInTheDocument();
  });

  it("renders todo items", () => {
    store.dispatch(createTodoAction("Buy groceries", "Milk, eggs"));
    render(<TodoList onNavigate={() => {}} />);
    expect(screen.getByText("Buy groceries")).toBeInTheDocument();
    expect(screen.getByText("Milk, eggs")).toBeInTheDocument();
  });

  it("groups todos by status", () => {
    store.dispatch(createTodoAction("Task A", ""));
    store.dispatch(createTodoAction("Task B", ""));
    const todoB = store.getState().todos[1]!;
    store.dispatch(toggleTodoAction(todoB.id)); // B → in-progress
    render(<TodoList onNavigate={() => {}} />);
    expect(screen.getByText("To Do")).toBeInTheDocument();
    expect(screen.getByText("In Progress")).toBeInTheDocument();
  });

  it("toggle button cycles status", () => {
    store.dispatch(createTodoAction("Task", ""));
    render(<TodoList onNavigate={() => {}} />);
    const toggleBtn = screen.getByLabelText(/toggle status/i);
    fireEvent.click(toggleBtn); // todo → in-progress
    expect(store.getState().todos[0]?.status).toBe("in-progress");
  });

  it("New Todo button navigates", () => {
    const navigate = vi.fn();
    render(<TodoList onNavigate={navigate} />);
    fireEvent.click(screen.getByText("New Todo"));
    expect(navigate).toHaveBeenCalledWith("/todos/new");
  });

  it("Edit button navigates to edit page", () => {
    store.dispatch(createTodoAction("Task", ""));
    const todoId = store.getState().todos[0]!.id;
    const navigate = vi.fn();
    render(<TodoList onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Edit"));
    expect(navigate).toHaveBeenCalledWith(`/todos/${todoId}/edit`);
  });
});
