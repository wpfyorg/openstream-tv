// Watch history: the last channels played, most recent first. The home rail
// and the channel list use it as their order, so the app reflects what this
// viewer actually watches; a fresh install simply falls back to catalogue
// order for everything it has not seen yet.
const KEY = "watchHistory";
const LIMIT = 40;

/** Reads the stored history, ignoring anything malformed. */
export function readHistory() {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || "[]");
    if (!Array.isArray(raw)) return [];
    return raw
      .filter((entry) => entry && typeof entry.id === "string" && entry.id)
      .map((entry) => ({ id: entry.id, at: Number(entry.at) || 0 }));
  } catch {
    return [];
  }
}

/** Records a channel as watched now, moving it to the front of the history. */
export function recordWatch(id, at = Date.now()) {
  if (!id) return;
  const next = [{ id, at }, ...readHistory().filter((entry) => entry.id !== id)].slice(0, LIMIT);
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {}
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("openstream:history-changed"));
  }
}

/**
  Orders `channels` so previously watched ones lead, most recent first, and
  everything else keeps its catalogue order after them.
*/
export function orderByHistory(channels, history = readHistory()) {
  const rank = new Map();
  history.forEach((entry, index) => {
    if (!rank.has(entry.id)) rank.set(entry.id, index);
  });
  const watched = [];
  const rest = [];
  for (const channel of channels) {
    (rank.has(channel.id) ? watched : rest).push(channel);
  }
  watched.sort((a, b) => rank.get(a.id) - rank.get(b.id));
  return [...watched, ...rest];
}
