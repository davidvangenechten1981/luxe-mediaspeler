import type { StreamInfo } from "../api/types";
import { audioDetails, channelLabel, hdrLabel, videoDetails } from "./mediaDetails";

const base: StreamInfo = {
  index: 0, kind: "video", codec: "hevc", language: null, title: null, width: 3840, height: 2160, fps: 23.976, channels: null,
  sample_rate: null, bitrate: 18_500_000, codec_long: "H.265 / HEVC", profile: "Main 10", pix_fmt: "yuv420p10le", bit_depth: 10,
  color_space: "bt2020nc", color_transfer: "smpte2084", channel_layout: null,
};

describe("hdrLabel", () => {
  it("herkent HDR10 en HLG", () => {
    expect(hdrLabel("smpte2084")).toBe("HDR10");
    expect(hdrLabel("arib-std-b67")).toBe("HLG");
  });
  it("geeft niets voor gewone beelden", () => {
    expect(hdrLabel("bt709")).toBeNull();
    expect(hdrLabel(null)).toBeNull();
  });
});

describe("channelLabel", () => {
  it("noemt bekende indelingen bij naam", () => {
    expect(channelLabel(1, null)).toBe("Mono");
    expect(channelLabel(2, "stereo")).toBe("Stereo");
    expect(channelLabel(6, "5.1(side)")).toBe("5.1");
    expect(channelLabel(8, "7.1")).toBe("7.1");
  });
  it("valt terug op het aantal kanalen", () => {
    expect(channelLabel(4, null)).toBe("4 kanalen");
    expect(channelLabel(null, null)).toBeNull();
  });
});

describe("videoDetails", () => {
  it("toont codec, profiel, resolutie, beeldsnelheid, bitrate, kleurdiepte en HDR", () => {
    const d = videoDetails(base);
    expect(d.title).toBe("H.265 / HEVC");
    expect(d.lines).toContain("3840×2160 (4K)");
    expect(d.lines).toContain("23,976 beelden/s");
    expect(d.lines).toContain("18,5 Mbit/s");
    expect(d.lines).toContain("Main 10");
    expect(d.lines).toContain("10 bit · yuv420p10le");
    expect(d.badges).toEqual(["HDR10"]);
  });
  it("laat ontbrekende gegevens weg in plaats van streepjes of 'null' te tonen", () => {
    const d = videoDetails({ ...base, codec_long: null, profile: null, pix_fmt: null, bit_depth: null, bitrate: null, fps: null, color_transfer: null });
    expect(d.title).toBe("hevc");
    expect(d.lines).toEqual(["3840×2160 (4K)"]);
    expect(d.badges).toEqual([]);
  });
});

describe("audioDetails", () => {
  it("toont codec, kanalen, samplefrequentie, bitrate en taal", () => {
    const a: StreamInfo = { ...base, kind: "audio", codec: "eac3", codec_long: "ATSC A/52B (AC-3, E-AC-3)", width: null, height: null, fps: null, channels: 6, channel_layout: "5.1(side)", sample_rate: 48000, bitrate: 640_000, language: "nld", profile: null, pix_fmt: null, bit_depth: null, color_space: null, color_transfer: null };
    const d = audioDetails(a);
    expect(d.title).toBe("ATSC A/52B (AC-3, E-AC-3)");
    expect(d.lines).toEqual(["5.1", "48 kHz", "640 kbit/s", "nld"]);
  });
  it("schrijft samplefrequenties met komma", () => {
    const a: StreamInfo = { ...base, kind: "audio", codec: "flac", codec_long: null, channels: 2, channel_layout: "stereo", sample_rate: 44100, bitrate: null, width: null, height: null, fps: null, language: null, profile: null, pix_fmt: null, bit_depth: 16, color_space: null, color_transfer: null };
    expect(audioDetails(a).lines).toEqual(["Stereo", "44,1 kHz", "16 bit"]);
  });
});
