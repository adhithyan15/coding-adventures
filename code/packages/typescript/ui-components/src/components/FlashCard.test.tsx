import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { FlashCard } from "./FlashCard.js";

describe("FlashCard", () => {
  it("renders the front text", () => {
    render(<FlashCard front="What is 2+2?" back="4" revealed={false} />);
    expect(screen.getByText("What is 2+2?")).toBeTruthy();
  });

  it("renders the back text", () => {
    render(<FlashCard front="What is 2+2?" back="4" revealed={false} />);
    expect(screen.getByText("4")).toBeTruthy();
  });

  it("shows 'Answer' label on the back face", () => {
    render(<FlashCard front="Q" back="A" revealed={false} />);
    expect(screen.getByText("Answer")).toBeTruthy();
  });

  it("has aria-label 'Question' when not revealed", () => {
    render(<FlashCard front="Q" back="A" revealed={false} />);
    const region = screen.getByRole("region");
    expect(region.getAttribute("aria-label")).toBe("Question");
  });

  it("has aria-label 'Answer' when revealed", () => {
    render(<FlashCard front="Q" back="A" revealed={true} />);
    const region = screen.getByRole("region");
    expect(region.getAttribute("aria-label")).toBe("Answer");
  });

  it("applies --revealed modifier class when revealed", () => {
    const { container } = render(<FlashCard front="Q" back="A" revealed={true} />);
    const inner = container.querySelector(".flash-card__inner");
    expect(inner?.classList.contains("flash-card__inner--revealed")).toBe(true);
  });

  it("does not apply --revealed modifier class when not revealed", () => {
    const { container } = render(<FlashCard front="Q" back="A" revealed={false} />);
    const inner = container.querySelector(".flash-card__inner");
    expect(inner?.classList.contains("flash-card__inner--revealed")).toBe(false);
  });

  it("accepts a custom className", () => {
    const { container } = render(
      <FlashCard front="Q" back="A" revealed={false} className="custom-card" />,
    );
    expect(container.querySelector(".custom-card")).toBeTruthy();
  });

  it("hides the front face with aria-hidden when revealed", () => {
    const { container } = render(<FlashCard front="Q" back="A" revealed={true} />);
    const front = container.querySelector(".flash-card__front");
    expect(front?.getAttribute("aria-hidden")).toBe("true");
  });

  it("hides the back face with aria-hidden when not revealed", () => {
    const { container } = render(<FlashCard front="Q" back="A" revealed={false} />);
    const back = container.querySelector(".flash-card__back");
    expect(back?.getAttribute("aria-hidden")).toBe("true");
  });
});
