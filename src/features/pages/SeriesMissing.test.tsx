import { render, screen } from "@testing-library/react";
import type { Series } from "../../api/types";
import { SeriesCard } from "./SeriesPage";

const base: Series = { show: "Gat", episodes: [], watched: 0, next_media_id: null };

describe("ontbrekende afleveringen", () => {
  it("toont welke afleveringen ontbreken", () => {
    render(<SeriesCard series={{ ...base, missing: [{ season: 1, episode: 2 }, { season: 2, episode: 1 }] }} />);
    expect(screen.getByRole("note").textContent).toContain("S01E02, S02E01");
  });
  it("toont niets als er niets ontbreekt", () => {
    render(<SeriesCard series={{ ...base, missing: [] }} />);
    expect(screen.queryByRole("note")).toBeNull();
  });
});
