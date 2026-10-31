import { describe, expect, it } from "vitest";
import { applyAppearance, DEFAULT_SETTINGS } from "./settings";

describe("hoog contrast", () => {
  it("zet en haalt het contrastattribuut weg", () => {
    applyAppearance({ ...DEFAULT_SETTINGS, high_contrast: true });
    expect(document.documentElement.dataset.contrast).toBe("high");
    applyAppearance({ ...DEFAULT_SETTINGS, high_contrast: false });
    expect(document.documentElement.dataset.contrast).toBe("normal");
  });
});

describe("tv-modus", () => {
  it("zet het tv-attribuut en vergroot de letters", () => {
    applyAppearance({ ...DEFAULT_SETTINGS, tv_mode: true });
    expect(document.documentElement.dataset.tv).toBe("true");
    expect(document.documentElement.style.fontSize).toBe("22.4px");
    applyAppearance({ ...DEFAULT_SETTINGS, tv_mode: false });
    expect(document.documentElement.dataset.tv).toBe("false");
    expect(document.documentElement.style.fontSize).toBe("16px");
  });
});
