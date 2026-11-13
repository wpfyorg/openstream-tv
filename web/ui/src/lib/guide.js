// Pure guide helpers. No fetching here, so the behaviour is testable without
// a server: startEpoch/endEpoch are milliseconds, as /epg/:id/:offset returns.

/** Languages can be absent on the beta providers; that gets its own filter value. */
export const UNSPECIFIED = "__unspecified";

// JioTV's language id 0 maps to this label, which is a category of channel
// rather than a language; it belongs in the Unspecified bucket, not beside the
// filter's own "All languages" reset.
const PSEUDO_LANGUAGES = new Set(["all languages"]);

/** The channel's real language, or "" when it has none. */
export function channelLanguage(channel) {
  const language = (channel?.language || "").trim();
  return PSEUDO_LANGUAGES.has(language.toLowerCase()) ? "" : language;
}

/** Sorted language labels present in the catalogue, pseudo-languages removed. */
export function languageLabels(channels) {
  const out = new Set();
  for (const channel of channels || []) {
    const language = channelLanguage(channel);
    if (language) out.add(language);
  }
  return [...out].sort();
}

/** The programme airing at `now`, or null. */
export function programAt(guide, now) {
  if (!Array.isArray(guide)) return null;
  return guide.find((p) => p.startEpoch <= now && p.endEpoch > now) ?? null;
}

/** The next programme to start after `now`, or null. */
export function nextProgram(guide, now) {
  if (!Array.isArray(guide)) return null;
  let best = null;
  for (const p of guide) {
    if (p.startEpoch > now && (!best || p.startEpoch < best.startEpoch)) best = p;
  }
  return best;
}

/** 0–100 through the programme, clamped; 0 for a missing or zero-length slot. */
export function programProgress(program, now) {
  if (!program) return 0;
  const duration = program.endEpoch - program.startEpoch;
  if (!(duration > 0)) return 0;
  const pct = ((now - program.startEpoch) / duration) * 100;
  return Math.max(0, Math.min(100, pct));
}

/**
  Multi-select language filter: an empty selection means every language, and
  channels without a language are matched only by the "Unspecified" value so a
  language filter never hides them silently.
*/
export function matchesLanguage(channel, selected, unspecified = UNSPECIFIED) {
  if (!Array.isArray(selected) || selected.length === 0) return true;
  const language = channelLanguage(channel);
  if (!language) return selected.includes(unspecified);
  return selected.includes(language);
}

/** "20:00" for an epoch, in the viewer's locale. */
export function formatClock(epoch) {
  return new Date(epoch).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** "20:00 – 22:30" for a programme, or "" when it has no times. */
export function formatRange(program) {
  if (!program || !program.startEpoch || !program.endEpoch) return "";
  return `${formatClock(program.startEpoch)} – ${formatClock(program.endEpoch)}`;
}

/**
  JioTV's guide reports artwork as a bare CDN path (`epgdata/<file>.jpg`), which
  a browser would resolve against the app's own origin and 404. The server
  proxies that CDN at `/jtvposter/`, so relative posters are routed through it;
  absolute URLs are left alone and anything else is dropped.
*/
export function posterUrl(poster) {
  if (!poster || typeof poster !== "string") return "";
  if (/^https?:\/\//i.test(poster) || poster.startsWith("//")) return poster;
  if (poster.startsWith("/")) return poster;
  if (poster.startsWith("epgdata/")) return "/jtvposter/" + poster;
  return "";
}
