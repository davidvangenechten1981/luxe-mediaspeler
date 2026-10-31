import { describe, expect, it } from "vitest";
import { basename, clamp, dirname, formatBitrate, formatBytes, formatDuration, formatEta, formatRate, formatRelative, formatTime, isUrl, mediaTitle, resolutionLabel, stem } from "./format";

describe("opmaak", () => {
  it("formatTime", () => {
    expect(formatTime(0)).toBe("0:00");
    expect(formatTime(65_000)).toBe("1:05");
    expect(formatTime(3_725_000)).toBe("1:02:05");
    expect(formatTime(-5)).toBe("0:00");
    expect(formatTime(NaN)).toBe("0:00");
  });
  it("formatDuration", () => {
    expect(formatDuration(0)).toBe("–");
    expect(formatDuration(45_000)).toBe("45 s");
    expect(formatDuration(720_000)).toBe("12 min");
    expect(formatDuration(3_720_000)).toBe("1 u 02 min");
  });
  it("formatBytes en bitrate", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1500)).toBe("1,50 kB");
    expect(formatBytes(5_000_000_000)).toBe("5,00 GB");
    expect(formatBytes(-1)).toBe("–");
    expect(formatBitrate(null)).toBe("–");
    expect(formatBitrate(8_000_000)).toBe("8,0 Mbit/s");
    expect(formatBitrate(192_000)).toBe("192 kbit/s");
  });
  it("relatieve tijd", () => {
    const now = 1_700_000_000_000;
    expect(formatRelative(1_700_000_000 - 10, now)).toBe("zojuist");
    expect(formatRelative(1_700_000_000 - 600, now)).toBe("10 min geleden");
    expect(formatRelative(1_700_000_000 - 7200, now)).toBe("2 u geleden");
    expect(formatRelative(1_700_000_000 - 3 * 86_400, now)).toBe("3 d geleden");
  });
  it("eta en snelheid", () => {
    expect(formatEta(null)).toBe("");
    expect(formatEta(45)).toBe("45 s");
    expect(formatEta(125)).toBe("2 min 05 s");
    expect(formatEta(3_900)).toBe("1 u 05 min");
    expect(formatRate(1.25)).toBe("1,25×");
  });
  it("paden", () => {
    expect(basename("C:\\Video's\\film één.mkv")).toBe("film één.mkv");
    expect(basename("/a/b/c.mp3")).toBe("c.mp3");
    expect(stem("/a/b/c.tar.gz")).toBe("c.tar");
    expect(stem("/a/.verborgen")).toBe(".verborgen");
    expect(dirname("C:\\a\\b.mkv")).toBe("C:\\a");
    expect(isUrl("https://x.nl/a")).toBe(true);
    expect(isUrl("C:\\x")).toBe(false);
  });
  it("overig", () => {
    expect(clamp(5, 0, 3)).toBe(3);
    expect(resolutionLabel(2160)).toBe("4K");
    expect(resolutionLabel(480)).toBe("480p");
    expect(resolutionLabel(null)).toBeNull();
    expect(mediaTitle({ title: "  ", path: "/m/Naam.mkv" })).toBe("Naam");
    expect(mediaTitle({ title: "Titel", path: "/m/x.mkv" })).toBe("Titel");
  });
});
