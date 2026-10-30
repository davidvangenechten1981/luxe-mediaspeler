import { act, fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { AfterActionDialog } from "./AfterActionDialog";

describe("AfterActionDialog", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("telt af en voert daarna één keer uit", () => {
    const onRun = vi.fn();
    render(<AfterActionDialog action="sleep" seconds={3} onRun={onRun} onCancel={() => undefined} />);
    expect(screen.getByRole("status")).toHaveTextContent("over 3 seconden");
    // elke seconde is een eigen render: de volgende timer wordt pas daarna gezet
    for (let i = 0; i < 2; i++) act(() => void vi.advanceTimersByTime(1000));
    expect(onRun).not.toHaveBeenCalled();
    act(() => void vi.advanceTimersByTime(1000));
    expect(onRun).toHaveBeenCalledTimes(1);
  });

  it("voert niets uit na annuleren", () => {
    const onRun = vi.fn();
    const onCancel = vi.fn();
    const { unmount } = render(<AfterActionDialog action="shutdown" seconds={5} onRun={onRun} onCancel={onCancel} />);
    fireEvent.click(screen.getByRole("button", { name: "Annuleren" }));
    expect(onCancel).toHaveBeenCalled();
    unmount();
    act(() => void vi.advanceTimersByTime(10_000));
    expect(onRun).not.toHaveBeenCalled();
  });

  it("voert direct uit met Nu uitvoeren", () => {
    const onRun = vi.fn();
    render(<AfterActionDialog action="shutdown" seconds={30} onRun={onRun} onCancel={() => undefined} />);
    fireEvent.click(screen.getByRole("button", { name: "Nu uitvoeren" }));
    expect(onRun).toHaveBeenCalled();
  });
});
