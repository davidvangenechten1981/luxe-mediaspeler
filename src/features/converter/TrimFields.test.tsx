import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { readTime, timeText, TrimFields } from "./TrimFields";

describe("readTime", () => {
  it("leest tijden, leeg en fouten", () => {
    expect(readTime("1:30")).toBe(90_000);
    expect(readTime("  ")).toBeNull();
    expect(readTime("abc")).toBeUndefined();
  });
  it("toont geen tijd als er niets is ingesteld", () => {
    expect(timeText(null)).toBe("");
    expect(timeText(65_000)).toBe("1:05");
  });
});

describe("TrimFields", () => {
  it("geeft begin en einde door in milliseconden", () => {
    const onChange = vi.fn();
    render(<TrimFields start={null} end={null} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText("Fragment: van"), { target: { value: "0:10" } });
    expect(onChange).toHaveBeenLastCalledWith(10_000, null);
    fireEvent.change(screen.getByLabelText("Fragment: tot"), { target: { value: "0:40" } });
    expect(onChange).toHaveBeenLastCalledWith(10_000, 40_000);
  });
  it("weigert een einde voor het begin en onleesbare tijden", () => {
    const onChange = vi.fn();
    render(<TrimFields start={null} end={null} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText("Fragment: van"), { target: { value: "1:00" } });
    onChange.mockClear();
    fireEvent.change(screen.getByLabelText("Fragment: tot"), { target: { value: "0:30" } });
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByText("Het einde moet na het begin liggen")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Fragment: tot"), { target: { value: "xx" } });
    expect(screen.getByText(/Typ een tijd/)).toBeInTheDocument();
  });
});
