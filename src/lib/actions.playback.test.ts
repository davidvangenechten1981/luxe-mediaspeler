import { IDLE_VIEW, usePlayer } from "../state/player";
import { adjustDelay, seekChapter } from "./actions";

const mediaInfo = vi.fn();
vi.mock("../api/client", async (orig) => ({ ...(await orig<typeof import("../api/client")>()), api: { mediaInfo: (p: string) => mediaInfo(p) } }));

const sent: unknown[] = [];
const view = (patch: Partial<typeof IDLE_VIEW.status>, location: string | null = "C:\\f.mkv") => ({ ...IDLE_VIEW, location, status: { ...IDLE_VIEW.status, ...patch } });

beforeEach(() => {
  sent.length = 0;
  mediaInfo.mockReset();
  usePlayer.setState({ send: async (c) => (sent.push(c), null) });
});

describe("adjustDelay", () => {
  it("telt de vertraging op bij de huidige waarde", async () => {
    usePlayer.setState({ view: view({ subtitle_delay_ms: 200, audio_delay_ms: -50 }) });
    await adjustDelay("subtitle", -100);
    await adjustDelay("audio", 50);
    expect(sent).toEqual([{ type: "subtitle_delay", ms: 100 }, { type: "audio_delay", ms: 0 }]);
  });
});

describe("seekChapter", () => {
  const info = { chapters: [{ start_ms: 0, end_ms: 60_000, title: null }, { start_ms: 60_000, end_ms: 120_000, title: null }] };

  it("springt naar het volgende hoofdstuk", async () => {
    usePlayer.setState({ view: view({ time_ms: 10_000 }, "C:\\a.mkv") });
    mediaInfo.mockResolvedValue(info);
    await seekChapter("next");
    expect(sent).toEqual([{ type: "seek_to", ms: 60_000 }]);
  });

  it("doet niets (en vraagt niets op) zonder spelend item", async () => {
    usePlayer.setState({ view: view({}, null) });
    await seekChapter("next");
    expect(sent).toEqual([]);
    expect(mediaInfo).not.toHaveBeenCalled();
  });

  it("onthoudt de hoofdstukken per bestand en vraagt ffprobe niet steeds opnieuw", async () => {
    usePlayer.setState({ view: view({ time_ms: 10_000 }, "C:\\b.mkv") });
    mediaInfo.mockResolvedValue(info);
    await seekChapter("next");
    await seekChapter("next");
    expect(mediaInfo).toHaveBeenCalledTimes(1);
  });
});
