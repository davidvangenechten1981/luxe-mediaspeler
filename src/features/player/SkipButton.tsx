import { Button } from "../../components/ui";

export function skipLabel(offer: "intro" | "credits"): string {
  return offer === "intro" ? "Intro overslaan" : "Naar volgende";
}

/** Verschijnt alleen binnen een door jou gemarkeerde intro of aftiteling. */
export function SkipButton({ offer, onSkip }: { offer: "intro" | "credits" | null; onSkip: () => void }) {
  if (!offer) return null;
  return (
    <div className="skip-button" onClick={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
      <Button variant="primary" onClick={onSkip}>{skipLabel(offer)}</Button>
    </div>
  );
}
