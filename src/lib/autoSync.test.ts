import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { startAutoSync } from "./autoSync";

describe("startAutoSync", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("synchroniseert meteen en daarna op de ingestelde tijd", async () => {
    const run = vi.fn().mockResolvedValue(null);
    const stop = startAutoSync({ folder: () => "/nas/sync", run, onFirstError: vi.fn(), everyMs: 1000 });
    await vi.advanceTimersByTimeAsync(0);
    expect(run).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(2500);
    expect(run).toHaveBeenCalledTimes(3);
    stop();
    await vi.advanceTimersByTimeAsync(5000);
    expect(run).toHaveBeenCalledTimes(3);
  });

  it("doet niets zonder gekozen map", async () => {
    const run = vi.fn().mockResolvedValue(null);
    let folder: string | null = null;
    const stop = startAutoSync({ folder: () => folder, run, onFirstError: vi.fn(), everyMs: 1000 });
    await vi.advanceTimersByTimeAsync(3000);
    expect(run).not.toHaveBeenCalled();
    folder = "/x"; // later ingesteld: de volgende ronde doet mee
    await vi.advanceTimersByTimeAsync(1000);
    expect(run).toHaveBeenCalledTimes(1);
    stop();
  });

  it("meldt een reeks mislukkingen maar één keer en meldt opnieuw na een succes", async () => {
    const run = vi.fn().mockRejectedValue(new Error("NAS uit"));
    const onFirstError = vi.fn();
    const stop = startAutoSync({ folder: () => "/nas", run, onFirstError, everyMs: 1000 });
    await vi.advanceTimersByTimeAsync(3500);
    expect(run.mock.calls.length).toBeGreaterThanOrEqual(3);
    expect(onFirstError).toHaveBeenCalledTimes(1);
    run.mockResolvedValueOnce(null);
    await vi.advanceTimersByTimeAsync(1000);
    run.mockRejectedValue(new Error("weer uit"));
    await vi.advanceTimersByTimeAsync(1000);
    expect(onFirstError).toHaveBeenCalledTimes(2);
    stop();
  });

  it("start geen tweede synchronisatie zolang de eerste loopt", async () => {
    let finish: (v: unknown) => void = () => undefined;
    const run = vi.fn().mockImplementation(() => new Promise((r) => (finish = r)));
    const stop = startAutoSync({ folder: () => "/nas", run, onFirstError: vi.fn(), everyMs: 1000 });
    await vi.advanceTimersByTimeAsync(3500);
    expect(run).toHaveBeenCalledTimes(1);
    finish(null);
    await vi.advanceTimersByTimeAsync(1000);
    expect(run).toHaveBeenCalledTimes(2);
    stop();
  });
});
