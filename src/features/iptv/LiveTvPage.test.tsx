import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import type { IptvChannel, IptvSource, IptvVpn } from "../../api/types";
import { DEFAULT_SETTINGS, useSettings } from "../../state/settings";
import { useUi } from "../../state/ui";
import { LiveTvPage } from "./LiveTvPage";

const source: IptvSource = { id: 1, name: "Thuis", kind: "m3u_file", location: "D:\\tv.m3u", epg_location: "", user_agent: null, last_refresh: 1, last_error: null, channel_count: 3 };

function ch(id: number, name: string, patch: Partial<IptvChannel> = {}): IptvChannel {
  return {
    id, source_id: 1, number: id, name, kind: "live", group: "Nieuws", logo: "https://tracker.example/logo.png", tvg_id: null, radio: false, catchup_mode: null,
    catchup_days: null, epg_id: null, epg_manual: false, favorite: false, hidden: false, last_watched: null, ...patch,
  };
}

const list = [ch(1, "Een"), ch(2, "Twee"), ch(3, "Voorbeeldserie", { kind: "series", number: null })];
const noVpn: IptvVpn = { checked: true, active: false, adapter: null, detail: "Geen actieve VPN-adapter gevonden", required: false, stopped: false };

beforeEach(() => {
  vi.restoreAllMocks();
  useUi.setState({ page: "live_tv", params: {}, toasts: [] });
  useSettings.setState({ settings: { ...DEFAULT_SETTINGS } });
  vi.spyOn(api, "iptvSources").mockResolvedValue([source]);
  vi.spyOn(api, "iptvGroups").mockResolvedValue([{ name: "Nieuws", count: 2 }]);
  vi.spyOn(api, "iptvChannels").mockResolvedValue(list);
  vi.spyOn(api, "iptvVpnCheck").mockResolvedValue(noVpn);
  vi.spyOn(api, "iptvNowNext").mockResolvedValue([
    { channel_id: 1, now: { epg_id: "een.nl", start: Math.floor(Date.now() / 1000) - 600, stop: Math.floor(Date.now() / 1000) + 600, title: "Journaal", desc: null }, next: { epg_id: "een.nl", start: Math.floor(Date.now() / 1000) + 600, stop: Math.floor(Date.now() / 1000) + 1200, title: "Weer", desc: null } },
  ]);
});

