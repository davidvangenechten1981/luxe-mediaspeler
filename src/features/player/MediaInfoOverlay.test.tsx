import { render, screen } from "@testing-library/react";
import type { MediaInfo } from "../../api/types";
import { usePlayer, IDLE_VIEW } from "../../state/player";
import { MediaInfoOverlay } from "./MediaInfoOverlay";

const info: MediaInfo = {
  duration_ms: 7_200_000, format: "matroska,webm", size: 8_000_000_000, bitrate: 9_000_000, chapters: [],
  tags: { title: null, artist: null, album: null, genre: null, year: null, track: null },
  streams: [
    { index: 0, kind: "video", codec: "hevc", codec_long: "H.265 / HEVC", profile: "Main 10", pix_fmt: "yuv420p10le", bit_depth: 10, color_space: "bt2020nc", color_transfer: "smpte2084", channel_layout: null, language: null, title: null, width: 3840, height: 2160, fps: 24, channels: null, sample_rate: null, bitrate: 8_000_000 },
    { index: 1, kind: "audio", codec: "eac3", codec_long: "E-AC-3", profile: null, pix_fmt: null, bit_depth: null, color_space: null, color_transfer: null, channel_layout: "5.1(side)", language: "nld", title: null, width: null, height: null, fps: null, channels: 6, sample_rate: 48000, bitrate: 640_000 },
    { index: 2, kind: "subtitle", codec: "subrip", codec_long: null, profile: null, pix_fmt: null, bit_depth: null, color_space: null, color_transfer: null, channel_layout: null, language: "eng", title: null, width: null, height: null, fps: null, channels: null, sample_rate: null, bitrate: null },
  ],
};

beforeEach(() => usePlayer.setState({ view: { ...IDLE_VIEW, location: "C:\\Films\\Voorbeeld.mkv" } }));

describe("MediaInfoOverlay", () => {
  it("toont container, video, audio en ondertitels", () => {
    render(<MediaInfoOverlay info={info} loading={false} onClose={() => undefined} />);
    expect(screen.getByRole("complementary", { name: "Mediagegevens" })).toBeInTheDocument();
    expect(screen.getByText("matroska")).toBeInTheDocument();
    expect(screen.getByText("H.265 / HEVC")).toBeInTheDocument();
    expect(screen.getByText("HDR10")).toBeInTheDocument();
    expect(screen.getByText(/3840×2160 \(4K\)/)).toBeInTheDocument();
    expect(screen.getByText("E-AC-3")).toBeInTheDocument();
    expect(screen.getByText(/5\.1 · 48 kHz · 640 kbit\/s · nld/)).toBeInTheDocument();
    expect(screen.getByText(/subrip eng/)).toBeInTheDocument();
  });
  it("legt uit dat er niets te tonen is als de analyse ontbreekt", () => {
    render(<MediaInfoOverlay info={null} loading={false} onClose={() => undefined} />);
    expect(screen.getByText(/geen technische gegevens/)).toBeInTheDocument();
  });
  it("sluit via de knop", () => {
    const onClose = vi.fn();
    render(<MediaInfoOverlay info={info} loading={false} onClose={onClose} />);
    screen.getByRole("button", { name: "Mediagegevens sluiten" }).click();
    expect(onClose).toHaveBeenCalled();
  });
});
