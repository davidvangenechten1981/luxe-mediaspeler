import { render } from "@testing-library/react";
import { Slider } from "./ui";

const fill = (c: HTMLElement) => c.querySelector<HTMLElement>(".slider__fill")!;

describe("Slider met oorsprong (equalizer: nul in het midden)", () => {
  it("vult omhoog vanaf het midden bij een positieve waarde", () => {
    const { container } = render(<Slider vertical label="60 Hz" min={-20} max={20} value={10} origin={0} onChange={() => undefined} />);
    expect(fill(container).style.bottom).toBe("50%");
    expect(fill(container).style.height).toBe("25%");
  });
  it("vult omlaag vanaf het midden bij een negatieve waarde", () => {
    const { container } = render(<Slider vertical label="60 Hz" min={-20} max={20} value={-10} origin={0} onChange={() => undefined} />);
    expect(fill(container).style.bottom).toBe("25%");
    expect(fill(container).style.height).toBe("25%");
  });
  it("toont niets bij precies nul", () => {
    const { container } = render(<Slider vertical label="60 Hz" min={-20} max={20} value={0} origin={0} onChange={() => undefined} />);
    expect(fill(container).style.height).toBe("0%");
  });
  it("werkt ook horizontaal", () => {
    const { container } = render(<Slider label="Balans" min={-10} max={10} value={5} origin={0} onChange={() => undefined} />);
    expect(fill(container).style.left).toBe("50%");
    expect(fill(container).style.width).toBe("25%");
  });
  it("blijft zonder oorsprong vullen vanaf het begin", () => {
    const { container } = render(<Slider label="Volume" max={100} value={40} onChange={() => undefined} />);
    expect(fill(container).style.width).toBe("40%");
  });
});
