// Onderdelen van derden die in de installer zitten. Houd dit gelijk aan licenses/THIRD-PARTY.md.

export const SOURCE_REPO = "https://github.com/davidvangenechten1981/luxe-mediaspeler";

export interface LicensedComponent {
  name: string;
  license: string;
  /** Waar de broncode of de officiële release te vinden is. */
  source: string;
  note?: string;
}

export const COMPONENTS: LicensedComponent[] = [
  { name: "LibVLC", license: "LGPL-2.1-of-later", source: "https://download.videolan.org/pub/videolan/vlc/3.0.21/", note: "Speelmotor van VideoLAN, dynamisch geladen. Sommige plugins zijn GPL." },
  { name: "FFmpeg en ffprobe", license: "GPL-3.0 (build met libx264 en libx265)", source: "https://github.com/BtbN/FFmpeg-Builds", note: "Analyse en conversie. De broncode van FFmpeg staat op https://ffmpeg.org/download.html." },
  { name: "Tauri", license: "MIT of Apache-2.0", source: "https://github.com/tauri-apps/tauri", note: "Vensterframework." },
  { name: "SQLite (rusqlite)", license: "MIT / publiek domein", source: "https://github.com/rusqlite/rusqlite", note: "Bibliotheek en instellingen." },
  { name: "React en overige npm-pakketten", license: "MIT", source: "https://github.com/facebook/react", note: "Interface; volledige lijst in pnpm-lock.yaml." },
  { name: "WebView2", license: "Microsoft-licentie", source: "https://developer.microsoft.com/microsoft-edge/webview2/", note: "Zit in Windows 10/11; wordt niet meegeleverd." },
];
