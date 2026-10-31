import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { GoToTimeDialog } from "./GoToTimeDialog";

describe("GoToTimeDialog", () => {
  it("springt naar de ingevoerde tijd", async () => {
    const onGo = vi.fn();
    render(<GoToTimeDialog lengthMs={7_200_000} onGo={onGo} onClose={() => undefined} />);
    await userEvent.type(screen.getByLabelText(/Tijd/), "1:23:45");
    await userEvent.click(screen.getByRole("button", { name: "Ga naar" }));
    expect(onGo).toHaveBeenCalledWith(5_025_000);
  });

  it("weigert ongeldige invoer met een duidelijke melding en een tijd voorbij het einde", async () => {
    const onGo = vi.fn();
    render(<GoToTimeDialog lengthMs={600_000} onGo={onGo} onClose={() => undefined} />);
    await userEvent.type(screen.getByLabelText(/Tijd/), "abc");
    await userEvent.click(screen.getByRole("button", { name: "Ga naar" }));
    expect(screen.getByRole("alert")).toHaveTextContent(/voorbeeld/i);
    await userEvent.clear(screen.getByLabelText(/Tijd/));
    await userEvent.type(screen.getByLabelText(/Tijd/), "59:00");
    await userEvent.click(screen.getByRole("button", { name: "Ga naar" }));
    expect(screen.getByRole("alert")).toHaveTextContent(/duurt maar/i);
    expect(onGo).not.toHaveBeenCalled();
  });

  it("bevestigt met Enter", async () => {
    const onGo = vi.fn();
    render(<GoToTimeDialog lengthMs={0} onGo={onGo} onClose={() => undefined} />);
    await userEvent.type(screen.getByLabelText(/Tijd/), "30{Enter}");
    expect(onGo).toHaveBeenCalledWith(30_000);
  });
});
