import { useState } from "react";
import { Icon } from "../../components/Icon";
import { Button, EmptyState, IconButton } from "../../components/ui";
import { stem } from "../../lib/format";
import { usePlayer } from "../../state/player";

/** De afspeelwachtrij met slepen om te herordenen. */
export function QueueList({ compact }: { compact?: boolean }) {
  const queue = usePlayer((s) => s.queue);
  const index = usePlayer((s) => s.view.queue_index);
  const send = usePlayer((s) => s.send);
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [over, setOver] = useState<number | null>(null);

  if (queue.length === 0) {
    return (
      <EmptyState icon="queue" title="De wachtrij is leeg">
        {compact ? "Speel iets af of voeg bestanden toe." : "Speel bestanden af of voeg ze toe via het menu bij een bestand."}
      </EmptyState>
    );
  }
  return (
    <div>
      <div className="row" style={{ justifyContent: "space-between", marginBottom: "var(--s-2)" }}>
        <span className="muted">{queue.length} {queue.length === 1 ? "item" : "items"}</span>
        <Button variant="ghost" size="sm" icon="trash" onClick={() => void send({ type: "clear_queue" })}>
          Leegmaken
        </Button>
      </div>
      <div className="list" role="list" aria-label="Afspeelwachtrij">
        {queue.map((q, i) => (
          <div
            key={`${q.location}-${i}`}
            role="listitem"
            className={`list-item ${i === index ? "is-active" : ""} ${over === i && dragFrom !== null && dragFrom !== i ? "is-drop-target" : ""}`}
            draggable
            onDragStart={() => setDragFrom(i)}
            onDragOver={(e) => (e.preventDefault(), setOver(i))}
            onDragEnd={() => (setDragFrom(null), setOver(null))}
            onDrop={() => {
              if (dragFrom !== null && dragFrom !== i) void send({ type: "move_in_queue", from: dragFrom, to: i });
              setDragFrom(null);
              setOver(null);
            }}
          >
            <Icon name="drag" size={16} style={{ color: "var(--text-faint)", cursor: "grab", flex: "none" }} />
            <span className="list-item__idx">{i === index ? <Icon name="play" size={12} filled /> : i + 1}</span>
            <button type="button" className="grow ellipsis" style={{ background: "none", border: 0, padding: 0, textAlign: "left" }} onClick={() => void send({ type: "jump_to", index: i })} title={q.location}>
              {q.title || stem(q.location)}
            </button>
            <IconButton icon="close" label="Uit wachtrij verwijderen" size={15} onClick={() => void send({ type: "remove_from_queue", index: i })} />
          </div>
        ))}
      </div>
    </div>
  );
}
