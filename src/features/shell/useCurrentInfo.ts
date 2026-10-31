import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { MediaInfo } from "../../api/types";
import { isUrl } from "../../lib/format";
import { usePlayer } from "../../state/player";

/** Technische informatie (sporen, hoofdstukken) van het huidige bestand. */
export function useCurrentInfo(): { info: MediaInfo | null; loading: boolean } {
  const location = usePlayer((s) => s.view.location);
  const [state, setState] = useState<{ loc: string | null; info: MediaInfo | null; loading: boolean }>({ loc: null, info: null, loading: false });
  useEffect(() => {
    if (!location || isUrl(location)) {
      setState({ loc: location, info: null, loading: false });
      return;
    }
    let alive = true;
    setState((s) => ({ loc: location, info: s.loc === location ? s.info : null, loading: true }));
    api.mediaInfo(location).then(
      (info) => alive && setState({ loc: location, info, loading: false }),
      () => alive && setState({ loc: location, info: null, loading: false }),
    );
    return () => {
      alive = false;
    };
  }, [location]);
  return { info: state.loc === location ? state.info : null, loading: state.loading };
}
