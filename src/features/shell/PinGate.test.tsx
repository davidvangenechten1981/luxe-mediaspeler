import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import { useLock } from "../../state/lock";
import { PinGate } from "./PinGate";

beforeEach(() => useLock.setState({ enabled: true, unlocked: false }));

describe("PinGate", () => {
  it("laat pas ontgrendelen bij minstens 4 cijfers en filtert letters weg", () => {
    render(<PinGate />);
    const button = screen.getByRole("button", { name: "Ontgrendelen" });
    const input = screen.getByLabelText("PIN") as HTMLInputElement;
    expect(button).toBeDisabled();
    fireEvent.change(input, { target: { value: "12ab" } });
    expect(input.value).toBe("12");
    fireEvent.change(input, { target: { value: "1234" } });
    expect(button).toBeEnabled();
  });

  it("ontgrendelt met de juiste PIN", async () => {
    vi.spyOn(api, "pinVerify").mockResolvedValue(true);
    render(<PinGate />);
    fireEvent.change(screen.getByLabelText("PIN"), { target: { value: "4821" } });
    fireEvent.click(screen.getByRole("button", { name: "Ontgrendelen" }));
    await waitFor(() => expect(useLock.getState().unlocked).toBe(true));
  });

  it("meldt een foute PIN en blijft vergrendeld", async () => {
    vi.spyOn(api, "pinVerify").mockResolvedValue(false);
    render(<PinGate />);
    fireEvent.change(screen.getByLabelText("PIN"), { target: { value: "0000" } });
    fireEvent.click(screen.getByRole("button", { name: "Ontgrendelen" }));
    expect(await screen.findByText("Deze PIN klopt niet.")).toBeInTheDocument();
    expect(useLock.getState().unlocked).toBe(false);
  });

  it("toont de blokkademelding van de backend", async () => {
    vi.spyOn(api, "pinVerify").mockRejectedValue(new Error("Te veel foute pogingen. Probeer het over 30 seconden opnieuw."));
    render(<PinGate />);
    fireEvent.change(screen.getByLabelText("PIN"), { target: { value: "0000" } });
    fireEvent.click(screen.getByRole("button", { name: "Ontgrendelen" }));
    expect(await screen.findByText(/Te veel foute pogingen/)).toBeInTheDocument();
  });
});
