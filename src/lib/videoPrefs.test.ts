import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../api/client";
import { applyFilePrefs, markDirty, nonNeutral, saveFilePrefs } from "./videoPrefs";

beforeEach(() => vi.restoreAllMocks());

describe("videoPrefs", () => {
  it("bewaart alleen afwijkende waarden", async () => {
    expect(nonNeutral({ brightness: 1, contrast: 1.2, hue: 0 })).toEqual({ contrast: 1.2 });
    const set = vi.spyOn(api, "videoPrefsSet").mockResolvedValue(null);
    await saveFilePrefs("C:\\a.mkv", { brightness: 1.1, gamma: 1 });
    expect(set).toHaveBeenCalledWith("C:\\a.mkv", { brightness: 1.1 });
  });
  it("past bewaarde waarden toe en zet daarna alles terug voor een bestand zonder keuze", async () => {
    const get = vi.spyOn(api, "videoPrefsGet").mockResolvedValueOnce({ brightness: 1.4 }).mockResolvedValueOnce(null);
    const adj = vi.spyOn(api, "videoAdjust").mockResolvedValue(null);
    await applyFilePrefs("a");
    expect(adj).toHaveBeenCalledWith("brightness", 1.4);
    adj.mockClear();
    await applyFilePrefs("b");
    expect(get).toHaveBeenCalledTimes(2);
    expect(adj).toHaveBeenCalledWith("brightness", 1);
    adj.mockClear();
    vi.spyOn(api, "videoPrefsGet").mockResolvedValue(null);
    await applyFilePrefs("c");
    expect(adj).not.toHaveBeenCalled(); // niets aangepast: niets te herstellen
  });
  it("herstelt ook na een handmatige aanpassing in de studio", async () => {
    markDirty();
    vi.spyOn(api, "videoPrefsGet").mockResolvedValue(null);
    const adj = vi.spyOn(api, "videoAdjust").mockResolvedValue(null);
    await applyFilePrefs("d");
    expect(adj).toHaveBeenCalledTimes(5);
  });
});
