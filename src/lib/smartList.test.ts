import { describe, expect, it } from "vitest";
import { describeQuery, EMPTY_RULES, hasRules, queryFromRules } from "./smartList";

describe("queryFromRules", () => {
  it("laat lege velden weg", () => {
    expect(queryFromRules(EMPTY_RULES)).toEqual({});
    expect(hasRules(queryFromRules(EMPTY_RULES))).toBe(false);
  });
  it("zet minuten, dagen en keuzes om", () => {
    const q = queryFromRules({ ...EMPTY_RULES, kind: "video", minMinutes: "120", addedDays: "30", played: "unplayed", favoritesOnly: true, genre: " Komedie " });
    expect(q).toEqual({ kind: "video", min_duration_ms: 7_200_000, added_within_days: 30, unplayed_only: true, favorite_only: true, genre: "Komedie" });
  });
  it("accepteert een komma als decimaalteken en negeert onzin of nul", () => {
    expect(queryFromRules({ ...EMPTY_RULES, maxMinutes: "1,5" }).max_duration_ms).toBe(90_000);
    const q = queryFromRules({ ...EMPTY_RULES, minMinutes: "abc", addedDays: "0", minHeight: "-5" });
    expect(q).toEqual({});
  });
  it("kiest niet tegelijk gespeeld en ongespeeld", () => {
    expect(queryFromRules({ ...EMPTY_RULES, played: "played" })).toEqual({ played_only: true });
  });
});

describe("describeQuery", () => {
  it("noemt alle regels in gewoon Nederlands", () => {
    const text = describeQuery({ kind: "video", min_duration_ms: 7_200_000, unplayed_only: true, added_within_days: 14, min_height: 1080 });
    expect(text).toBe("video's, langer dan 2 uur, minstens 1080p, toegevoegd in de laatste 14 dagen, nog niet afgespeeld");
  });
  it("zegt iets zinnigs zonder regels", () => {
    expect(describeQuery({})).toBe("Alles in de bibliotheek");
  });
});
