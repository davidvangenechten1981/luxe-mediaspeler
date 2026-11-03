import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import type { IptvChannel, IptvProgramme, IptvRecording, IptvSource } from "../../api/types";
import { DEFAULT_SETTINGS, useSettings } from "../../state/settings";
import { useUi } from "../../state/ui";
import { GuidePage } from "./GuidePage";
import { RecordingsPage } from "./RecordingsPage";
import { SourcesDialog } from "./SourcesDialog";

const at = (h: number, m = 0) => Math.floor(new Date(2026, 10, 3, h, m).getTime() / 1000);

const channel: IptvChannel = {
  id: 1, source_id: 1, number: 1, name: "Een", kind: "live", group: "Nieuws", logo: null, tvg_id: "een.nl", radio: false, catchup_mode: "shift", catchup_days: 3,
  epg_id: "een.nl", epg_manual: false, favorite: false, hidden: false, last_watched: null,
};
const prog = (title: string, start: number, stop: number): IptvProgramme => ({ epg_id: "een.nl", start, stop, title, desc: title === "Journaal" ? "Het nieuws van de dag." : null });

describe("GuidePage", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(2026, 10, 3, 12, 30));
    useUi.setState({ page: "tv_guide", params: {}, toasts: [] });
    vi.spyOn(api, "iptvGroups").mockResolvedValue([{ name: "Nieuws", count: 1 }]);
    vi.spyOn(api, "iptvChannels").mockResolvedValue([channel, { ...channel, id: 2, name: "Zonder gids", epg_id: null }]);
    vi.spyOn(api, "iptvGuide").mockResolvedValue([prog("Voorbij", at(12), at(12, 20)), prog("Journaal", at(12, 20), at(13)), prog("Film", at(13), at(14))]);
    vi.spyOn(api, "iptvSearchProgrammes").mockResolvedValue([]);
  });
  afterEach(() => vi.useRealTimers());

  it("toont alleen zenders met gids en de programma's als blokken", async () => {
    render(<GuidePage />);
    expect(await screen.findByRole("button", { name: /Een: Journaal, 12:20 tot 13:00/ })).toBeTruthy();
    expect(screen.queryByText("Zonder gids")).toBeNull();
    expect(screen.getAllByRole("button", { name: /^Een:/ })).toHaveLength(3);
  });

  it("een lopend programma: nu kijken", async () => {
    const play = vi.spyOn(api, "iptvPlay").mockResolvedValue(channel);
    render(<GuidePage />);
    fireEvent.click(await screen.findByRole("button", { name: /Een: Journaal/ }));
    expect(await screen.findByText("Het nieuws van de dag.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Nu kijken" }));
    await waitFor(() => expect(play).toHaveBeenCalledWith(1));
  });

  it("een toekomstig programma: herinnering en opnemen, geen terugkijken", async () => {
    const remind = vi.spyOn(api, "iptvReminderAdd").mockResolvedValue({ id: 1, channel_id: 1, channel_name: "Een", title: "Film", start: at(13), notified: false });
    const add = vi.spyOn(api, "iptvRecordingAdd").mockResolvedValue({ id: 1 } as IptvRecording);
    render(<GuidePage />);
    fireEvent.click(await screen.findByRole("button", { name: /Een: Film/ }));
    expect(screen.queryByRole("button", { name: "Terugkijken" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Herinner mij" }));
    await waitFor(() => expect(remind).toHaveBeenCalledWith(1, "Film", at(13)));
    fireEvent.click(await screen.findByRole("button", { name: /Een: Film/ }));
    fireEvent.click(screen.getByRole("button", { name: "Opnemen" }));
    await waitFor(() => expect(add).toHaveBeenCalledWith(1, "Film", at(13), at(14)));
  });

  it("een voorbij programma binnen het archief kan worden teruggekeken", async () => {
    const catchup = vi.spyOn(api, "iptvPlayCatchup").mockResolvedValue(channel);
    render(<GuidePage />);
    fireEvent.click(await screen.findByRole("button", { name: /Een: Voorbij/ }));
    fireEvent.click(screen.getByRole("button", { name: "Terugkijken" }));
    await waitFor(() => expect(catchup).toHaveBeenCalledWith(1, at(12), at(12, 20)));
  });

  it("legt uit wat er mis is zonder gids", async () => {
    vi.spyOn(api, "iptvChannels").mockResolvedValue([{ ...channel, epg_id: null }]);
    render(<GuidePage />);
    expect(await screen.findByText("Nog geen gids")).toBeTruthy();
  });
});

describe("RecordingsPage", () => {
  const rec = (id: number, status: IptvRecording["status"], patch: Partial<IptvRecording> = {}): IptvRecording => ({
    id, channel_name: "Een", title: `Opname ${id}`, start: at(10), stop: at(11), status, path: null, message: null, ...patch,
  });

  beforeEach(() => {
    vi.restoreAllMocks();
    useSettings.setState({ settings: { ...DEFAULT_SETTINGS, iptv_record_dir: null } });
    vi.spyOn(api, "iptvReminders").mockResolvedValue([{ id: 7, channel_id: 1, channel_name: "Een", title: "Journaal", start: at(20), notified: false }]);
  });

  it("vraagt om een opnamemap als die ontbreekt, en toont statussen", async () => {
    vi.spyOn(api, "iptvRecordings").mockResolvedValue([rec(1, "recording"), rec(2, "done", { path: "D:\\Opnames\\x.ts" }), rec(3, "failed", { message: "Kies eerst een map" })]);
    render(<RecordingsPage />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText(/Kies eerst een map voor je opnames/)).toBeTruthy();
    expect(await screen.findByText("Neemt op")).toBeTruthy();
    expect(screen.getByText("Klaar")).toBeTruthy();
    expect(screen.getByText("Mislukt")).toBeTruthy();
    expect(screen.getByText("Kies eerst een map")).toBeTruthy();
    expect(screen.getByText(/Er loopt nu een opname/)).toBeTruthy();
  });

  it("stopt een lopende opname, haalt een afgeronde uit de lijst en verwijdert een herinnering", async () => {
    vi.spyOn(api, "iptvRecordings").mockResolvedValue([rec(1, "recording"), rec(2, "done", { path: "D:\\Opnames\\x.ts" })]);
    const cancel = vi.spyOn(api, "iptvRecordingCancel").mockResolvedValue(null);
    const del = vi.spyOn(api, "iptvRecordingDelete").mockResolvedValue(null);
    const delReminder = vi.spyOn(api, "iptvReminderDelete").mockResolvedValue(null);
    render(<RecordingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Stoppen" }));
    await waitFor(() => expect(cancel).toHaveBeenCalledWith(1));
    fireEvent.click(await screen.findByRole("button", { name: "Opname 2 uit de lijst halen" }));
    await waitFor(() => expect(del).toHaveBeenCalledWith(2));
    expect(screen.queryByRole("button", { name: "Opname 1 uit de lijst halen" })).toBeNull(); // lopende opname eerst stoppen
    fireEvent.click(await screen.findByRole("button", { name: "Herinnering Journaal verwijderen" }));
    await waitFor(() => expect(delReminder).toHaveBeenCalledWith(7));
  });
});

describe("SourcesDialog", () => {
  const source: IptvSource = { id: 1, name: "Thuis", kind: "xtream", location: "http://prov.example:8080", epg_location: "", user_agent: null, last_refresh: null, last_error: "Inloggen is mislukt", channel_count: 0 };

  beforeEach(() => {
    vi.restoreAllMocks();
    useUi.setState({ toasts: [] });
  });

  it("voegt een Xtream-bron toe en toont het wachtwoord nooit leesbaar", async () => {
    const add = vi.spyOn(api, "iptvSourceAddXtream").mockResolvedValue(source);
    const changed = vi.fn();
    render(<SourcesDialog sources={[]} onClose={() => undefined} onChanged={changed} />);
    fireEvent.click(screen.getByRole("tab", { name: "Xtream" }));
    fireEvent.change(screen.getByLabelText("Naam"), { target: { value: "Thuis" } });
    fireEvent.change(screen.getByLabelText("Serveradres"), { target: { value: "http://prov.example:8080" } });
    fireEvent.change(screen.getByLabelText("Gebruikersnaam"), { target: { value: "jan" } });
    const pass = screen.getByLabelText("Wachtwoord") as HTMLInputElement;
    expect(pass.type).toBe("password");
    const button = screen.getByRole("button", { name: "Bron toevoegen" });
    expect((button as HTMLButtonElement).disabled).toBe(true); // wachtwoord ontbreekt nog
    fireEvent.change(pass, { target: { value: "geheim" } });
    fireEvent.click(button);
    await waitFor(() => expect(add).toHaveBeenCalledWith("Thuis", "http://prov.example:8080", "jan", "geheim"));
    await waitFor(() => expect(changed).toHaveBeenCalled());
  });

  it("de link van een M3U-bron staat in een wachtwoordveld", () => {
    render(<SourcesDialog sources={[]} onClose={() => undefined} onChanged={() => undefined} />);
    expect((screen.getByLabelText(/Link naar de afspeellijst/) as HTMLInputElement).type).toBe("password");
  });

  it("ververst, meldt een fout van de bron en vraagt bevestiging voor verwijderen", async () => {
    const refresh = vi.spyOn(api, "iptvSourceRefresh").mockResolvedValue({ source_id: 1, channels: 5, skipped: 1, guide_link_found: false });
    const del = vi.spyOn(api, "iptvSourceDelete").mockResolvedValue(null);
    render(<SourcesDialog sources={[source]} onClose={() => undefined} onChanged={() => undefined} />);
    expect(screen.getByText("Inloggen is mislukt")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Verversen" }));
    await waitFor(() => expect(refresh).toHaveBeenCalledWith(1));
    fireEvent.click(screen.getByRole("button", { name: "Thuis verwijderen" }));
    expect(del).not.toHaveBeenCalled();
    fireEvent.click(await screen.findByRole("button", { name: "Verwijderen" }));
    await waitFor(() => expect(del).toHaveBeenCalledWith(1));
  });
});
