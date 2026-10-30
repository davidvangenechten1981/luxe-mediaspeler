import { render, screen } from "@testing-library/react";
import type { Diagnostics } from "../../api/types";
import { toolsProblems, ToolsBanner } from "./ToolsBanner";

const ok: Diagnostics = {
  app_version: "0.1.0", ffmpeg: "ffmpeg version 7", ffprobe: "ffprobe version 7", ffmpeg_path: "C:\\x", vlc_dir: "C:\\vlc", player_available: true,
  player_error: null, schema_version: 2, data_dir: "C:\\d", warnings: [], encoders: [], containers: [],
};

describe("toolsProblems", () => {
  it("meldt niets als alles werkt", () => {
    expect(toolsProblems(ok)).toEqual([]);
  });
  it("legt uit dat de speler ontbreekt, met de reden en een tip over antivirus", () => {
    const p = toolsProblems({ ...ok, player_available: false, player_error: "libvlc.dll niet gevonden" });
    expect(p.join(" ")).toMatch(/Afspelen is niet mogelijk/);
    expect(p.join(" ")).toMatch(/libvlc\.dll niet gevonden/);
    expect(p.join(" ")).toMatch(/antivirus/i);
  });
  it("meldt ontbrekende FFmpeg apart (afspelen kan dan nog wel)", () => {
    const p = toolsProblems({ ...ok, ffmpeg: null, ffprobe: null });
    expect(p).toHaveLength(1);
    expect(p[0]).toMatch(/Converteren/);
  });
});

describe("ToolsBanner", () => {
  it("toont niets zonder problemen en anders de meldingen", () => {
    const { container, rerender } = render(<ToolsBanner diagnostics={ok} />);
    expect(container).toBeEmptyDOMElement();
    rerender(<ToolsBanner diagnostics={{ ...ok, player_available: false, player_error: "stuk" }} />);
    expect(screen.getByRole("alert")).toHaveTextContent("stuk");
  });
});
