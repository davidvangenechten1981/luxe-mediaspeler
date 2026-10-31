// Leesbare technische gegevens van een spoor, voor het infopaneel tijdens het afspelen.
// Ontbrekende gegevens worden weggelaten (geen streepjes of "null").

import type { StreamInfo } from "../api/types";
import { formatBitrate, resolutionLabel } from "./format";

export interface StreamDetails {
  title: string;
  lines: string[];
  badges: string[];
}

const nl = (n: number, digits = 3) => String(Math.round(n * 10 ** digits) / 10 ** digits).replace(".", ",");

/** HDR-type uit de overdrachtskarakteristiek van ffprobe. */
export function hdrLabel(transfer: string | null): string | null {
  if (transfer === "smpte2084") return "HDR10";
  if (transfer === "arib-std-b67") return "HLG";
  return null;
}

/** Uitleg bij HDR-beeld: de speler zet HDR niet om naar een gewoon scherm, dus het beeld kan flets lijken. */
export function hdrWarning(transfer: string | null): string | null {
  const label = hdrLabel(transfer);
  return label ? `Dit beeld is ${label}. Op een scherm zonder HDR kan het beeld flets lijken; de speler kan dit nog niet omzetten.` : null;
}

/** "Stereo", "5.1", "7.1" of anders het aantal kanalen. */
export function channelLabel(channels: number | null, layout: string | null): string | null {
  if (layout) {
    const l = layout.toLowerCase();
    if (l === "mono") return "Mono";
    if (l === "stereo") return "Stereo";
    const m = l.match(/^(\d)\.(\d)/);
    if (m) return `${m[1]}.${m[2]}`;
  }
  if (channels === 1) return "Mono";
  if (channels === 2) return "Stereo";
  return channels ? `${channels} kanalen` : null;
}

export function videoDetails(s: StreamInfo): StreamDetails {
  const res = s.width && s.height ? `${s.width}×${s.height}${resolutionLabel(s.height) ? ` (${resolutionLabel(s.height)})` : ""}` : null;
  const depth = [s.bit_depth ? `${s.bit_depth} bit` : null, s.pix_fmt].filter(Boolean).join(" · ");
  const hdr = hdrLabel(s.color_transfer);
  return {
    title: s.codec_long ?? s.codec,
    lines: [s.profile, res, s.fps ? `${nl(s.fps)} beelden/s` : null, s.bitrate ? formatBitrate(s.bitrate) : null, depth || null].filter((x): x is string => !!x),
    badges: hdr ? [hdr] : [],
  };
}

export function audioDetails(s: StreamInfo): StreamDetails {
  const kHz = s.sample_rate ? `${nl(s.sample_rate / 1000, 1)} kHz` : null;
  return {
    title: s.codec_long ?? s.codec,
    lines: [channelLabel(s.channels, s.channel_layout), kHz, s.bitrate ? formatBitrate(s.bitrate) : null, s.bit_depth ? `${s.bit_depth} bit` : null, s.language].filter((x): x is string => !!x),
    badges: [],
  };
}
