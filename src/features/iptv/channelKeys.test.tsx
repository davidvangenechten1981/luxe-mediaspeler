import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import type { IptvChannel } from "../../api/types";
import { IDLE_VIEW, usePlayer } from "../../state/player";
import { useUi } from "../../state/ui";
import { ChannelKeys, channelIdFromLocation, useChannelEntry } from "./channelKeys";

const chan = (id: number, name: string): IptvChannel => ({
  id, source_id: 1, number: id, name, kind: "live", group: null, logo: null, tvg_id: null, radio: false, catchup_mode: null, catchup_days: null,
  epg_id: null, epg_manual: false, favorite: false, hidden: false, last_watched: null,
});

function playing(location: string | null) {
  usePlayer.setState({ view: { ...IDLE_VIEW, location } });
}

beforeEach(() => {
  vi.restoreAllMocks();
  useUi.setState({ page: "home", params: {}, toasts: [] });
  useChannelEntry.setState({ typed: "" });
  playing(null);
});

describe("channelIdFromLocation", () => {
  it("herkent alleen de neutrale naam van een livezender", () => {
    expect(channelIdFromLocation("iptv://channel/42")).toBe(42);
    expect(channelIdFromLocation("iptv://catchup/42/1000")).toBeNull();
    expect(channelIdFromLocation("C:\\media\\film.mkv")).toBeNull();
    expect(channelIdFromLocation(null)).toBeNull();
  });
});

describe("ChannelKeys", () => {
  it("doet niets op andere pagina's en bij een gewoon bestand (PageUp/PageDown blijven scrollen)", () => {
    const zap = vi.spyOn(api, "iptvZap").mockResolvedValue(chan(2, "Twee"));
    const byNumber = vi.spyOn(api, "iptvPlayNumber").mockResolvedValue(chan(2, "Twee"));
    playing("C:\\media\\film.mkv");
    render(<ChannelKeys />);
    fireEvent.keyDown(window, { key: "PageDown" });
    fireEvent.keyDown(window, { key: "1" });
    fireEvent.keyDown(window, { key: "Enter" });
    expect(zap).not.toHaveBeenCalled();
    expect(byNumber).not.toHaveBeenCalled();
  });

  it("zapt vanaf de zender die nu speelt, ook als je ergens anders in de app bent", async () => {
    const zap = vi.spyOn(api, "iptvZap").mockResolvedValue(chan(6, "Zes"));
    playing("iptv://channel/5");
    render(<ChannelKeys />);
    fireEvent.keyDown(window, { key: "PageDown" });
    await waitFor(() => expect(zap).toHaveBeenCalledWith(5, 1, false));
    fireEvent.keyDown(window, { key: "PageUp" });
    await waitFor(() => expect(zap).toHaveBeenLastCalledWith(5, -1, false)); // de speler-staat is leidend
    await waitFor(() => expect(useUi.getState().toasts.some((t) => t.text.includes("Zes"))).toBe(true));
  });

  it("typt een kanaalnummer met cijfers en Enter en toont wat je typt", async () => {
    const byNumber = vi.spyOn(api, "iptvPlayNumber").mockResolvedValue(chan(12, "Twaalf"));
    playing("iptv://channel/5");
    render(<ChannelKeys />);
    fireEvent.keyDown(window, { key: "1" });
    fireEvent.keyDown(window, { key: "2" });
    expect(await screen.findByText("Kanaal 12")).toBeTruthy();
    fireEvent.keyDown(window, { key: "Enter" });
    await waitFor(() => expect(byNumber).toHaveBeenCalledWith(12));
    await waitFor(() => expect(screen.queryByText(/Kanaal 12/)).toBeNull());
  });

  it("op de Live-tv-pagina werkt het ook zonder dat er iets speelt, en gaat het daarna naar Nu afspelen", async () => {
    useUi.setState({ page: "live_tv" });
    const zap = vi.spyOn(api, "iptvZap").mockResolvedValue(chan(1, "Een"));
    vi.spyOn(api, "iptvPlayNumber").mockResolvedValue(chan(3, "Drie"));
    render(<ChannelKeys />);
    fireEvent.keyDown(window, { key: "PageDown" });
    await waitFor(() => expect(zap).toHaveBeenCalledWith(expect.any(Number), 1, false)); // begint bij de laatst bekende zender
    fireEvent.keyDown(window, { key: "3" });
    fireEvent.keyDown(window, { key: "Enter" });
    await waitFor(() => expect(useUi.getState().page).toBe("now_playing"));
  });

  it("negeert toetsen in een invoerveld en met Ctrl", () => {
    const zap = vi.spyOn(api, "iptvZap").mockResolvedValue(chan(2, "Twee"));
    const byNumber = vi.spyOn(api, "iptvPlayNumber").mockResolvedValue(chan(2, "Twee"));
    playing("iptv://channel/5");
    render(<><ChannelKeys /><input aria-label="veld" /></>);
    const box = screen.getByLabelText("veld");
    fireEvent.keyDown(box, { key: "PageDown" });
    fireEvent.keyDown(box, { key: "7" });
    fireEvent.keyDown(window, { key: "PageDown", ctrlKey: true });
    fireEvent.keyDown(window, { key: "Enter" });
    expect(zap).not.toHaveBeenCalled();
    expect(byNumber).not.toHaveBeenCalled();
  });
});
