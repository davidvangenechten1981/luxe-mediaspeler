import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { Media, TagFields } from "../../api/types";
import { buildUpdate, TagEditDialog } from "./TagEditDialog";

const orig: TagFields = { title: "Oud", artist: "Band", album: null, genre: null, year: 2000, track: 1 };
const song = { id: 1, path: "/m/lied.mp3", kind: "audio" } as unknown as Media;

describe("buildUpdate", () => {
  it("stuurt alleen gewijzigde velden mee", () => {
    const f = { title: "Nieuw", artist: "Band", album: "", genre: "", year: "2000", track: "1" };
    expect(buildUpdate(orig, f)).toEqual({ title: "Nieuw" });
  });
  it("laat een leeg veld toe om iets te wissen en zet getallen om", () => {
    const f = { title: "Oud", artist: "", album: "", genre: "", year: "2021", track: "" };
    expect(buildUpdate(orig, f)).toEqual({ artist: "", year: 2021 });
  });
});

describe("TagEditDialog", () => {
  it("bewaart pas na een wijziging en meldt het", async () => {
    vi.spyOn(api, "tagsGet").mockResolvedValue(orig);
    const set = vi.spyOn(api, "tagsSet").mockResolvedValue(null);
    const saved = vi.fn();
    render(<TagEditDialog media={song} onClose={() => undefined} onSaved={saved} />);
    const save = await screen.findByRole("button", { name: "Opslaan met reservekopie" });
    expect(save).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Titel"), { target: { value: "Nieuw" } });
    expect(save).toBeEnabled();
    fireEvent.click(save);
    await waitFor(() => expect(saved).toHaveBeenCalled());
    expect(set).toHaveBeenCalledWith("/m/lied.mp3", { title: "Nieuw" });
  });
});
