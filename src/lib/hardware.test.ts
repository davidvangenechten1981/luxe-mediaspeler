import { describe, expect, it } from "vitest";
import type { CapabilityOption } from "../api/types";
import { hwChoices, hwHint, threadsHint } from "./hardware";

const gpu = (id: string, label: string): CapabilityOption => ({ id, label, available: true, audio_only: false });

describe("hwChoices", () => {
  it("biedt zonder videokaart alleen Automatisch en Alleen processor", () => {
    const c = hwChoices([]);
    expect(c.map((x) => x.value)).toEqual(["auto", "off"]);
    expect(c[0]!.label).toMatch(/processor/);
  });
  it("noemt de eerste videokaart bij Automatisch en toont elke werkende kaart", () => {
    const c = hwChoices([gpu("nvenc", "NVIDIA (NVENC)"), gpu("qsv", "Intel (Quick Sync)")]);
    expect(c.map((x) => x.value)).toEqual(["auto", "off", "nvenc", "qsv"]);
    expect(c[0]!.label).toContain("NVIDIA");
  });
  it("negeert onbekende kaarten", () => {
    expect(hwChoices([gpu("x", "Onbekend")]).map((x) => x.value)).toEqual(["auto", "off"]);
  });
});

describe("hints", () => {
  it("legt uit wat er gebeurt", () => {
    expect(hwHint("off", [])).toMatch(/processor/);
    expect(hwHint("auto", [])).toMatch(/geen bruikbare videokaart/);
    expect(hwHint("nvenc", [gpu("nvenc", "NVIDIA")])).toMatch(/automatisch over/);
    expect(threadsHint(12)).toContain("12");
  });
});
