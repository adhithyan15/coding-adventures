import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ProgressBar } from "./ProgressBar.js";

describe("ProgressBar", () => {
  it("renders a progressbar role", () => {
    render(<ProgressBar value={5} max={10} />);
    expect(screen.getByRole("progressbar")).toBeTruthy();
  });

  it("sets aria-valuenow correctly", () => {
    render(<ProgressBar value={3} max={20} />);
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("3");
  });

  it("sets aria-valuemin to 0", () => {
    render(<ProgressBar value={3} max={20} />);
    expect(screen.getByRole("progressbar").getAttribute("aria-valuemin")).toBe("0");
  });

  it("sets aria-valuemax correctly", () => {
    render(<ProgressBar value={3} max={20} />);
    expect(screen.getByRole("progressbar").getAttribute("aria-valuemax")).toBe("20");
  });

  it("renders the label when provided", () => {
    render(<ProgressBar value={5} max={10} label="5 / 10 cards" />);
    expect(screen.getByText("5 / 10 cards")).toBeTruthy();
  });

  it("does not render a label element when label is not provided", () => {
    const { container } = render(<ProgressBar value={5} max={10} />);
    expect(container.querySelector(".progress-bar__label")).toBeNull();
  });

  it("fill width is 50% when value=5, max=10", () => {
    const { container } = render(<ProgressBar value={5} max={10} />);
    const fill = container.querySelector(".progress-bar__fill") as HTMLElement;
    expect(fill.style.width).toBe("50%");
  });

  it("fill width is 0% when value=0", () => {
    const { container } = render(<ProgressBar value={0} max={10} />);
    const fill = container.querySelector(".progress-bar__fill") as HTMLElement;
    expect(fill.style.width).toBe("0%");
  });

  it("fill width is 100% when value equals max", () => {
    const { container } = render(<ProgressBar value={10} max={10} />);
    const fill = container.querySelector(".progress-bar__fill") as HTMLElement;
    expect(fill.style.width).toBe("100%");
  });

  it("clamps fill width to 100% when value exceeds max", () => {
    const { container } = render(<ProgressBar value={15} max={10} />);
    const fill = container.querySelector(".progress-bar__fill") as HTMLElement;
    expect(fill.style.width).toBe("100%");
  });

  it("clamps fill width to 0% when value is negative", () => {
    const { container } = render(<ProgressBar value={-1} max={10} />);
    const fill = container.querySelector(".progress-bar__fill") as HTMLElement;
    expect(fill.style.width).toBe("0%");
  });

  it("applies --complete modifier class when value equals max", () => {
    const { container } = render(<ProgressBar value={10} max={10} />);
    expect(
      container.querySelector(".progress-bar__fill--complete"),
    ).toBeTruthy();
  });

  it("does not apply --complete modifier class when not done", () => {
    const { container } = render(<ProgressBar value={5} max={10} />);
    expect(
      container.querySelector(".progress-bar__fill--complete"),
    ).toBeNull();
  });

  it("handles max=0 gracefully (no divide by zero)", () => {
    const { container } = render(<ProgressBar value={0} max={0} />);
    const fill = container.querySelector(".progress-bar__fill") as HTMLElement;
    expect(fill.style.width).toBe("0%");
  });

  it("accepts a custom className", () => {
    const { container } = render(
      <ProgressBar value={5} max={10} className="custom-bar" />,
    );
    expect(container.querySelector(".custom-bar")).toBeTruthy();
  });
});
