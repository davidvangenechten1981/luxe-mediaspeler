import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import { ProfileSettings } from "./ProfileSettings";

const reload = vi.fn();
beforeEach(() => {
  vi.restoreAllMocks();
  reload.mockReset();
  vi.spyOn(api, "profileList").mockResolvedValue({ profiles: [{ id: 1, name: "Standaard" }, { id: 2, name: "Kinderen" }], active: 1 });
});

describe("ProfileSettings", () => {
  it("toont de profielen en schakelt over", async () => {
    const sw = vi.spyOn(api, "profileSwitch").mockResolvedValue(null);
    render(<ProfileSettings onSwitched={reload} />);
    fireEvent.click(await screen.findByRole("button", { name: "Wissel naar Kinderen" }));
    await waitFor(() => expect(sw).toHaveBeenCalledWith(2));
    await waitFor(() => expect(reload).toHaveBeenCalled());
  });
  it("kan het standaardprofiel niet verwijderen", async () => {
    render(<ProfileSettings onSwitched={reload} />);
    await screen.findByText("Kinderen");
    expect(screen.queryByRole("button", { name: "Verwijder Standaard" })).toBeNull();
    expect(screen.getByRole("button", { name: "Verwijder Kinderen" })).toBeTruthy();
  });
  it("maakt een nieuw profiel", async () => {
    const create = vi.spyOn(api, "profileCreate").mockResolvedValue(3);
    render(<ProfileSettings onSwitched={reload} />);
    fireEvent.change(await screen.findByLabelText("Naam nieuw profiel"), { target: { value: "Opa" } });
    fireEvent.click(screen.getByRole("button", { name: "Toevoegen" }));
    await waitFor(() => expect(create).toHaveBeenCalledWith("Opa"));
  });
});
