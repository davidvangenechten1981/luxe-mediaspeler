import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { ConvertSettings, MediaInfo, StreamInfo } from "../../api/types";
import * as platform from "../../lib/platform";
import { burnPatch, burnValue, subtitleStreams, SubtitleTools, trackLabel } from "./SubtitleTools";

const stream = (index: number, codec: string, language: string | null = "nld"): StreamInfo => ({ index, kind: "subtitle", codec, language, title: null } as unknown as StreamInfo);
const info = { streams: [{ index: 0, kind: "video" } as unknown as StreamInfo, stream(2, "subrip"), stream(3, "hdmv_pgs_subtitle", "eng")] } as unknown as MediaInfo;
const settings = { burn_subtitle_track: null, burn_external_subtitle: false, external_subtitle: null } as unknown as ConvertSettings;

describe("helpers", () => {
  it("vindt de ondertitelsporen en maakt een leesbaar label", () => {
    expect(subtitleStreams(info).map((s) => s.index)).toEqual([2, 3]);
    expect(subtitleStreams(null)).toEqual([]);
    expect(trackLabel(stream(2, "subrip"), 0)).toBe("Spoor 1: nld · subrip");
  });
  it("zet keuzes om naar instellingen en terug", () => {
    expect(burnPatch("geen")).toEqual({ burn_external_subtitle: false, burn_subtitle_track: null });
    expect(burnPatch("1")).toEqual({ burn_external_subtitle: false, burn_subtitle_track: 1 });
    expect(burnPatch("extern")).toEqual({ burn_external_subtitle: true, burn_subtitle_track: null });
    expect(burnValue({ ...settings, burn_subtitle_track: 1 })).toBe("1");
    expect(burnValue({ ...settings, burn_external_subtitle: true })).toBe("extern");
    expect(burnValue(settings)).toBe("geen");
  });
});

describe("SubtitleTools", () => {
  it("laat inbranden kiezen en geeft de keuze door", () => {
    const set = vi.fn();
    render(<SubtitleTools input="/m/a.mkv" info={info} settings={settings} set={set} />);
    fireEvent.change(screen.getByLabelText("Ondertitels in het beeld branden"), { target: { value: "0" } });
    expect(set).toHaveBeenCalledWith({ burn_external_subtitle: false, burn_subtitle_track: 0 });
  });
  it("biedt opslaan als tekst alleen aan bij tekstondertitels (niet bij plaatjes)", async () => {
    vi.spyOn(platform, "pickFolder").mockResolvedValue("/uit");
    const extract = vi.spyOn(api, "convertExtractSubtitle").mockResolvedValue("/uit/a.nld.srt");
    render(<SubtitleTools input="/m/a.mkv" info={info} settings={settings} set={() => undefined} />);
    expect(screen.getAllByRole("button", { name: /opslaan als \.srt/ })).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: /Spoor 1 opslaan als \.srt/ }));
    await waitFor(() => expect(extract).toHaveBeenCalledWith("/m/a.mkv", 2, "/uit"));
  });
  it("toont niets zonder ondertitels", () => {
    const { container } = render(<SubtitleTools input="/m/a.mkv" info={{ streams: [] } as unknown as MediaInfo} settings={settings} set={() => undefined} />);
    expect(container).toBeEmptyDOMElement();
  });
});
