import { Icon, type IconName } from "../../components/Icon";
import { PAGES, useUi, type PageId } from "../../state/ui";
import { useJobs } from "../../state/jobs";
import { usePlayer } from "../../state/player";

const ICONS: Record<PageId, IconName> = {
  home: "home",
  now_playing: "play",
  videos: "film",
  music: "music",
  all_media: "library",
  series: "film",
  playlists: "list",
  favorites: "heart",
  recent: "clock",
  continue: "continue",
  audio_studio: "sliders",
  video_studio: "wand",
  converter: "convert",
  queue: "queue",
  history: "history",
  settings: "settings",
};

export function Sidebar() {
  const page = useUi((s) => s.page);
  const go = useUi((s) => s.go);
  const collapsed = useUi((s) => s.sidebarCollapsed);
  const activeJobs = useJobs((s) => s.jobs.filter((j) => j.state === "running" || j.state === "queued").length);
  const queueLen = usePlayer((s) => s.view.queue_len);

  const groups = PAGES.reduce<Record<string, (typeof PAGES)[number][]>>((acc, p) => {
    (acc[p.group] ??= []).push(p);
    return acc;
  }, {});

  return (
    <nav className={`sidebar ${collapsed ? "is-collapsed" : ""}`} aria-label="Hoofdnavigatie">
      {Object.entries(groups).map(([group, pages]) => (
        <div key={group} className="sidebar__group">
          {!collapsed && <div className="sidebar__heading">{group}</div>}
          {pages.map((p) => {
            const badge = p.id === "queue" ? activeJobs + queueLen : 0;
            return (
              <button key={p.id} type="button" className={`nav-item ${page === p.id ? "is-active" : ""}`} aria-current={page === p.id ? "page" : undefined} title={collapsed ? p.label : undefined} onClick={() => go(p.id)}>
                <Icon name={ICONS[p.id]} size={19} />
                {!collapsed && <span className="grow ellipsis">{p.label}</span>}
                {badge > 0 && <span className="nav-item__badge">{badge}</span>}
              </button>
            );
          })}
        </div>
      ))}
    </nav>
  );
}
