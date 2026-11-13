// Channel guide fetching: one upstream request per channel per day, bounded
// concurrency and a short cache, because /epg/:id/:offset is proxied per
// channel and a guide listing must never fan out into one request per row.
import { api } from "./api.js";

const TTL = 5 * 60 * 1000;
const cache = new Map();

/** Drops every cached day. Called when the active provider changes. */
export function invalidateGuides() {
  cache.clear();
}

async function loadOne(id) {
  const hit = cache.get(id);
  if (hit && Date.now() - hit.at < TTL) {
    if (hit.error) throw hit.error;
    return hit.guide;
  }
  try {
    const data = await api(`/epg/${encodeURIComponent(id)}/0`);
    const guide = Array.isArray(data?.epg) ? data.epg : [];
    cache.set(id, { at: Date.now(), guide });
    return guide;
  } catch (error) {
    // Cached too: Playbox answers 501 by design and a failing channel should
    // not be retried on every render.
    cache.set(id, { at: Date.now(), error });
    throw error;
  }
}

/**
  Loads today's guide for `ids`, at most `limit` requests in flight.
  Returns `{ [channelId]: { guide } | { error } }`; a channel with no guide is
  a normal answer, not a failure of the whole screen.
*/
export async function loadGuides(ids, limit = 4) {
  const unique = [...new Set(ids.filter(Boolean))];
  const out = {};
  const queue = [...unique];
  const worker = async () => {
    for (let id = queue.shift(); id !== undefined; id = queue.shift()) {
      try {
        out[id] = { guide: await loadOne(id) };
      } catch (error) {
        out[id] = { error: error?.message || "Guide unavailable" };
      }
    }
  };
  const workers = Math.min(limit, unique.length);
  await Promise.all(Array.from({ length: workers }, worker));
  return out;
}
