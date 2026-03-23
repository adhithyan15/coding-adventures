import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import { stateLoadAction } from "../actions.js";
import { TemplateEditor } from "./TemplateEditor.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  store.dispatch(stateLoadAction([], []));
});

describe("TemplateEditor", () => {
  it("shows New Checklist title when creating", () => {
    render(<TemplateEditor onNavigate={() => {}} />);
    expect(screen.getByText("New Checklist")).toBeInTheDocument();
  });

  it("shows validation error if saved with empty name", () => {
    render(<TemplateEditor onNavigate={() => {}} />);
    fireEvent.click(screen.getByText("Save"));
    expect(
      screen.getByText(/please enter a checklist name/i),
    ).toBeInTheDocument();
  });

  it("creates template and navigates on save", () => {
    const navigate = vi.fn();
    render(<TemplateEditor onNavigate={navigate} />);
    fireEvent.change(screen.getByPlaceholderText("Checklist name"), {
      target: { value: "My New List" },
    });
    fireEvent.click(screen.getByText("Save"));
    expect(store.getState().templates).toHaveLength(1);
    expect(store.getState().templates[0]?.name).toBe("My New List");
    expect(navigate).toHaveBeenCalledWith("/");
  });

  it("Cancel navigates back without saving", () => {
    const navigate = vi.fn();
    render(<TemplateEditor onNavigate={navigate} />);
    fireEvent.change(screen.getByPlaceholderText("Checklist name"), {
      target: { value: "Unsaved" },
    });
    fireEvent.click(screen.getByText("Cancel"));
    expect(store.getState().templates).toHaveLength(0);
    expect(navigate).toHaveBeenCalledWith("/");
  });
});
