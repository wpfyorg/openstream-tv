// The home screen's rows: an "On now" strip plus a few category rails, ordered
// by what this viewer actually watches. Pure so it can be tested and so the
// guide prefetch uses exactly the channels that get rendered.
import { orderByHistory } from "./history.js";

const ON_NOW = 6;
const PER_RAIL = 6;
// One guide request per rendered channel, with headroom for overlap.
const MAX_GUIDE_REQUESTS = 60;

// The catalogue categories the home screen browses, in their default order.
// A provider whose catalogue uses other names still gets rails, just from its
// own categories.
export const HOME_CATEGORIES = [
  "Entertainment",
  "Infotainment",
  "Kids",
  "Lifestyle",
  "Movies",
  "Music",
  "News",
  "Sports",
];

/** Categories that have more than one channel, most-watched first. */
export function categoryRails(channels, heroId = "", history = []) {
  const byId = new Map(channels.map((c) => [c.id, c]));
  const watched = new Map();
  for (const entry of history) {
    const category = byId.get(entry.id)?.category;
    if (category && category !== "All Categories") {
      watched.set(category, (watched.get(category) ?? 0) + 1);
    }
  }
  const categories = [...new Set(channels.map((c) => c.category))].filter(
    (category) => category && category !== "All Categories",
  );
  return categories
    .map((category) => ({
      title: category,
      watched: watched.get(category) ?? 0,
      items: channels.filter((c) => c.category === category && c.id !== heroId),
    }))
    .filter((entry) => entry.items.length > 1)
    .filter((entry, _, all) => !all.some((other) => HOME_CATEGORIES.includes(other.title)) || HOME_CATEGORIES.includes(entry.title))
    .sort(
      (a, b) =>
        b.watched - a.watched ||
        HOME_CATEGORIES.indexOf(a.title) - HOME_CATEGORIES.indexOf(b.title) ||
        b.items.length - a.items.length,
    )
    .map((entry) => ({ title: entry.title, items: orderByHistory(entry.items, history).slice(0, PER_RAIL) }));
}

/** Everything the home screen renders, and the channel ids it needs guides for. */
export function homeSelection(channels, heroId = "", history = []) {
  const others = channels.filter((c) => c.id !== heroId);
  const onNow = orderByHistory(others, history).slice(0, ON_NOW);
  const rails = categoryRails(channels, heroId, history);
  const ids = [
    ...new Set([heroId, ...onNow.map((c) => c.id), ...rails.flatMap((r) => r.items.map((c) => c.id))].filter(Boolean)),
  ].slice(0, MAX_GUIDE_REQUESTS);
  return { onNow, rails, ids };
}
