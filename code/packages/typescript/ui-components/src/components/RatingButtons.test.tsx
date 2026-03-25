import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { RatingButtons } from "./RatingButtons.js";
import type { Rating } from "./RatingButtons.js";

describe("RatingButtons", () => {
  it("renders all four rating buttons", () => {
    render(<RatingButtons onRate={() => {}} />);
    expect(screen.getByText("Again")).toBeTruthy();
    expect(screen.getByText("Hard")).toBeTruthy();
    expect(screen.getByText("Good")).toBeTruthy();
    expect(screen.getByText("Easy")).toBeTruthy();
  });

  it("calls onRate with 'again' when Again is clicked", () => {
    const onRate = vi.fn();
    render(<RatingButtons onRate={onRate} />);
    fireEvent.click(screen.getByText("Again"));
    expect(onRate).toHaveBeenCalledWith("again");
  });

  it("calls onRate with 'hard' when Hard is clicked", () => {
    const onRate = vi.fn();
    render(<RatingButtons onRate={onRate} />);
    fireEvent.click(screen.getByText("Hard"));
    expect(onRate).toHaveBeenCalledWith("hard");
  });

  it("calls onRate with 'good' when Good is clicked", () => {
    const onRate = vi.fn();
    render(<RatingButtons onRate={onRate} />);
    fireEvent.click(screen.getByText("Good"));
    expect(onRate).toHaveBeenCalledWith("good");
  });

  it("calls onRate with 'easy' when Easy is clicked", () => {
    const onRate = vi.fn();
    render(<RatingButtons onRate={onRate} />);
    fireEvent.click(screen.getByText("Easy"));
    expect(onRate).toHaveBeenCalledWith("easy");
  });

  it("disables all buttons when disabled=true", () => {
    render(<RatingButtons onRate={() => {}} disabled={true} />);
    const buttons = screen.getAllByRole("button");
    buttons.forEach((btn) => {
      expect((btn as HTMLButtonElement).disabled).toBe(true);
    });
  });

  it("does not call onRate when disabled buttons are clicked", () => {
    const onRate = vi.fn();
    render(<RatingButtons onRate={onRate} disabled={true} />);
    fireEvent.click(screen.getByText("Good"));
    expect(onRate).not.toHaveBeenCalled();
  });

  it("has role=group with aria-label", () => {
    render(<RatingButtons onRate={() => {}} />);
    const group = screen.getByRole("group");
    expect(group.getAttribute("aria-label")).toBe("Rate your recall");
  });

  it("accepts a custom className", () => {
    const { container } = render(
      <RatingButtons onRate={() => {}} className="custom-ratings" />,
    );
    expect(container.querySelector(".custom-ratings")).toBeTruthy();
  });

  it("each button has a title attribute for accessibility", () => {
    render(<RatingButtons onRate={() => {}} />);
    const buttons = screen.getAllByRole("button");
    buttons.forEach((btn) => {
      expect(btn.getAttribute("title")).toBeTruthy();
    });
  });

  it("renders exactly 4 buttons", () => {
    render(<RatingButtons onRate={() => {}} />);
    expect(screen.getAllByRole("button")).toHaveLength(4);
  });

  it("each button has the correct modifier class", () => {
    const { container } = render(<RatingButtons onRate={() => {}} />);
    expect(container.querySelector(".rating-buttons__btn--again")).toBeTruthy();
    expect(container.querySelector(".rating-buttons__btn--hard")).toBeTruthy();
    expect(container.querySelector(".rating-buttons__btn--good")).toBeTruthy();
    expect(container.querySelector(".rating-buttons__btn--easy")).toBeTruthy();
  });

  it("onRate receives Rating type values only", () => {
    const received: Rating[] = [];
    render(<RatingButtons onRate={(r) => received.push(r)} />);
    fireEvent.click(screen.getByText("Again"));
    fireEvent.click(screen.getByText("Hard"));
    fireEvent.click(screen.getByText("Good"));
    fireEvent.click(screen.getByText("Easy"));
    expect(received).toEqual(["again", "hard", "good", "easy"]);
  });
});
