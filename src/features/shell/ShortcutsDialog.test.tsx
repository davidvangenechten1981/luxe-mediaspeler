import { render, screen } from "@testing-library/react";
import { DEFAULT_SETTINGS, useSettings } from "../../state/settings";
import { ShortcutsDialog } from "./ShortcutsDialog";

beforeEach(() => useSettings.setState({ settings: { ...DEFAULT_SETTINGS, shortcuts: { play_pause: "Space", fullscreen: "F", go_to_time: "Ctrl+T" } } }));

describe("ShortcutsDialog", () => {
  it("toont de ingestelde toetsen bij hun omschrijving", () => {
    render(<ShortcutsDialog onClose={() => undefined} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Afspelen / pauzeren").closest("tr")).toHaveTextContent("Space");
    expect(screen.getByText("Volledig scherm").closest("tr")).toHaveTextContent("F");
    expect(screen.getByText("Ga naar tijd").closest("tr")).toHaveTextContent("Ctrl+T");
  });

  it("noemt ook de vaste toetsen van de interface", () => {
    render(<ShortcutsDialog onClose={() => undefined} />);
    expect(screen.getByText("Mediagegevens tonen").closest("tr")).toHaveTextContent("I");
    expect(screen.getByText("Dit overzicht tonen").closest("tr")).toHaveTextContent("F1");
  });

  it("slaat acties zonder toets over", () => {
    render(<ShortcutsDialog onClose={() => undefined} />);
    expect(screen.queryByText("Dempen")).not.toBeInTheDocument();
  });
});
