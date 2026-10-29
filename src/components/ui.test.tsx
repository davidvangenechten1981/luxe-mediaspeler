import { render, screen, fireEvent, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { ConfirmDialog, MenuButton, Modal, ProgressBar, Slider, Switch, Tabs } from "./ui";

function Controlled({ onCommit }: { onCommit?: (v: number) => void }) {
  const [v, setV] = useState(50);
  return <Slider label="Volume" value={v} max={100} step={1} bigStep={10} onChange={setV} onCommit={onCommit} />;
}

describe("Slider", () => {
  it("is bedienbaar met het toetsenbord en meldt de commit", async () => {
    const commit = vi.fn();
    render(<Controlled onCommit={commit} />);
    const s = screen.getByRole("slider", { name: "Volume" });
    expect(s).toHaveAttribute("aria-valuenow", "50");
    s.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(s).toHaveAttribute("aria-valuenow", "51");
    await userEvent.keyboard("{PageUp}");
    expect(s).toHaveAttribute("aria-valuenow", "61");
    await userEvent.keyboard("{Shift>}{ArrowLeft}{/Shift}");
    expect(s).toHaveAttribute("aria-valuenow", "51");
    await userEvent.keyboard("{Home}");
    expect(s).toHaveAttribute("aria-valuenow", "0");
    await userEvent.keyboard("{ArrowLeft}"); // niet onder het minimum
    expect(s).toHaveAttribute("aria-valuenow", "0");
    await userEvent.keyboard("{End}");
    expect(s).toHaveAttribute("aria-valuenow", "100");
    expect(commit).toHaveBeenLastCalledWith(100);
  });

  it("doet niets als hij is uitgeschakeld", async () => {
    const onChange = vi.fn();
    render(<Slider label="Positie" value={0} max={10} onChange={onChange} disabled />);
    const s = screen.getByRole("slider");
    expect(s).toHaveAttribute("aria-disabled", "true");
    fireEvent.keyDown(s, { key: "ArrowRight" });
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("Switch en Tabs", () => {
  it("schakelaar meldt de nieuwe waarde", async () => {
    const on = vi.fn();
    render(<Switch checked={false} onChange={on} label="Geschiedenis" description="Onthoud wat je hebt gekeken" />);
    await userEvent.click(screen.getByRole("switch", { name: /Geschiedenis/ }));
    expect(on).toHaveBeenCalledWith(true);
    expect(screen.getByText("Onthoud wat je hebt gekeken")).toBeInTheDocument();
  });

  it("tabs wisselen met pijltoetsen en wikkelen rond", async () => {
    const on = vi.fn();
    render(<Tabs label="Studio" tabs={[{ id: "a", label: "A" }, { id: "b", label: "B" }]} value="b" onChange={on} />);
    const tab = screen.getByRole("tab", { name: "B" });
    expect(tab).toHaveAttribute("aria-selected", "true");
    tab.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(on).toHaveBeenCalledWith("a");
  });
});

describe("Modal en ConfirmDialog", () => {
  it("sluit met Escape, houdt de focus binnen en zet de focus terug", async () => {
    const onClose = vi.fn();
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();
    const { unmount } = render(
      <Modal title="Titel" onClose={onClose} footer={<button>Opslaan</button>}>
        <input aria-label="Naam" />
      </Modal>,
    );
    expect(screen.getByRole("dialog", { name: "Titel" })).toBeInTheDocument();
    expect(screen.getByLabelText("Naam")).toHaveFocus();
    await userEvent.tab();
    await userEvent.tab();
    await userEvent.tab();
    expect(screen.getByRole("dialog").contains(document.activeElement)).toBe(true);
    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalled();
    unmount();
    expect(opener).toHaveFocus();
    opener.remove();
  });

  it("ConfirmDialog roept de juiste callback aan", async () => {
    const yes = vi.fn();
    const no = vi.fn();
    render(<ConfirmDialog title="Wissen?" text="Weet je het zeker?" confirmLabel="Wissen" danger onConfirm={yes} onCancel={no} />);
    await userEvent.click(screen.getByRole("button", { name: "Wissen" }));
    expect(yes).toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Annuleren" }));
    expect(no).toHaveBeenCalled();
  });
});

describe("MenuButton", () => {
  it("opent, navigeert met pijltoetsen en kiest een item", async () => {
    const a = vi.fn();
    const b = vi.fn();
    render(
      <MenuButton
        label="Meer"
        entries={[
          { id: "h", label: "Kop", heading: true },
          { id: "a", label: "Eerste", onSelect: a },
          { id: "x", label: "Uit", disabled: true },
          { id: "b", label: "Tweede", onSelect: b },
        ]}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Meer" }));
    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Eerste" })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}"); // het uitgeschakelde item wordt overgeslagen
    expect(screen.getByRole("menuitem", { name: "Tweede" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(b).toHaveBeenCalled();
    expect(a).not.toHaveBeenCalled();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("sluit met Escape", async () => {
    render(<MenuButton label="Meer" entries={[{ id: "a", label: "Eerste" }]} />);
    await userEvent.click(screen.getByRole("button", { name: "Meer" }));
    await act(async () => {
      await userEvent.keyboard("{Escape}");
    });
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});

describe("ProgressBar", () => {
  it("toont een percentage of is onbepaald", () => {
    const { rerender } = render(<ProgressBar value={0.4} label="Voortgang" />);
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "40");
    rerender(<ProgressBar value={null} label="Voortgang" />);
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  });
});
