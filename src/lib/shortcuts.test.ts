import { describe, expect, it } from "vitest";
import { actionFor, eventToAccelerator } from "./shortcuts";

const ev = (key: string, mods: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }> = {}) => ({ key, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });

describe("sneltoetsen", () => {
  it("zet toetsaanslagen om naar dezelfde schrijfwijze als de backend", () => {
    expect(eventToAccelerator(ev(" "))).toBe("Space");
    expect(eventToAccelerator(ev("o", { ctrlKey: true }))).toBe("Ctrl+O");
    expect(eventToAccelerator(ev("S", { shiftKey: true }))).toBe("Shift+S");
    expect(eventToAccelerator(ev("ArrowRight", { shiftKey: true }))).toBe("Shift+ArrowRight");
    expect(eventToAccelerator(ev("F11"))).toBe("F11");
    expect(eventToAccelerator(ev(",", { ctrlKey: true }))).toBe("Ctrl+,");
    expect(eventToAccelerator(ev("]"))).toBe("]");
    expect(eventToAccelerator(ev("l", { ctrlKey: true, shiftKey: true, altKey: true }))).toBe("Ctrl+Alt+Shift+L");
  });

  it("negeert losse modifier-toetsen", () => {
    for (const k of ["Control", "Shift", "Alt", "Meta"]) expect(eventToAccelerator(ev(k))).toBeNull();
  });

  it("vindt de actie bij een toets en geeft null bij onbekend", () => {
    const map = { play_pause: "Space", fullscreen: "F", open_file: "Ctrl+O", screenshot: "Shift+S" };
    expect(actionFor(ev(" "), map)).toBe("play_pause");
    expect(actionFor(ev("f"), map)).toBe("fullscreen");
    expect(actionFor(ev("o", { ctrlKey: true }), map)).toBe("open_file");
    expect(actionFor(ev("S", { shiftKey: true }), map)).toBe("screenshot");
    expect(actionFor(ev("q"), map)).toBeNull();
    expect(actionFor(ev("Shift", { shiftKey: true }), map)).toBeNull();
  });
});
