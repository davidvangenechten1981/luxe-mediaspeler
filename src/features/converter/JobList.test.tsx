import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { JobSnapshot } from "../../api/types";
import { JobRow, stateLabel } from "./JobList";

const job = (over: Partial<JobSnapshot> = {}): JobSnapshot => ({
  id: 1, input: "C:\\v\\film.mkv", output: null, profile: "Compact", state: "running", mode: "transcode", fraction: 0.4, eta_s: 60, speed: 2,
  error: null, warnings: [], paused: false, started_at: 1, finished_at: null, ...over,
});

describe("stateLabel", () => {
  it("toont Gepauzeerd alleen bij een lopende, gepauzeerde taak", () => {
    expect(stateLabel(job())).toBe("Bezig");
    expect(stateLabel(job({ paused: true }))).toBe("Gepauzeerd");
    expect(stateLabel(job({ state: "done", paused: true }))).toBe("Klaar");
  });
});

describe("JobRow", () => {
  it("pauzeert een lopende conversie", async () => {
    const pause = vi.spyOn(api, "convertPause").mockResolvedValue(null);
    render(<JobRow job={job()} />);
    fireEvent.click(screen.getByRole("button", { name: "Pauzeren" }));
    await waitFor(() => expect(pause).toHaveBeenCalledWith(1));
  });
  it("hervat een gepauzeerde conversie en verbergt dan de snelheid", async () => {
    const resume = vi.spyOn(api, "convertResume").mockResolvedValue(null);
    render(<JobRow job={job({ paused: true })} />);
    expect(screen.queryByText(/sneller dan realtime/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Hervatten" }));
    await waitFor(() => expect(resume).toHaveBeenCalledWith(1));
  });
  it("biedt geen pauzeknop voor wachtende taken", () => {
    render(<JobRow job={job({ state: "queued" })} />);
    expect(screen.queryByRole("button", { name: "Pauzeren" })).toBeNull();
  });
});
