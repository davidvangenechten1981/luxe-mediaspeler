import { render, screen } from "@testing-library/react";
import { NextUpCard, secondsLeft } from "./NextUpCard";

describe("secondsLeft", () => {
  it("rondt omhoog zodat je nooit '0 s' ziet terwijl het nog loopt", () => {
    expect(secondsLeft(10_000)).toBe(10);
    expect(secondsLeft(9_001)).toBe(10);
    expect(secondsLeft(250)).toBe(1);
    expect(secondsLeft(0)).toBe(0);
  });
});

describe("NextUpCard", () => {
  it("toont de titel en de resterende tijd", () => {
    render(<NextUpCard title="Aflevering 2" remainingMs={7_300} onNow={() => undefined} onCancel={() => undefined} />);
    expect(screen.getByText("Aflevering 2")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("begint over 8 s");
  });
  it("heeft knoppen om direct door te gaan of te annuleren", () => {
    const onNow = vi.fn();
    const onCancel = vi.fn();
    render(<NextUpCard title="Aflevering 2" remainingMs={5_000} onNow={onNow} onCancel={onCancel} />);
    screen.getByRole("button", { name: "Nu afspelen" }).click();
    screen.getByRole("button", { name: "Annuleren" }).click();
    expect(onNow).toHaveBeenCalledTimes(1);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
