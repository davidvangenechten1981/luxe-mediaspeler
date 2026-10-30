import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useSleep } from "../../state/sleep";
import { SleepMenu } from "./SleepMenu";

beforeEach(() => useSleep.setState({ timer: null }));

describe("SleepMenu", () => {
  it("start de timer met de gekozen tijd", async () => {
    render(<SleepMenu />);
    await userEvent.click(screen.getByRole("button", { name: "Slaaptimer" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "30 minuten" }));
    const t = useSleep.getState().timer!;
    expect(t.endsAt - Date.now()).toBeGreaterThan(29 * 60_000);
    expect(t.endsAt - Date.now()).toBeLessThanOrEqual(30 * 60_000);
  });

  it("biedt 'uitzetten' aan zodra er een timer loopt", async () => {
    useSleep.setState({ timer: { endsAt: Date.now() + 10 * 60_000, baseVolume: null } });
    render(<SleepMenu />);
    await userEvent.click(screen.getByRole("button", { name: /Slaaptimer/ }));
    await userEvent.click(screen.getByRole("menuitem", { name: /Slaaptimer uitzetten/ }));
    expect(useSleep.getState().timer).toBeNull();
  });
});
