import { readFileSync } from "node:fs";

// De tests draaien altijd vanuit de projectmap (pnpm test)
const css = readFileSync("src/styles/shell.css", "utf8");

// Het videovenster zit ACHTER het interfacevenster. Een dichte achtergrond op .app zou de video bedekken
// (in de browser is dat niet te zien, daar zit geen video achter). Deze test bewaakt dat.
function rule(selector: string): string {
  const m = css.match(new RegExp(`(?:^|\\n)${selector.replace(/[.]/g, "\\.")}\\s*\\{([^}]*)\\}`));
  return m?.[1] ?? "";
}

describe("videozone", () => {
  it("maakt de app transparant zolang de videopagina actief is", () => {
    expect(rule(".app.is-video")).toMatch(/background:\s*transparent/);
  });
  it("laat de gewone app een dichte achtergrond houden", () => {
    expect(rule(".app")).toMatch(/background:\s*var\(--bg\)/);
  });
  it("houdt de balken zelf dicht, zodat alleen de videozone doorzichtig is", () => {
    expect(rule(".playerbar")).toMatch(/background:\s*var\(--bg-elev\)/);
    expect(rule(".sidebar")).toMatch(/background:\s*var\(--bg-elev\)/);
  });
});
