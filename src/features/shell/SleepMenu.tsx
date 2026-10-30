import { MenuButton, type MenuEntry } from "../../components/ui";
import { useSleep } from "../../state/sleep";

const CHOICES = [15, 30, 45, 60, 90];

/** Menuknop om af te spelen te laten stoppen na een tijd (met zacht uitfaden). */
export function SleepMenu() {
  const timer = useSleep((s) => s.timer);
  const start = useSleep((s) => s.start);
  const cancel = useSleep((s) => s.cancel);
  const entries: MenuEntry[] = CHOICES.map((m) => ({ id: String(m), label: `${m} minuten`, onSelect: () => start(m) }));
  if (timer) entries.unshift({ id: "off", label: `Slaaptimer uitzetten (nog ${Math.max(1, Math.ceil((timer.endsAt - Date.now()) / 60_000))} min)`, onSelect: cancel });
  return <MenuButton icon="clock" label="Slaaptimer" placement="above" pressed={timer !== null} entries={entries} />;
}
