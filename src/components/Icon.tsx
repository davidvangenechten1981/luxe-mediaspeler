// Eigen iconenset: eenvoudige SVG-tekeningen (24×24, lijnen), zonder externe licentie.

import type { SVGProps } from "react";

const P: Record<string, string> = {
  play: "M8 5.5v13L19 12z",
  pause: "M8 5v14 M16 5v14",
  stop: "M6.5 6.5h11v11h-11z",
  next: "M6 6l9 6-9 6z M18.5 6v12",
  prev: "M18 6l-9 6 9 6z M5.5 6v12",
  "skip-forward": "M12 5a7 7 0 11-7 7 M12 2v6l4-3z M10 10v5 M14 10h-2.2v2.3h2a1.2 1.2 0 010 2.4H11.5",
  "skip-back": "M12 5a7 7 0 10-7 7 M12 2v6L8 5z M10 10v5 M14 10h-2.2v2.3h2a1.2 1.2 0 010 2.4H11.5",
  volume: "M4 9.5v5h3.5l4.5 4v-13l-4.5 4z M15.5 9a4 4 0 010 6 M18.2 6.3a8 8 0 010 11.4",
  "volume-low": "M4 9.5v5h3.5l4.5 4v-13l-4.5 4z M15.5 9a4 4 0 010 6",
  mute: "M4 9.5v5h3.5l4.5 4v-13l-4.5 4z M16 9.5l5 5 M21 9.5l-5 5",
  fullscreen: "M4 9V4h5 M20 9V4h-5 M4 15v5h5 M20 15v5h-5",
  "fullscreen-exit": "M9 4v5H4 M15 4v5h5 M9 20v-5H4 M15 20v-5h5",
  shuffle: "M3 7h3c2 0 3 1 4.2 3l2.6 4c1.2 2 2.2 3 4.2 3H20 M3 17h3c1.3 0 2.2-.5 3-1.4 M13.6 8.3C14.5 7.4 15.6 7 17 7h3 M17.5 4.5L20 7l-2.5 2.5 M17.5 14.5L20 17l-2.5 2.5",
  repeat: "M4 11V9a3 3 0 013-3h12 M16 3l3 3-3 3 M20 13v2a3 3 0 01-3 3H5 M8 21l-3-3 3-3",
  "repeat-one": "M4 11V9a3 3 0 013-3h12 M16 3l3 3-3 3 M20 13v2a3 3 0 01-3 3H5 M8 21l-3-3 3-3 M11.2 10.3l1.3-.8v5",
  queue: "M4 7h11 M4 12h11 M4 17h6 M17.5 13.5v6l4.5-3z",
  list: "M9 6h11 M9 12h11 M9 18h11 M4.5 6h.01 M4.5 12h.01 M4.5 18h.01",
  grid: "M4 4h7v7H4z M13 4h7v7h-7z M4 13h7v7H4z M13 13h7v7h-7z",
  heart: "M12 20s-7.5-4.6-7.5-10.2A4.3 4.3 0 0112 7.3a4.3 4.3 0 017.5 2.5C19.5 15.4 12 20 12 20z",
  folder: "M3.5 7.5a2 2 0 012-2H10l2 2h6.5a2 2 0 012 2v7.5a2 2 0 01-2 2h-13a2 2 0 01-2-2z",
  "folder-plus": "M3.5 7.5a2 2 0 012-2H10l2 2h6.5a2 2 0 012 2v7.5a2 2 0 01-2 2h-13a2 2 0 01-2-2z M12 10.5v5 M9.5 13h5",
  file: "M7 3.5h7l4.5 4.5v11a1.5 1.5 0 01-1.5 1.5H7A1.5 1.5 0 015.5 19V5A1.5 1.5 0 017 3.5z M14 3.5V8h4.5",
  film: "M4 5.5h16v13H4z M8 5.5v13 M16 5.5v13 M4 9.5h4 M4 14.5h4 M16 9.5h4 M16 14.5h4",
  music: "M9 17.5V6l10-2v11.5 M9 17.5a2.5 2.5 0 11-5 0 2.5 2.5 0 015 0z M19 15.5a2.5 2.5 0 11-5 0 2.5 2.5 0 015 0z",
  library: "M5 4v16 M10 4v16 M15 5l4.5 15",
  search: "M11 4.5a6.5 6.5 0 100 13 6.5 6.5 0 000-13z M20 20l-4.2-4.2",
  settings: "M12 9a3 3 0 100 6 3 3 0 000-6z M19.4 13.5a7.7 7.7 0 000-3l1.9-1.5-1.8-3.1-2.3.8a7.600 7.600 0 00-2.600-1.500L14 2.500h-3.600l-.6 2.700a7.600 7.600 0 00-2.600 1.500l-2.300-.8-1.800 3.100 1.900 1.500a7.700 7.700 0 000 3l-1.900 1.500 1.800 3.100 2.300-.8a7.600 7.600 0 002.600 1.500l.6 2.700H14l.6-2.700a7.600 7.600 0 002.600-1.500l2.300.8 1.800-3.100z",
  home: "M4 11l8-6.500 8 6.500 M6 9.800V20h4.500v-5.500h3V20H18V9.800",
  history: "M4.5 12a7.500 7.500 0 107.500-7.500A7.500 7.500 0 006 6.800 M4 4v3.500h3.500 M12 8v4.500l3 1.800",
  clock: "M12 4.500a7.500 7.500 0 100 15 7.500 7.500 0 000-15z M12 8v4.500l3 1.800",
  continue: "M5 5.500v13L16 12z M19 6v12",
  sliders: "M5 4v16 M12 4v16 M19 4v16 M3 14h4 M10 9h4 M17 16h4",
  wand: "M5 19L16 8 M14.500 5.500l4 4 M17 3.500v2 M20.500 7h-2 M7 4v2 M8 5H6 M5 11v2 M6 12H4",
  convert: "M5 8h12 M14 4.500L17.500 8 14 11.500 M19 16H7 M10 12.500L6.500 16 10 19.500",
  download: "M12 4v11 M7.500 10.500L12 15l4.500-4.500 M5 19.500h14",
  upload: "M12 15V4 M7.500 8.500L12 4l4.500 4.500 M5 19.500h14",
  close: "M6 6l12 12 M18 6L6 18",
  minimize: "M6 12h12",
  maximize: "M6.500 6.500h11v11h-11z",
  restore: "M9 9h8.500v8.500H9z M6.500 15V6.500H15",
  "chevron-left": "M14.500 6L8.500 12l6 6",
  "chevron-right": "M9.500 6l6 6-6 6",
  "chevron-down": "M6 9.500l6 6 6-6",
  "chevron-up": "M6 14.500l6-6 6 6",
  plus: "M12 5v14 M5 12h14",
  minus: "M5 12h14",
  trash: "M5 7h14 M9.500 7V4.500h5V7 M7 7l.8 12h8.400L17 7 M10.500 10.500v5.500 M13.500 10.500v5.500",
  edit: "M5 19l.8-3.800L16.500 4.500a1.800 1.800 0 012.500 0l.5.5a1.800 1.800 0 010 2.500L8.800 18.200z M14.500 6.500l3 3",
  more: "M6 12h.01 M12 12h.01 M18 12h.01",
  info: "M12 4.500a7.500 7.500 0 100 15 7.500 7.500 0 000-15z M12 11v5 M12 8h.01",
  camera: "M4 8.500a2 2 0 012-2h2l1.500-2h5l1.500 2h2a2 2 0 012 2V17a2 2 0 01-2 2H6a2 2 0 01-2-2z M12 10.500a3 3 0 100 6 3 3 0 000-6z",
  loop: "M4 12a4 4 0 014-4h8 M13 5l3 3-3 3 M20 12a4 4 0 01-4 4H8 M11 19l-3-3 3-3",
  speed: "M4.500 17a8 8 0 1115 0 M12 17l4-5 M7 17h.01 M17 17h.01",
  subtitles: "M4 6.500h16v11H4z M7.500 11.500h3 M13 11.500h3.500 M7.500 14.500h5 M15 14.500h1.500",
  "audio-track": "M4 12v3 M8 9v9 M12 5.500v13 M16 9v9 M20 12v3",
  check: "M5 12.500l4.500 4.500L19 7.500",
  warning: "M12 4.500L21 19.500H3z M12 10v4.500 M12 17h.01",
  error: "M12 4.500a7.500 7.500 0 100 15 7.500 7.500 0 000-15z M12 8v5 M12 16h.01",
  sun: "M12 8a4 4 0 100 8 4 4 0 000-8z M12 3v2 M12 19v2 M3 12h2 M19 12h2 M5.600 5.600L7 7 M17 17l1.400 1.400 M5.600 18.400L7 17 M17 7l1.400-1.400",
  moon: "M19.500 14.500A7.500 7.500 0 019.500 4.500a7.500 7.500 0 1010 10z",
  pin: "M9 4h6l-1 5 3 3H7l3-3z M12 12v8",
  mini: "M4 6h16v12H4z M12 12h6v5h-6z",
  theater: "M3 7h18v10H3z M3 4.500h18 M3 19.500h18",
  crop: "M7 3v14h14 M3 7h14v14",
  rotate: "M19 12a7 7 0 11-2.100-5 M19 4v4.500h-4.500",
  flip: "M12 4v16 M8 7L3.500 12 8 17z M16 7l4.500 5L16 17z",
  brightness: "M12 8a4 4 0 100 8 4 4 0 000-8z M12 3v2 M12 19v2 M3 12h2 M19 12h2",
  contrast: "M12 4.500a7.500 7.500 0 100 15 7.500 7.500 0 000-15z M12 4.500v15",
  tune: "M4 7h9 M17 7h3 M4 17h3 M11 17h9 M15 5v4 M9 15v4",
  import: "M12 4v11 M7.500 10.500L12 15l4.500-4.500 M4 19.500h16",
  export: "M12 15V4 M7.500 8.500L12 4l4.500 4.500 M4 19.500h16",
  refresh: "M19 12a7 7 0 11-2.100-5 M19 4v4.500h-4.500",
  drag: "M9 6h.01 M15 6h.01 M9 12h.01 M15 12h.01 M9 18h.01 M15 18h.01",
  star: "M12 4l2.400 5 5.500.7-4 3.800 1 5.500L12 16.400 7.100 19l1-5.500-4-3.800L9.600 9z",
  external: "M14 4.500h5.500V10 M19.500 4.500L11 13 M17 14v4a1.500 1.500 0 01-1.500 1.500h-9A1.500 1.500 0 015 18V9a1.500 1.500 0 011.500-1.500H10",
  copy: "M8.500 8.500h10v10h-10z M5.500 15.500V5.500h10",
  bookmark: "M7 4.500h10v15l-5-3.500-5 3.500z",
  link: "M10 14a4 4 0 005.700 0l3-3a4 4 0 00-5.700-5.700l-1 1 M14 10a4 4 0 00-5.700 0l-3 3a4 4 0 005.700 5.700l1-1",
  cd: "M12 4.500a7.500 7.500 0 100 15 7.500 7.500 0 000-15z M12 10.500a1.500 1.500 0 100 3 1.500 1.500 0 000-3z",
  shield: "M12 4l7 2.500v5c0 4.500-3 7.500-7 9-4-1.500-7-4.500-7-9v-5z M9 12l2.200 2.200L15.500 10",
  eye: "M2.500 12S6 6 12 6s9.500 6 9.500 6-3.500 6-9.500 6S2.500 12 2.500 12z M12 9.500a2.500 2.500 0 100 5 2.500 2.500 0 000-5z",
  frame: "M7 5.500v13 M7 12h10 M13.500 8.500L17 12l-3.500 3.500",
  keyboard: "M3.500 7h17v10h-17z M7 10.500h.01 M10 10.500h.01 M13 10.500h.01 M16 10.500h.01 M8 14h8",
};

export type IconName = keyof typeof P;

export interface IconProps extends Omit<SVGProps<SVGSVGElement>, "name"> {
  name: IconName;
  size?: number;
  filled?: boolean;
}

export function Icon({ name, size = 20, filled = false, ...rest }: IconProps) {
  const d = P[name];
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={filled ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {d.split(" M").map((seg, i) => (
        <path key={i} d={i === 0 ? seg : `M${seg}`} />
      ))}
    </svg>
  );
}

export const ICON_NAMES = Object.keys(P) as IconName[];
