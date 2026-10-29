// SPIKE: bewijst dat interface en LibVLC-video samen op Windows werken
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function App() {
  useEffect(() => {
    void invoke("ui_ready", { note: `${window.innerWidth}x${window.innerHeight}` });
  }, []);
  return (
    <>
      <div className="spike-side">Zijbalk</div>
      <div className="spike-box" data-testid="ui-box">Interface zichtbaar</div>
    </>
  );
}
