import type { Chapter } from "../api/types";
import { chapterTarget, parseTimeInput, sleepStep } from "./playback";

describe("parseTimeInput", () => {
  it("leest uren, minuten en seconden", () => {
    expect(parseTimeInput("1:23:45")).toBe(5_025_000);
    expect(parseTimeInput("12:05")).toBe(725_000);
    expect(parseTimeInput("0:07")).toBe(7_000);
  });
  it("leest een los getal als seconden", () => {
    expect(parseTimeInput("90")).toBe(90_000);
    expect(parseTimeInput("  45 ")).toBe(45_000);
  });
  it("weigert onzin, negatieve of onmogelijke tijden", () => {
    for (const bad of ["", "abc", "-5", "1:60", "1:2:3:4", "1::2", "1:75:00"]) expect(parseTimeInput(bad), bad).toBeNull();
  });
});

const chapters: Chapter[] = [
  { start_ms: 0, end_ms: 60_000, title: "A" },
  { start_ms: 60_000, end_ms: 120_000, title: "B" },
  { start_ms: 120_000, end_ms: 180_000, title: "C" },
];

describe("chapterTarget", () => {
  it("gaat naar het volgende hoofdstuk", () => {
    expect(chapterTarget(chapters, 10_000, "next")).toBe(60_000);
    expect(chapterTarget(chapters, 60_000, "next")).toBe(120_000);
  });
  it("geeft niets na het laatste hoofdstuk", () => {
    expect(chapterTarget(chapters, 150_000, "next")).toBeNull();
  });
  it("springt bij 'vorige' eerst naar het begin van het huidige hoofdstuk, daarna naar het vorige", () => {
    expect(chapterTarget(chapters, 90_000, "previous")).toBe(60_000);
    expect(chapterTarget(chapters, 61_000, "previous")).toBe(0); // binnen 3 s van het begin: echt naar het vorige
    expect(chapterTarget(chapters, 1_000, "previous")).toBe(0);
  });
  it("doet niets zonder hoofdstukken", () => {
    expect(chapterTarget([], 1_000, "next")).toBeNull();
    expect(chapterTarget([], 1_000, "previous")).toBeNull();
  });
});

describe("sleepStep", () => {
  const end = 100_000;
  it("wacht tot de laatste seconden", () => {
    expect(sleepStep(10_000, end, 8_000)).toEqual({ phase: "waiting", factor: 1 });
  });
  it("zet het volume zachter tijdens het uitfaden", () => {
    const mid = sleepStep(96_000, end, 8_000);
    expect(mid.phase).toBe("fading");
    expect(mid.factor).toBeCloseTo(0.5, 2);
  });
  it("is klaar op het einde", () => {
    expect(sleepStep(100_000, end, 8_000)).toEqual({ phase: "done", factor: 0 });
    expect(sleepStep(150_000, end, 8_000)).toEqual({ phase: "done", factor: 0 });
  });
});

import { sleepTick } from "./playback";

describe("sleepTick", () => {
  it("doet niets zolang er genoeg tijd over is", () => {
    const r = sleepTick({ endsAt: 100_000, baseVolume: null }, 10_000, 80);
    expect(r.commands).toEqual([]);
    expect(r.next).toEqual({ endsAt: 100_000, baseVolume: null });
  });
  it("onthoudt het volume bij het begin van het uitfaden en zet het zachter", () => {
    const r = sleepTick({ endsAt: 100_000, baseVolume: null }, 96_000, 80);
    expect(r.next?.baseVolume).toBe(80);
    expect(r.commands).toEqual([{ type: "volume", percent: 40 }]);
    // volgende tik gebruikt het onthouden volume, niet het al verlaagde
    const r2 = sleepTick(r.next!, 98_000, 40);
    expect(r2.commands).toEqual([{ type: "volume", percent: 20 }]);
  });
  it("stopt aan het einde en zet het volume terug voor de volgende keer", () => {
    const r = sleepTick({ endsAt: 100_000, baseVolume: 80 }, 100_500, 2);
    expect(r.commands).toEqual([{ type: "stop" }, { type: "volume", percent: 80 }]);
    expect(r.next).toBeNull();
  });
  it("zet bij stoppen zonder uitfaden het huidige volume niet vreemd terug", () => {
    const r = sleepTick({ endsAt: 100_000, baseVolume: null }, 120_000, 70);
    expect(r.commands).toEqual([{ type: "stop" }, { type: "volume", percent: 70 }]);
  });
});
