import { IDLE_VIEW, usePlayer } from "./player";
import { useSleep } from "./sleep";

const sent: unknown[] = [];

beforeEach(() => {
  sent.length = 0;
  useSleep.setState({ timer: null });
  usePlayer.setState({ view: { ...IDLE_VIEW, status: { ...IDLE_VIEW.status, volume: 80 } }, send: async (c) => (sent.push(c), null) });
});

describe("slaaptimer", () => {
  it("start, loopt af en stopt het afspelen", () => {
    useSleep.getState().start(30);
    const t = useSleep.getState().timer!;
    expect(t.endsAt).toBeGreaterThan(Date.now());
    useSleep.getState().tick(t.endsAt - 60_000);
    expect(sent).toEqual([]);
    useSleep.getState().tick(t.endsAt + 1);
    expect(sent).toEqual([{ type: "stop" }, { type: "volume", percent: 80 }]);
    expect(useSleep.getState().timer).toBeNull();
  });

  it("annuleren midden in het uitfaden zet het volume terug", () => {
    useSleep.getState().start(1);
    const t = useSleep.getState().timer!;
    useSleep.getState().tick(t.endsAt - 4_000); // halverwege het uitfaden
    expect(sent.at(-1)).toEqual({ type: "volume", percent: 40 });
    useSleep.getState().cancel();
    expect(sent.at(-1)).toEqual({ type: "volume", percent: 80 });
    expect(useSleep.getState().timer).toBeNull();
  });

  it("annuleren zonder uitfaden laat het volume met rust", () => {
    useSleep.getState().start(10);
    useSleep.getState().cancel();
    expect(sent).toEqual([]);
  });
});
