import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { COMPONENTS, SOURCE_REPO } from "./licenses";
import { LicensesDialog } from "./LicensesDialog";

describe("licenties", () => {
  it("noemt FFmpeg als GPL en LibVLC als LGPL, met een bronadres", () => {
    const ff = COMPONENTS.find((c) => c.name.startsWith("FFmpeg"))!;
    const vlc = COMPONENTS.find((c) => c.name.startsWith("LibVLC"))!;
    expect(ff.license).toMatch(/GPL-3/);
    expect(vlc.license).toMatch(/LGPL/);
    for (const c of COMPONENTS) expect(c.source).toMatch(/^https:\/\//);
  });

  it("toont de applicatielicentie, het bronaanbod en de componenten", () => {
    render(<LicensesDialog ffmpegVersion="ffmpeg version 7.1" onClose={() => undefined} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/GPL-3\.0-of-later/)).toBeInTheDocument();
    expect(screen.getByText(SOURCE_REPO)).toBeInTheDocument();
    expect(screen.getByText(/ffmpeg version 7\.1/)).toBeInTheDocument();
    expect(screen.getByText("LibVLC")).toBeInTheDocument();
  });

  it("toont de volledige GPL-tekst op verzoek", async () => {
    render(<LicensesDialog onClose={() => undefined} />);
    expect(screen.queryByText(/GNU GENERAL PUBLIC LICENSE/)).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Volledige GPL-3\.0-tekst/ }));
    expect(screen.getAllByText(/GNU GENERAL PUBLIC LICENSE/).length).toBeGreaterThan(0);
  });
});
