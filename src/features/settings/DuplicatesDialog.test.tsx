import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { DuplicateGroup, Media } from "../../api/types";
import { DuplicatesDialog } from "./SettingsPage";

const m = (id: number, path: string): Media => ({ id, path, title: null, kind: "video", size: 1000 }) as unknown as Media;
const group: DuplicateGroup = { keep_id: 1, items: [m(1, "/a/film.mkv"), m(2, "/b/kopie/film.mkv"), m(3, "/c/film.mkv")] };

describe("DuplicatesDialog", () => {
  it("markeert het aanbevolen bestand", async () => {
    vi.spyOn(api, "libraryDuplicates").mockResolvedValue([group]);
    render(<DuplicatesDialog onClose={() => undefined} />);
    expect(await screen.findAllByText("Aanbevolen")).toHaveLength(1);
  });
  it("haalt alle andere bestanden uit de bibliotheek met één klik, nooit het aanbevolen bestand", async () => {
    vi.spyOn(api, "libraryDuplicates").mockResolvedValue([group]);
    const remove = vi.spyOn(api, "libraryRemoveMedia").mockResolvedValue(null);
    render(<DuplicatesDialog onClose={() => undefined} />);
    fireEvent.click(await screen.findByRole("button", { name: "Alleen het aanbevolen bestand bewaren" }));
    await waitFor(() => expect(remove).toHaveBeenCalledTimes(2));
    expect(remove.mock.calls.map((c) => c[0]).sort()).toEqual([2, 3]);
  });
  it("meldt als er geen dubbelen zijn", async () => {
    vi.spyOn(api, "libraryDuplicates").mockResolvedValue([]);
    render(<DuplicatesDialog onClose={() => undefined} />);
    expect(await screen.findByText(/Geen dubbele bestanden/)).toBeInTheDocument();
  });
});
