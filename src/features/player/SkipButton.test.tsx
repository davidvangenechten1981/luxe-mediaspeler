import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { SkipButton, skipLabel } from "./SkipButton";

describe("SkipButton", () => {
  it("toont niets buiten een gemarkeerd stuk", () => {
    const { container } = render(<SkipButton offer={null} onSkip={() => undefined} />);
    expect(container).toBeEmptyDOMElement();
  });
  it("heeft duidelijke teksten voor intro en aftiteling", () => {
    expect(skipLabel("intro")).toBe("Intro overslaan");
    expect(skipLabel("credits")).toBe("Naar volgende");
  });
  it("roept de actie aan bij een klik", () => {
    const onSkip = vi.fn();
    render(<SkipButton offer="intro" onSkip={onSkip} />);
    fireEvent.click(screen.getByRole("button", { name: "Intro overslaan" }));
    expect(onSkip).toHaveBeenCalledTimes(1);
  });
});
