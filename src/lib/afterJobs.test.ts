import { describe, expect, it } from "vitest";
import type { JobSnapshot, JobState } from "../api/types";
import { actionLabel, countdownText, queueJustFinished } from "./afterJobs";

const j = (id: number, state: JobState): JobSnapshot =>
  ({ id, input: "a", output: null, profile: "p", state, mode: "transcode", fraction: null, eta_s: null, speed: null, error: null, warnings: [], paused: false, started_at: null, finished_at: null });

describe("queueJustFinished", () => {
  it("is waar als de laatste actieve taak klaar is", () => {
    expect(queueJustFinished([j(1, "running"), j(2, "queued")], [j(1, "done"), j(2, "done")])).toBe(true);
  });
  it("is onwaar zolang er nog werk is", () => {
    expect(queueJustFinished([j(1, "running"), j(2, "queued")], [j(1, "done"), j(2, "running")])).toBe(false);
  });
  it("is onwaar als er niets actief was, of alles mislukte of werd geannuleerd", () => {
    expect(queueJustFinished([j(1, "done")], [j(1, "done")])).toBe(false);
    expect(queueJustFinished([j(1, "running")], [j(1, "failed")])).toBe(false);
    expect(queueJustFinished([j(1, "running")], [j(1, "cancelled")])).toBe(false);
  });
});

describe("teksten", () => {
  it("noemt de actie en de aftelling", () => {
    expect(actionLabel("sleep")).toMatch(/slaapstand/);
    expect(countdownText("shutdown", 30)).toContain("30 seconden");
    expect(countdownText("sleep", -5)).toContain("0 seconden");
  });
});
