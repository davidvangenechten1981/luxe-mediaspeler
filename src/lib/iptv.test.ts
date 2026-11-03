import { describe, expect, it } from "vitest";
import { blockPlacement, canCatchUp, floorToHour, formatClock, initials, nextDigitBuffer, programmeProgress } from "./iptv";

describe("iptv-hulpfuncties", () => {
  it("formatClock geeft uu:mm in lokale tijd", () => {
    const t = new Date(2026, 10, 3, 9, 5).getTime() / 1000;
    expect(formatClock(t)).toBe("09:05");
  });

  it("programmeProgress blijft tussen 0 en 1", () => {
    const p = { start: 100, stop: 200 };
    expect(programmeProgress(p, 50)).toBe(0);
    expect(programmeProgress(p, 150)).toBe(0.5);
    expect(programmeProgress(p, 500)).toBe(1);
    expect(programmeProgress({ start: 5, stop: 5 }, 5)).toBe(0);
  });

  it("initials vervangt het logo door een of twee letters", () => {
    expect(initials("NPO 1 HD")).toBe("N1");
    expect(initials("Klassiek")).toBe("KL");
    expect(initials("  ")).toBe("?");
    expect(initials("één")).toBe("ÉÉ");
  });

  it("nextDigitBuffer verzamelt maximaal vijf cijfers en negeert andere toetsen", () => {
    expect(nextDigitBuffer("", "4")).toBe("4");
    expect(nextDigitBuffer("42", "7")).toBe("427");
    expect(nextDigitBuffer("12345", "6")).toBeNull();
    expect(nextDigitBuffer("1", "a")).toBeNull();
    expect(nextDigitBuffer("1", "10")).toBeNull();
  });

  it("blockPlacement knipt blokken af aan het venster en laat buitenstaanders weg", () => {
    expect(blockPlacement({ start: 0, stop: 100 }, 0, 400)).toEqual({ left: 0, width: 25 });
    expect(blockPlacement({ start: -100, stop: 100 }, 0, 400)).toEqual({ left: 0, width: 25 });
    expect(blockPlacement({ start: 300, stop: 900 }, 0, 400)).toEqual({ left: 75, width: 25 });
    expect(blockPlacement({ start: 400, stop: 500 }, 0, 400)).toBeNull();
    expect(blockPlacement({ start: -50, stop: 0 }, 0, 400)).toBeNull();
    expect(blockPlacement({ start: 0, stop: 10 }, 5, 5)).toBeNull();
  });

  it("floorToHour rondt af naar het hele uur", () => {
    expect(floorToHour(3600 * 5 + 1234)).toBe(3600 * 5);
  });

  it("canCatchUp eist een voorbij programma binnen het archief", () => {
    const now = 1_000_000;
    expect(canCatchUp({ start: now - 7200, stop: now - 3600 }, 3, true, now)).toBe(true);
    expect(canCatchUp({ start: now - 7200, stop: now + 60 }, 3, true, now)).toBe(false); // loopt nog
    expect(canCatchUp({ start: now - 5 * 86_400, stop: now - 5 * 86_400 + 3600 }, 3, true, now)).toBe(false); // te oud
    expect(canCatchUp({ start: now - 7200, stop: now - 3600 }, null, true, now)).toBe(true); // archief onbekend
    expect(canCatchUp({ start: now - 7200, stop: now - 3600 }, 3, false, now)).toBe(false); // geen terugkijken
  });
});
