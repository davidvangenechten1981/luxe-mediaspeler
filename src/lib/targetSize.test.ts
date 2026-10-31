import { describe, expect, it } from "vitest";
import { videoBitrateForSize } from "./targetSize";

describe("videoBitrateForSize", () => {
  it("rekent de videobitrate uit voor een doelgrootte", () => {
    // 700 MB in 3600 s met 128 kbit/s audio: 700*8388.608/3600 = 1631 totaal -> ~1503 video (met 2% marge)
    const v = videoBitrateForSize(700, 3_600_000, 128)!;
    expect(v).toBeGreaterThan(1400);
    expect(v).toBeLessThan(1520);
  });
  it("geeft niets bij onbekende duur of ongeldige grootte", () => {
    expect(videoBitrateForSize(700, 0, 128)).toBeNull();
    expect(videoBitrateForSize(0, 60_000, 128)).toBeNull();
    expect(videoBitrateForSize(Number.NaN, 60_000, 128)).toBeNull();
  });
  it("geeft niets als de audio alleen al te groot is voor het doel", () => {
    expect(videoBitrateForSize(1, 3_600_000, 160)).toBeNull();
  });
});
