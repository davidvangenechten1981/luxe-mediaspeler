import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import { PhotosPage, stepIndex } from "./PhotosPage";

const photos = [1, 2, 3].map((n) => ({ path: `C:\\f\\${n}.jpg`, name: `${n}.jpg`, modified: n, size: 1000 }));

beforeEach(() => {
  vi.restoreAllMocks();
  vi.spyOn(api, "photosList").mockResolvedValue(photos);
  vi.spyOn(api, "thumbnail").mockResolvedValue("C:\\cache\\x.jpg");
});

describe("stepIndex", () => {
  it("loopt rond", () => {
    expect(stepIndex(0, -1, 3)).toBe(2);
    expect(stepIndex(2, 1, 3)).toBe(0);
    expect(stepIndex(1, 1, 3)).toBe(2);
  });
});

describe("PhotosPage", () => {
  it("toont de foto's en opent een grote weergave met pijltjes", async () => {
    render(<PhotosPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Open 1.jpg" }));
    expect(await screen.findByText("1.jpg", { selector: "h2, strong, span" })).toBeTruthy();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => expect(screen.getAllByText("2.jpg").length).toBeGreaterThan(0));
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });
  it("legt uit wat er mis is als er geen foto's zijn", async () => {
    vi.spyOn(api, "photosList").mockResolvedValue([]);
    render(<PhotosPage />);
    expect(await screen.findByText(/Geen foto/)).toBeTruthy();
  });
});
