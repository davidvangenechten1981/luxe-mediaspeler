import type { CapabilityOption, HwAccel } from "../api/types";

export interface HwChoice {
  value: HwAccel;
  label: string;
}

/** Keuzes voor "Videokaart": altijd Automatisch en Alleen processor, plus elke videokaart die echt werkt. */
export function hwChoices(gpus: CapabilityOption[]): HwChoice[] {
  const named: Record<string, HwAccel> = { nvenc: "nvenc", qsv: "qsv", amf: "amf" };
  return [
    { value: "auto", label: gpus.length > 0 ? `Automatisch (${gpus[0]!.label})` : "Automatisch (geen videokaart gevonden: processor)" },
    { value: "off", label: "Alleen processor" },
    ...gpus.filter((g) => named[g.id]).map((g) => ({ value: named[g.id]!, label: g.label })),
  ];
}

/** Uitleg onder het veld, zodat duidelijk is wat er gebeurt. */
export function hwHint(value: HwAccel, gpus: CapabilityOption[]): string {
  if (value === "off") return "Alleen de processor codeert. Meestal trager, maar de kwaliteit per bestandsgrootte is het best.";
  if (gpus.length === 0 && value === "auto") return "Er is geen bruikbare videokaart gevonden; de processor wordt gebruikt.";
  return "De videokaart codeert veel sneller. Mislukt dat, dan neemt de processor het automatisch over.";
}

/** Tekst voor het veld "Threads": leeg betekent dat de encoder alle kernen gebruikt. */
export function threadsHint(cpuThreads: number): string {
  return `Leeg = alle ${cpuThreads} processorthreads gebruiken`;
}
