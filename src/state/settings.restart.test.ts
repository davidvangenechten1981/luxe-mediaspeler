import { RESTART_KEYS, useSettings, DEFAULT_SETTINGS } from "./settings";

const save = vi.fn();
vi.mock("../api/client", async (orig) => ({ ...(await orig<typeof import("../api/client")>()), api: { settingsSave: (s: unknown) => save(s) } }));
vi.mock("../lib/platform", async (orig) => ({ ...(await orig<typeof import("../lib/platform")>()), windowApi: { setAlwaysOnTop: async () => undefined } }));

beforeEach(() => {
  save.mockReset();
  save.mockImplementation(async (s) => s);
  useSettings.setState({ settings: DEFAULT_SETTINGS, needsRestart: false });
});

describe("herstart nodig", () => {
  it("noemt de instellingen die pas na een herstart gelden", () => {
    expect(RESTART_KEYS).toEqual(expect.arrayContaining(["subtitle_scale", "subtitle_color", "convert_parallel"]));
  });

  it("zet de herstartmelding aan bij zo'n instelling", async () => {
    await useSettings.getState().update({ subtitle_scale: 1.5 });
    expect(useSettings.getState().needsRestart).toBe(true);
  });

  it("laat de melding uit bij gewone instellingen en bij onveranderde waarden", async () => {
    await useSettings.getState().update({ compact: true });
    expect(useSettings.getState().needsRestart).toBe(false);
    await useSettings.getState().update({ subtitle_scale: DEFAULT_SETTINGS.subtitle_scale });
    expect(useSettings.getState().needsRestart).toBe(false);
  });

  it("zet de melding niet aan als opslaan mislukt", async () => {
    save.mockRejectedValue(new Error("nee"));
    await useSettings.getState().update({ subtitle_color: "#ff0000" });
    expect(useSettings.getState().needsRestart).toBe(false);
  });
});
