import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import { DEFAULT_SETTINGS, useSettings } from "../../state/settings";
import { WatchSettings } from "./WatchSettings";

beforeEach(() => {
  vi.restoreAllMocks();
  vi.spyOn(api, "convertProfiles").mockResolvedValue([{ id: "compact", name: "Compact" }] as never);
  useSettings.setState({ settings: { ...DEFAULT_SETTINGS, watch_dir: "C:\\Inbox", watch_profile: "compact" } });
});

describe("WatchSettings", () => {
  it("toont de gekozen map en controleert op verzoek", async () => {
    const run = vi.spyOn(api, "watchRun").mockResolvedValue({ queued: 2, rejected: 0 });
    render(<WatchSettings />);
    expect((screen.getByLabelText("Bewaakte map") as HTMLInputElement).value).toBe("C:\\Inbox");
    fireEvent.click(screen.getByRole("button", { name: "Nu controleren" }));
    await waitFor(() => expect(run).toHaveBeenCalled());
  });
  it("kan niet controleren zonder map", () => {
    useSettings.setState({ settings: { ...DEFAULT_SETTINGS, watch_dir: null } });
    render(<WatchSettings />);
    expect(screen.getByRole("button", { name: "Nu controleren" })).toBeDisabled();
  });
});
