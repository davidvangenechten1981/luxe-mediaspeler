import { describe, expect, it } from "vitest";
import { pickNext, type Box } from "./tvNav";

const b = (id: string, x: number, y: number, w = 100, h = 40): Box => ({ id, x, y, w, h });

describe("pickNext", () => {
  const items = [b("a", 0, 0), b("b", 200, 0), b("c", 0, 100), b("d", 200, 100)];
  it("kiest de dichtstbijzijnde in de gevraagde richting", () => {
    expect(pickNext(items, "a", "right")).toBe("b");
    expect(pickNext(items, "a", "down")).toBe("c");
    expect(pickNext(items, "d", "left")).toBe("c");
    expect(pickNext(items, "d", "up")).toBe("b");
  });
  it("blijft staan als er niets in die richting is", () => {
    expect(pickNext(items, "a", "left")).toBeNull();
    expect(pickNext(items, "a", "up")).toBeNull();
  });
  it("geeft voorkeur aan wat beter uitgelijnd is", () => {
    const grid = [b("a", 0, 0), b("near-off", 120, 80), b("far-aligned", 300, 0)];
    expect(pickNext(grid, "a", "right")).toBe("far-aligned");
  });
  it("kent een onbekende start niet", () => {
    expect(pickNext(items, "zzz", "down")).toBeNull();
  });
});
