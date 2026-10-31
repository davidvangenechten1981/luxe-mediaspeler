import { render, screen } from "@testing-library/react";
import { StatsStrip, statsSummary } from "./StatsStrip";

describe("statsSummary", () => {
  it("telt enkelvoud en meervoud goed", () => {
    expect(statsSummary({ video_count: 1, audio_count: 2, total_duration_ms: 3_780_000, total_plays: 5 })).toEqual([
      "1 video", "2 nummers", "1 u 03 min in totaal", "5 keer afgespeeld",
    ]);
  });
});

describe("StatsStrip", () => {
  it("toont niets zolang er geen cijfers zijn", () => {
    const { container } = render(<StatsStrip stats={null} />);
    expect(container).toBeEmptyDOMElement();
  });
  it("toont de samenvatting", () => {
    render(<StatsStrip stats={{ video_count: 0, audio_count: 0, total_duration_ms: 0, total_plays: 0 }} />);
    expect(screen.getByRole("group", { name: "Bibliotheekstatistiek" })).toHaveTextContent("0 video's");
  });
});
