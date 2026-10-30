import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { Media, Series, SeriesEpisode } from "../../api/types";
import { bySeason, SeriesCard } from "./SeriesPage";

const media = (id: number, path: string): Media =>
  ({ id, path, title: null, kind: "video", duration_ms: 1000 }) as unknown as Media;
const ep = (id: number, season: number, episode: number, watched: boolean): SeriesEpisode => ({
  media: media(id, `/s/Show.S0${season}E0${episode}.mkv`), season, episode, watched,
});

describe("bySeason", () => {
  it("groepeert per seizoen op volgorde", () => {
    const g = bySeason([ep(3, 2, 1, false), ep(1, 1, 1, true), ep(2, 1, 2, false)]);
    expect(g.map(([s, e]) => [s, e.length])).toEqual([[1, 2], [2, 1]]);
  });
});

describe("SeriesCard", () => {
  const series: Series = { show: "Show", episodes: [ep(1, 1, 1, true), ep(2, 1, 2, false)], watched: 1, next_media_id: 2 };

  it("toont de voortgang en de volgende aflevering", () => {
    render(<SeriesCard series={series} />);
    expect(screen.getByText("1 van 2 afleveringen gezien")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Volgende: S01E02/ })).toBeInTheDocument();
  });

  it("zegt dat alles gezien is zonder volgende aflevering", () => {
    render(<SeriesCard series={{ ...series, watched: 2, next_media_id: null, episodes: series.episodes.map((e) => ({ ...e, watched: true })) }} />);
    expect(screen.getByText("Alles gezien")).toBeInTheDocument();
  });

  it("zet een aflevering handmatig op gezien of niet gezien", async () => {
    const spy = vi.spyOn(api, "markCompleted").mockResolvedValue(null);
    const changed = vi.fn();
    render(<SeriesCard series={series} onChanged={changed} />);
    fireEvent.click(screen.getByRole("button", { name: "Markeer als gezien" }));
    await waitFor(() => expect(changed).toHaveBeenCalled());
    expect(spy).toHaveBeenCalledWith("/s/Show.S01E02.mkv", true);
    fireEvent.click(screen.getByRole("button", { name: "Markeer als niet gezien" }));
    await waitFor(() => expect(spy).toHaveBeenCalledWith("/s/Show.S01E01.mkv", false));
  });
});
