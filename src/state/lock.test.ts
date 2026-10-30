import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../api/client";
import { isBlocked, isLockedPage, useLock } from "./lock";

beforeEach(() => useLock.setState({ enabled: false, unlocked: true }));

describe("isLockedPage", () => {
  it("vergrendelt bibliotheekpagina's maar niet de instellingen of de converter", () => {
    expect(isLockedPage("videos")).toBe(true);
    expect(isLockedPage("home")).toBe(true);
    expect(isLockedPage("settings")).toBe(false);
    expect(isLockedPage("converter")).toBe(false);
    expect(isLockedPage("now_playing")).toBe(false);
  });
});

describe("useLock", () => {
  it("blijft open zonder PIN en vergrendelt na init met een PIN", async () => {
    vi.spyOn(api, "pinStatus").mockResolvedValueOnce({ enabled: false });
    await useLock.getState().init();
    expect(isBlocked("videos", useLock.getState())).toBe(false);
    vi.spyOn(api, "pinStatus").mockResolvedValueOnce({ enabled: true });
    await useLock.getState().init();
    expect(isBlocked("videos", useLock.getState())).toBe(true);
    expect(isBlocked("settings", useLock.getState())).toBe(false);
  });

  it("ontgrendelt met de juiste PIN en niet met een foute", async () => {
    useLock.setState({ enabled: true, unlocked: false });
    vi.spyOn(api, "pinVerify").mockResolvedValueOnce(false);
    expect(await useLock.getState().unlock("0000")).toBe(false);
    expect(useLock.getState().unlocked).toBe(false);
    vi.spyOn(api, "pinVerify").mockResolvedValueOnce(true);
    expect(await useLock.getState().unlock("1234")).toBe(true);
    expect(useLock.getState().unlocked).toBe(true);
  });

  it("kan opnieuw vergrendelen, maar alleen als er een PIN is", () => {
    useLock.getState().lock();
    expect(useLock.getState().unlocked).toBe(true);
    useLock.setState({ enabled: true, unlocked: true });
    useLock.getState().lock();
    expect(useLock.getState().unlocked).toBe(false);
  });

  it("vergrendelt niets als de backend niet bereikbaar is", async () => {
    vi.spyOn(api, "pinStatus").mockRejectedValueOnce(new Error("geen backend"));
    await useLock.getState().init();
    expect(useLock.getState()).toMatchObject({ enabled: false, unlocked: true });
  });
});
