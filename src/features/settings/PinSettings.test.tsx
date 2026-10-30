import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import { useLock } from "../../state/lock";
import { PinDialog, pinProblem, PinSettings } from "./PinSettings";

beforeEach(() => useLock.setState({ enabled: false, unlocked: true }));

describe("pinProblem", () => {
  it("controleert lengte, herhaling en de huidige PIN", () => {
    expect(pinProblem("12", "12", false, "", false)).toMatch(/4 tot 8/);
    expect(pinProblem("1234", "1235", false, "", false)).toMatch(/niet gelijk/);
    expect(pinProblem("1234", "1234", false, "", false)).toBeNull();
    expect(pinProblem("1234", "1234", true, "", false)).toMatch(/huidige/);
    expect(pinProblem("", "", true, "4821", true)).toBeNull();
    expect(pinProblem("", "", true, "", true)).toMatch(/huidige/);
  });
});

describe("PinDialog", () => {
  it("stelt een nieuwe PIN in en schakelt de vergrendeling in", async () => {
    const set = vi.spyOn(api, "pinSet").mockResolvedValue(null);
    render(<PinDialog mode="set" onClose={() => undefined} />);
    const save = screen.getByRole("button", { name: "Opslaan" });
    expect(save).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Nieuwe PIN"), { target: { value: "4821" } });
    fireEvent.change(screen.getByLabelText("Nieuwe PIN nog eens"), { target: { value: "4821" } });
    fireEvent.click(save);
    await waitFor(() => expect(set).toHaveBeenCalledWith("4821", null));
    expect(useLock.getState().enabled).toBe(true);
  });

  it("vraagt de huidige PIN bij verwijderen", async () => {
    useLock.setState({ enabled: true, unlocked: true });
    const set = vi.spyOn(api, "pinSet").mockResolvedValue(null);
    render(<PinDialog mode="remove" onClose={() => undefined} />);
    const del = screen.getByRole("button", { name: "Verwijderen" });
    expect(del).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Huidige PIN"), { target: { value: "4821" } });
    fireEvent.click(del);
    await waitFor(() => expect(set).toHaveBeenCalledWith(null, "4821"));
    expect(useLock.getState().enabled).toBe(false);
  });
});

describe("PinSettings", () => {
  it("biedt vergrendelen en verwijderen alleen aan met een PIN", () => {
    const { rerender } = render(<PinSettings />);
    expect(screen.getByRole("button", { name: "PIN instellen…" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Nu vergrendelen" })).toBeNull();
    useLock.setState({ enabled: true, unlocked: true });
    rerender(<PinSettings />);
    fireEvent.click(screen.getByRole("button", { name: "Nu vergrendelen" }));
    expect(useLock.getState().unlocked).toBe(false);
  });
});
