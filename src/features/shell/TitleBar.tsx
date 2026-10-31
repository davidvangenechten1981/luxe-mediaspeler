import { useEffect, useState } from "react";
import { Icon } from "../../components/Icon";
import { windowApi } from "../../lib/platform";
import { useUi } from "../../state/ui";
import { IconButton } from "../../components/ui";

export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  const sidebarCollapsed = useUi((s) => s.sidebarCollapsed);
  const setSidebarCollapsed = useUi((s) => s.setSidebarCollapsed);

  useEffect(() => {
    const sync = () => void windowApi.isMaximized().then(setMaximized);
    sync();
    window.addEventListener("resize", sync);
    return () => window.removeEventListener("resize", sync);
  }, []);

  return (
    <header className="titlebar" data-tauri-drag-region onDoubleClick={() => void windowApi.toggleMaximize()}>
      <div className="titlebar__left" data-tauri-drag-region>
        <IconButton icon="list" label={sidebarCollapsed ? "Navigatie uitklappen" : "Navigatie inklappen"} size={18} onClick={() => setSidebarCollapsed(!sidebarCollapsed)} onDoubleClick={(e) => e.stopPropagation()} />
        <div className="brand" data-tauri-drag-region>
          <span className="brand__mark" aria-hidden="true">
            <Icon name="play" size={12} filled />
          </span>
          <span className="brand__name" data-tauri-drag-region>
            VLC Ultimate
          </span>
          <span className="brand__edition" data-tauri-drag-region>
            Studio
          </span>
        </div>
      </div>
      <div className="titlebar__controls" onDoubleClick={(e) => e.stopPropagation()}>
        <button className="icon-btn icon-btn--window" aria-label="Minimaliseren" title="Minimaliseren" onClick={() => void windowApi.minimize()}>
          <Icon name="minimize" size={16} />
        </button>
        <button className="icon-btn icon-btn--window" aria-label={maximized ? "Herstellen" : "Maximaliseren"} title={maximized ? "Herstellen" : "Maximaliseren"} onClick={() => void windowApi.toggleMaximize()}>
          <Icon name={maximized ? "restore" : "maximize"} size={14} />
        </button>
        <button className="icon-btn icon-btn--window is-close" aria-label="Sluiten" title="Sluiten" onClick={() => void windowApi.close()}>
          <Icon name="close" size={16} />
        </button>
      </div>
    </header>
  );
}
