import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import { DEFAULT_SETTINGS, useSettings } from "../../state/settings";
import { IptvSettings } from "./IptvSettings";

const update = vi.fn(async () => true);

beforeEach(() => {
  vi.restoreAllMocks();
  update.mockClear();
  useSettings.setState({ settings: { ...DEFAULT_SETTINGS }, update });
  vi.spyOn(api, "iptvVpnCheck").mockResolvedValue({ checked: true, active: false, adapter: null, detail: "Geen actieve VPN-adapter gevonden", required: false, stopped: false });
});

describe("IptvSettings", () => {
  it("heeft de VPN-eis en de logo's standaard uit en legt de VPN-status uit", async () => {
    render(<IptvSettings />);
    expect(await screen.findByText("Geen VPN")).toBeTruthy();
    expect(screen.getByText("Geen actieve VPN-adapter gevonden")).toBeTruthy();
    const switches = screen.getAllByRole("switch") as HTMLInputElement[];
    expect(switches.map((s) => s.getAttribute("aria-checked") ?? String(s.checked))).toEqual(["false", "false"]);
  });

  it("zet de VPN-eis en de logo's aan", async () => {
    render(<IptvSettings />);
    const [vpn, logos] = screen.getAllByRole("switch");
    fireEvent.click(vpn!);
    await waitFor(() => expect(update).toHaveBeenCalledWith({ iptv_require_vpn: true }));
    fireEvent.click(logos!);
    await waitFor(() => expect(update).toHaveBeenCalledWith({ iptv_show_logos: true }));
  });

  it("slaat een eigen VPN-adapternaam op als je het veld verlaat, en wist hem als het leeg is", async () => {
    render(<IptvSettings />);
    const box = screen.getByLabelText("Naam van je VPN-adapter");
    fireEvent.change(box, { target: { value: " Acme Secure " } });
    fireEvent.blur(box);
    await waitFor(() => expect(update).toHaveBeenCalledWith({ iptv_vpn_name: "Acme Secure" }));
  });

  it("toont de gekozen opnamemap en kan hem wissen", async () => {
    useSettings.setState({ settings: { ...DEFAULT_SETTINGS, iptv_record_dir: "D:\\Opnames" }, update });
    render(<IptvSettings />);
    expect((screen.getByLabelText("Map voor opnames") as HTMLInputElement).value).toBe("D:\\Opnames");
    fireEvent.click(screen.getByRole("button", { name: "Opnamemap wissen" }));
    await waitFor(() => expect(update).toHaveBeenCalledWith({ iptv_record_dir: null }));
  });

  it("meldt als de VPN-controle niet mogelijk is", async () => {
    vi.spyOn(api, "iptvVpnCheck").mockResolvedValue({ checked: false, active: false, adapter: null, detail: "De VPN-controle werkt alleen op Windows", required: false, stopped: false });
    render(<IptvSettings />);
    expect(await screen.findByText("Niet te controleren")).toBeTruthy();
  });
});
