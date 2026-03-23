import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import { stateLoadAction } from "../actions.js";
import { TodoEditor } from "./TodoEditor.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  store.dispatch(stateLoadAction([], [], []));
});

describe("TodoEditor", () => {
  it("shows New Todo title when creating", () => {
    render(<TodoEditor onNavigate={() => {}} />);
    expect(screen.getByText("New Todo")).toBeInTheDocument();
  });

  it("shows validation error if saved with empty title", () => {
    render(<TodoEditor onNavigate={() => {}} />);
    fireEvent.click(screen.getByText("Save"));
    expect(screen.getByText(/please enter/i)).toBeInTheDocument();
  });

  it("creates todo and navigates on save", () => {
    const navigate = vi.fn();
    render(<TodoEditor onNavigate={navigate} />);
    fireEvent.change(screen.getByPlaceholderText(/what needs to be done/i), {
      target: { value: "Buy milk" },
    });
    fireEvent.click(screen.getByText("Save"));
    expect(store.getState().todos).toHaveLength(1);
    expect(store.getState().todos[0]?.title).toBe("Buy milk");
    expect(navigate).toHaveBeenCalledWith("/todos");
  });

  it("Cancel navigates back without saving", () => {
    const navigate = vi.fn();
    render(<TodoEditor onNavigate={navigate} />);
    fireEvent.change(screen.getByPlaceholderText(/what needs to be done/i), {
      target: { value: "Unsaved" },
    });
    fireEvent.click(screen.getByText("Cancel"));
    expect(store.getState().todos).toHaveLength(0);
    expect(navigate).toHaveBeenCalledWith("/todos");
  });
});
