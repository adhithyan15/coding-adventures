import { describe, it, expect, beforeAll } from "vitest";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";
import { ProgressBar } from "./ProgressBar.js";

beforeAll(() => {
  initI18n({ en });
});

describe("ProgressBar", () => {
  it("renders a progressbar role element", () => {
    render(<ProgressBar checked={0} total={5} />);
    expect(screen.getByRole("progressbar")).toBeInTheDocument();
  });

  it("sets aria-valuenow, aria-valuemin, aria-valuemax", () => {
    render(<ProgressBar checked={3} total={5} />);
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "3");
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "5");
  });

  it("shows label text with counts", () => {
    render(<ProgressBar checked={2} total={4} />);
    expect(screen.getByText(/2 of 4/i)).toBeInTheDocument();
  });

  it("0 of 0 does not divide by zero", () => {
    expect(() => render(<ProgressBar checked={0} total={0} />)).not.toThrow();
  });
});
