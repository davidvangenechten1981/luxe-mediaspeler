import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import { SmartListDialog } from "./SmartListsPage";

describe("SmartListDialog", () => {
  it("maakt pas een lijst met een naam én minstens één regel", () => {
    render(<SmartListDialog onClose={() => undefined} onSaved={() => undefined} />);
    const make = screen.getByRole("button", { name: "Maken" });
    expect(make).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Naam"), { target: { value: "Lang" } });
    expect(make).toBeDisabled(); // nog geen regel
    fireEvent.change(screen.getByLabelText("Langer dan (minuten)"), { target: { value: "120" } });
    expect(make).toBeEnabled();
    expect(screen.getByRole("status")).toHaveTextContent("langer dan 2 uur");
  });

  it("bewaart de regels als zoekopdracht", async () => {
    const create = vi.spyOn(api, "collectionCreate").mockResolvedValue(1);
    const saved = vi.fn();
    render(<SmartListDialog onClose={() => undefined} onSaved={saved} />);
    fireEvent.change(screen.getByLabelText("Naam"), { target: { value: "Nog te kijken" } });
    fireEvent.change(screen.getByLabelText("Afgespeeld"), { target: { value: "unplayed" } });
    fireEvent.change(screen.getByLabelText("Soort"), { target: { value: "video" } });
    fireEvent.click(screen.getByRole("button", { name: "Maken" }));
    await waitFor(() => expect(saved).toHaveBeenCalled());
    expect(create).toHaveBeenCalledWith("Nog te kijken", { kind: "video", unplayed_only: true });
  });
});
