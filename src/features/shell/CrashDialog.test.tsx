import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CrashDialog } from "./CrashDialog";

const save = vi.fn();
const exportDiag = vi.fn();
vi.mock("../../lib/platform", async (orig) => ({ ...(await orig<typeof import("../../lib/platform")>()), pickSavePath: (...a: unknown[]) => save(...a) }));
vi.mock("../../api/client", async (orig) => ({ ...(await orig<typeof import("../../api/client")>()), api: { diagnosticsExport: (...a: unknown[]) => exportDiag(...a) } }));

beforeEach(() => {
  save.mockReset();
  exportDiag.mockReset();
});

describe("CrashDialog", () => {
  it("legt uit wat er is gebeurd en toont het begin van het logboek", () => {
    render(<CrashDialog text={"PANIC: iets is kapot"} onClose={() => undefined} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/onverwacht gestopt/i, { selector: "h2, h1, [id]" })).toBeInTheDocument();
    expect(screen.getByText(/PANIC: iets is kapot/)).toBeInTheDocument();
  });

  it("slaat de diagnose op naar het gekozen bestand", async () => {
    save.mockResolvedValue("C:\\Users\\jij\\diagnose.txt");
    exportDiag.mockResolvedValue(null);
    render(<CrashDialog text="x" onClose={() => undefined} />);
    await userEvent.click(screen.getByRole("button", { name: /Diagnose opslaan/ }));
    expect(exportDiag).toHaveBeenCalledWith("C:\\Users\\jij\\diagnose.txt");
  });

  it("doet niets als je de opslagdialoog annuleert", async () => {
    save.mockResolvedValue(null);
    render(<CrashDialog text="x" onClose={() => undefined} />);
    await userEvent.click(screen.getByRole("button", { name: /Diagnose opslaan/ }));
    expect(exportDiag).not.toHaveBeenCalled();
  });
});
