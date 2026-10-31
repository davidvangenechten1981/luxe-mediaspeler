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