describe("LiveTvPage", () => {
  it("legt uit dat er een eigen bron nodig is als er nog geen is", async () => {
    vi.spyOn(api, "iptvSources").mockResolvedValue([]);
    render(<LiveTvPage />);
    expect(await screen.findByText("Nog geen bron")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Bron toevoegen/ })).toBeTruthy();
  });

  it("toont zenders met nu en straks, standaard zonder plaatjes van internet", async () => {
    const logo = vi.spyOn(api, "iptvLogo").mockResolvedValue(null);
    const { container } = render(<LiveTvPage />);
    expect(await screen.findByRole("button", { name: "Speel Een" })).toBeTruthy();
    expect(await screen.findByText(/Journaal/)).toBeTruthy();
    expect(screen.getByText(/straks .* Weer/)).toBeTruthy();
    expect(container.querySelector("img")).toBeNull(); // privacy: geen verbindingen voor logo's
    expect(logo).not.toHaveBeenCalled();
  });

  it("toont logo's alleen als de instelling aan staat", async () => {
    useSettings.setState({ settings: { ...DEFAULT_SETTINGS, iptv_show_logos: true } });
    const logo = vi.spyOn(api, "iptvLogo").mockImplementation(async (id) => (id === 1 ? "C:\\cache\\thumbnails\\logos\\een.png" : null));
    const { container } = render(<LiveTvPage />);
    await screen.findByRole("button", { name: "Speel Een" });
    await waitFor(() => expect(container.querySelectorAll("img")).toHaveLength(1)); // alleen zender 1 had een bestand
    expect(logo).toHaveBeenCalledWith(1);
    expect(screen.getByText("TW")).toBeTruthy(); // zonder bestand: initialen
  });

  it("toont de VPN-status: actief, ontbrekend en verplicht", async () => {
    const check = vi.spyOn(api, "iptvVpnCheck");
    check.mockResolvedValue({ ...noVpn, active: true, adapter: "NordLynx", detail: "Je internetverkeer loopt via de VPN" });
    const { unmount } = render(<LiveTvPage />);
    expect(await screen.findByText("VPN actief · NordLynx")).toBeTruthy();
    unmount();
    check.mockResolvedValue(noVpn);
    const second = render(<LiveTvPage />);
    expect(await screen.findByText("Geen VPN gevonden")).toBeTruthy();
    second.unmount();
    check.mockResolvedValue({ ...noVpn, required: true });
    render(<LiveTvPage />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText("Geen VPN: streams zijn geblokkeerd")).toBeTruthy();
  });

  it("meldt dat de stream is gestopt als de kill switch heeft ingegrepen", async () => {
    vi.spyOn(api, "iptvVpnCheck").mockResolvedValue({ ...noVpn, required: true, stopped: true });
    render(<LiveTvPage />);
    await waitFor(() => expect(useUi.getState().toasts.some((t) => t.text.includes("VPN is verbroken"))).toBe(true));
  });

  it("speelt een zender af en gaat naar Nu afspelen", async () => {
    const play = vi.spyOn(api, "iptvPlay").mockResolvedValue(list[0]!);
    render(<LiveTvPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Speel Een" }));
    await waitFor(() => expect(play).toHaveBeenCalledWith(1));
    await waitFor(() => expect(useUi.getState().page).toBe("now_playing"));
  });

  it("markeert favorieten en zoekt met de ingetypte tekst", async () => {
    const fav = vi.spyOn(api, "iptvChannelFavorite").mockResolvedValue(null);
    render(<LiveTvPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Een als favoriet markeren" }));
    await waitFor(() => expect(fav).toHaveBeenCalledWith(1, true));
    fireEvent.change(screen.getByRole("searchbox", { name: "Zoeken in zenders" }), { target: { value: "twee" } });
    await waitFor(() => expect(api.iptvChannels).toHaveBeenLastCalledWith(expect.objectContaining({ search: "twee", kind: "live" })));
  });

  it("de knoppen vorige en volgende zender zappen", async () => {
    const zap = vi.spyOn(api, "iptvZap").mockResolvedValue(list[1]!);
    render(<LiveTvPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Volgende zender" }));
    await waitFor(() => expect(zap).toHaveBeenCalledWith(0, 1, false));
  });

  it("opent een serie met afleveringen en speelt er een af", async () => {
    vi.spyOn(api, "iptvSeriesEpisodes").mockResolvedValue([{ id: "9001", season: 1, episode: 1, title: "Begin", extension: "mkv" }]);
    const playEpisode = vi.spyOn(api, "iptvPlayEpisode").mockResolvedValue(null);
    render(<LiveTvPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Open Voorbeeldserie" }));
    fireEvent.click(await screen.findByRole("button", { name: /1\. Begin/ }));
    await waitFor(() => expect(playEpisode).toHaveBeenCalledWith(3, "9001", "mkv"));
  });

  it("neemt een zender op voor 60 minuten en meldt als dat niet lukt of wacht op de VPN", async () => {
    const rec = vi.spyOn(api, "iptvRecordNow").mockResolvedValue({ id: 1, channel_name: "Een", title: "x", start: 1, stop: 2, status: "failed", path: null, message: "Kies eerst een map voor opnames" });
    render(<LiveTvPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Een opnemen (60 minuten)" }));
    await waitFor(() => expect(rec).toHaveBeenCalledWith(1, 60));
    await act(async () => {});
    expect(useUi.getState().toasts.some((t) => t.kind === "error" && t.text.includes("map voor opnames"))).toBe(true);

    rec.mockResolvedValue({ id: 2, channel_name: "Een", title: "x", start: 1, stop: 2, status: "planned", path: null, message: "Geen VPN actief: test." });
    fireEvent.click(screen.getByRole("button", { name: "Een opnemen (60 minuten)" }));
    await waitFor(() => expect(useUi.getState().toasts.some((t) => t.kind === "warning" && t.text.includes("Geen VPN actief"))).toBe(true));
  });
});
