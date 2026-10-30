import { render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { Lyrics } from "../../api/types";
import { currentLine, LyricsPanel, LyricsView } from "./LyricsPanel";

const synced: Lyrics = {
  synced: true,
  lines: [{ time_ms: 5000, text: "eerste" }, { time_ms: 10000, text: "tweede" }],
};

describe("currentLine", () => {
  it("volgt de positie", () => {
    expect(currentLine(synced.lines, 0)).toBe(-1);
    expect(currentLine(synced.lines, 5000)).toBe(0);
    expect(currentLine(synced.lines, 12000)).toBe(1);
  });
});

describe("LyricsView", () => {
  it("markeert alleen de huidige regel", () => {
    render(<LyricsView lyrics={synced} positionMs={11000} />);
    expect(screen.getByText("tweede")).toHaveAttribute("aria-current", "true");
    expect(screen.getByText("eerste")).not.toHaveAttribute("aria-current");
  });
  it("markeert niets bij tekst zonder tijden", () => {
    render(<LyricsView lyrics={{ synced: false, lines: [{ time_ms: null, text: "los" }] }} positionMs={99999} />);
    expect(screen.getByText("los")).not.toHaveAttribute("aria-current");
  });
});

describe("LyricsPanel", () => {
  it("toont niets zonder songtekst", async () => {
    const spy = vi.spyOn(api, "lyricsGet").mockResolvedValue(null);
    const { container } = render(<LyricsPanel location="/m/a.mp3" positionMs={0} />);
    await waitFor(() => expect(spy).toHaveBeenCalledWith("/m/a.mp3"));
    expect(container).toBeEmptyDOMElement();
  });
  it("toont de songtekst als die er is", async () => {
    vi.spyOn(api, "lyricsGet").mockResolvedValue(synced);
    render(<LyricsPanel location="/m/b.mp3" positionMs={6000} />);
    expect(await screen.findByText("eerste")).toBeInTheDocument();
  });
});
