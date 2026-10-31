import { create } from "zustand";
import { api } from "../api/client";
import type { CapabilityView, JobSnapshot, Profile } from "../api/types";

interface JobsState {
  jobs: JobSnapshot[];
  profiles: Profile[];
  caps: CapabilityView | null;
  available: boolean;
  refresh: () => Promise<void>;
  upsert: (job: JobSnapshot) => void;
  loadStatic: () => Promise<void>;
}

export const useJobs = create<JobsState>((set, get) => ({
  jobs: [],
  profiles: [],
  caps: null,
  available: true,
  refresh: async () => {
    try {
      set({ jobs: await api.convertList(), available: true });
    } catch {
      set({ available: false });
    }
  },
  upsert: (job) => {
    const jobs = get().jobs;
    const i = jobs.findIndex((j) => j.id === job.id);
    set({ jobs: i < 0 ? [...jobs, job] : jobs.map((j) => (j.id === job.id ? job : j)) });
  },
  loadStatic: async () => {
    try {
      const [profiles, caps] = await Promise.all([api.convertProfiles(), api.convertCapabilities()]);
      set({ profiles, caps, available: true });
    } catch {
      set({ available: false });
    }
  },
}));
