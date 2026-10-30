import { render, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { api } from "../../api/client";
import type { Media } from "../../api/types";
import * as platform from "../../lib/platform";
import { Thumb } from "./Thumb";

const song = { id: 1, path: "/m/lied.mp3", kind: "audio", missing: false, duration_ms: 1000, album: "A", artist: "B" } as unknown as Media;

describe("Thumb voor muziek", () => {
  it("toont de albumhoes als die er is", async () => {
    const cover = vi.spyOn(api, "coverArt").mockResolvedValue("/cache/cover.jpg");
    vi.spyOn(platform, "fileSrc").mockResolvedValue("asset://cover.jpg");
    const { container } = render(<Thumb media={song} />);
    await waitFor(() => expect(container.querySelector("img")).toHaveAttribute("src", "asset://cover.jpg"));
    expect(cover).toHaveBeenCalledWith("/m/lied.mp3");
  });

  it("houdt het kleurvlak zonder hoes", async () => {
    const cover = vi.spyOn(api, "coverArt").mockResolvedValue(null);
    const { container } = render(<Thumb media={{ ...song, path: "/m/kaal.mp3" }} />);
    await waitFor(() => expect(cover).toHaveBeenCalledWith("/m/kaal.mp3"));
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector(".card-media__placeholder")).not.toBeNull();
  });
});
