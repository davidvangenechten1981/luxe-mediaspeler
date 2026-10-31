import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { Media } from "../../api/types";
import { MediaBrowser } from "./MediaBrowser";

const film = (id: number): Media => ({ id, path: `/m/film${id}.mkv`, title: `Film ${id}`, kind: "video", size: 1000, duration_ms: 1000, favorite: false, missing: false } as unknown as Media);

describe("MediaBrowser: posterweergave", () => {
  beforeEach(() => {
    vi.spyOn(api, "libraryQuery").mockResolvedValue([film(1), film(2)]);
    vi.spyOn(api, "libraryCount").mockResolvedValue(2);
    vi.spyOn(api, "coverArt").mockResolvedValue(null);
    vi.spyOn(api, "thumbnail").mockRejectedValue(new Error("geen"));
  });

  it("wisselt tussen raster, posters en lijst", async () => {
    render(<MediaBrowser title="Films" base={{ kind: "video" }} emptyTitle="Leeg" emptyText="Niets" />);
    const list = await screen.findByRole("list");
    expect(list).toHaveClass("grid");
    expect(list).not.toHaveClass("grid--posters");
    fireEvent.click(screen.getByRole("button", { name: "Posterweergave" }));
    await waitFor(() => expect(screen.getByRole("list")).toHaveClass("grid--posters"));
    fireEvent.click(screen.getByRole("button", { name: "Rasterweergave" }));
    await waitFor(() => expect(screen.getByRole("list")).not.toHaveClass("grid--posters"));
  });

  it("biedt geen posterweergave voor muziek", async () => {
    render(<MediaBrowser title="Muziek" base={{ kind: "audio" }} music emptyTitle="Leeg" emptyText="Niets" />);
    await screen.findByRole("list");
    expect(screen.queryByRole("button", { name: "Posterweergave" })).toBeNull();
  });

  it("heeft staande kaarten (2:3) in de opmaak", () => {
    const css = readFileSync("src/styles/shell.css", "utf8");
    expect(css).toMatch(/\.grid--posters \.card-media__thumb\s*\{[^}]*aspect-ratio:\s*2\s*\/\s*3/);
  });
});
