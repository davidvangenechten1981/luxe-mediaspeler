import { describe, expect, it } from "vitest";
import namesRaw from "../../crates/vu-app/src/api_names.txt?raw";
import { api, setCommandTransport } from "./client";

// Elk verzoektype uit de Rust-enum moet door de client worden gebruikt.
describe("API-dekking", () => {
  it("de client stuurt elk Rust-verzoektype minstens één keer", async () => {
    const names = namesRaw.split(/\r?\n/).map((n) => n.trim()).filter(Boolean);
    const sent = new Set<string>();
    setCommandTransport(async (_name, args) => {
      sent.add((args as { request: { type: string } }).request.type);
      return null;
    });
    // roep elke methode met voldoende dummy-argumenten aan
    const dummy: unknown[] = [1, 1, 1, 1];
    for (const fn of Object.values(api)) {
      await (fn as (...a: unknown[]) => Promise<unknown>)(...dummy).catch(() => undefined);
    }
    const missing = names.filter((n) => !sent.has(n));
    expect(missing).toEqual([]);
    expect([...sent].filter((s) => !names.includes(s))).toEqual([]); // geen verzoek dat Rust niet kent
  });

  it("fouten uit de backend worden ApiError met de originele tekst", async () => {
    setCommandTransport(async () => {
      throw "Ongeldig pad";
    });
    await expect(api.libraryFolders()).rejects.toMatchObject({ name: "ApiError", message: "Ongeldig pad" });
  });
});
