import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { appState } from "../state.js";
import { TemplateEditor } from "./TemplateEditor.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  appState.templates.length = 0;
  appState.instances.length = 0;
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
    expect(appState.templates).toHaveLength(1);
    expect(appState.templates[0]?.name).toBe("My New List");
    expect(navigate).toHaveBeenCalledWith("/");
  });

  it("Cancel navigates back without saving", () => {
    const navigate = vi.fn();
    render(<TemplateEditor onNavigate={navigate} />);
    fireEvent.change(screen.getByPlaceholderText("Checklist name"), {
      target: { value: "Unsaved" },
    });
    fireEvent.click(screen.getByText("Cancel"));
    expect(appState.templates).toHaveLength(0);
    expect(navigate).toHaveBeenCalledWith("/");
  });
});
