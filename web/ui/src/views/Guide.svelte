<script>
  import { onMount } from "svelte";
  import { loadChannels } from "../lib/api.js";
  import { invalidateGuides, loadGuides } from "../lib/epg.js";
  import {
    channelLanguage,
    formatRange,
    languageLabels,
    matchesLanguage,
    programAt,
    programProgress,
    UNSPECIFIED,
  } from "../lib/guide.js";
  import Icon from "../lib/Icon.svelte";

  // Rows render in pages and guides are fetched per row, only once the row is
  // near the viewport. 248 channels must never mean 248 EPG requests on load.
  const PAGE = 24;
  const BATCH_MS = 60;

  let channels = $state([]);
  let guides = $state({});
  let loading = $state(true);
  let error = $state("");
  let languages = $state([]);
  let selected = $state([]); // empty selection = all languages
  let rendered = $state(PAGE);
  let now = $state(Date.now());

  const pending = new Set();
  const queued = new Set();
  let flushTimer = null;

  const playable = $derived(channels.filter((c) => c.playable));
  const hasUnspecified = $derived(playable.some((c) => !channelLanguage(c)));
  const shown = $derived(playable.filter((c) => matchesLanguage(c, selected, UNSPECIFIED)));
  const windowed = $derived(shown.slice(0, rendered));
  // A provider catalogue that reports one language (VZY free tier, for one)
  // has nothing to filter, so the chip row is only shown when it can differ.
  const filterable = $derived(languages.length + (hasUnspecified ? 1 : 0) > 1);

  function watchHref(id) {
    return "#/watch/" + encodeURIComponent(id);
  }

  function toggleLanguage(value) {
    selected = selected.includes(value)
      ? selected.filter((v) => v !== value)
      : [...selected, value];
    rendered = PAGE;
    primeWindow();
  }

  function clearLanguages() {
    selected = [];
    rendered = PAGE;
    primeWindow();
  }

  // Requests are coalesced into one batch and spread a few at a time, so the
  // rendered window costs at most one request per channel and never a fan-out.
  function requestGuides(ids) {
    for (const id of ids) {
      if (!id || guides[id] || pending.has(id) || queued.has(id)) continue;
      queued.add(id);
    }
    if (!queued.size || flushTimer) return;
    flushTimer = window.setTimeout(async () => {
      flushTimer = null;
      const batch = [...queued];
      queued.clear();
      if (!batch.length) return;
      for (const id of batch) pending.add(id);
      try {
        const next = await loadGuides(batch, 4);
        guides = { ...guides, ...next };
      } finally {
        for (const id of batch) pending.delete(id);
      }
    }, BATCH_MS);
  }

  // Every rendered row is fetched eagerly: the window *is* the budget, so the
  // list never depends on scroll observation to fill in.
  function primeWindow() {
    requestGuides(windowed.map((c) => c.id));
  }

  /** Extends the rendered window and loads the guide for the rows it adds. */
  function grow() {
    if (rendered >= shown.length) return;
    const from = rendered;
    rendered = from + PAGE;
    requestGuides(shown.slice(from, rendered).map((c) => c.id));
  }

  /** Svelte action: extend the rendered window when the end of the list is near. */
  function moreRows(node) {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) grow();
      },
      { rootMargin: "240px 0px" },
    );
    observer.observe(node);
    return {
      destroy() {
        observer.disconnect();
      },
    };
  }

  async function load() {
    loading = true;
    error = "";
    try {
      channels = await loadChannels();
      languages = languageLabels(channels.filter((c) => c.playable));
      primeWindow();
    } catch (err) {
      error = err.message;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    const ticker = setInterval(() => (now = Date.now()), 15000);
    const onProvider = () => {
      invalidateGuides();
      guides = {};
      selected = [];
      rendered = PAGE;
      load();
    };
    window.addEventListener("openstream:provider-changed", onProvider);
    load();
    return () => {
      clearInterval(ticker);
      clearTimeout(flushTimer);
      flushTimer = null;
      queued.clear();
      window.removeEventListener("openstream:provider-changed", onProvider);
    };
  });
</script>

<div class="bar-row" class:solo={!filterable}>
  {#if filterable}
    <div class="chips" role="group" aria-label="Filter by language">
      <button class="chip" aria-pressed={selected.length === 0} onclick={clearLanguages}>
        All languages
      </button>
      {#each languages as name (name)}
        <button class="chip" aria-pressed={selected.includes(name)} onclick={() => toggleLanguage(name)}>
          {name}
        </button>
      {/each}
      {#if hasUnspecified}
        <button
          class="chip"
          aria-pressed={selected.includes(UNSPECIFIED)}
          onclick={() => toggleLanguage(UNSPECIFIED)}
        >
          Unspecified
        </button>
      {/if}
    </div>
  {/if}
  {#if !loading && !error}
    <p class="muted small count">
      {#if selected.length}
        {shown.length} of {playable.length} channels
      {:else}
        {playable.length} channels
      {/if}
      · today
    </p>
  {/if}
</div>

{#if loading}
  <p class="muted">Loading guide…</p>
{:else if error}
  <p class="error" role="alert">Couldn't load the guide: {error}</p>
{:else if !playable.length}
  <div class="empty">
    <p class="muted">No playable channels for the active provider. Check <a href="#/settings">Settings</a>.</p>
  </div>
{:else if !shown.length}
  <p class="muted">No channels in the selected languages.</p>
{:else}
  <ul class="rows">
    {#each windowed as c (c.id)}
      {@const program = programAt(guides[c.id]?.guide, now)}
      <li>
        <a class="row" href={watchHref(c.id)}>
          <span class="logo">
            <span class="letter" aria-hidden="true">{c.name.slice(0, 1)}</span>
            {#if c.logo}<img
                src={c.logo}
                alt=""
                loading="lazy"
                decoding="async"
                onerror={(event) => event.currentTarget.remove()}
              />{/if}
          </span>
          <span class="copy">
            <span class="name">{c.name}{#if c.hd}<span class="badge">HD</span>{/if}</span>
            <span class="meta">{[channelLanguage(c), c.category].filter(Boolean).join(" · ") || "Live TV"}</span>
          </span>
          <span class="now">
            {#if program}
              <span class="title">{program.showname}</span>
              <span class="range muted">{formatRange(program)}</span>
              <span class="progress" aria-hidden="true">
                <span style:width={`${programProgress(program, now)}%`}></span>
              </span>
            {:else if guides[c.id]?.error}
              <span class="muted">Guide unavailable</span>
            {:else}
              <span class="muted">Loading guide…</span>
            {/if}
          </span>
          <span class="play" aria-hidden="true"><Icon name="play_arrow" size={18} /></span>
        </a>
      </li>
    {/each}
  </ul>

  {#if rendered < shown.length}
    <div class="more" use:moreRows>
      <button class="btn" onclick={grow}>Show more channels</button>
    </div>
  {:else}
    <p class="muted small end">End of the channel list.</p>
  {/if}
{/if}

<style>
  .bar-row { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin-bottom: 14px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip {
    min-height: 34px;
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .chip:hover { color: var(--text); }
  .chip[aria-pressed="true"] {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  }
  .count { margin: 0 0 0 auto; }
  .bar-row.solo .count { margin-left: 0; }
  .small { font-size: 12px; }

  .rows { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    color: var(--text);
    text-decoration: none;
  }
  .row:hover, .row:focus-visible { border-color: color-mix(in srgb, var(--accent) 50%, var(--border)); }
  .logo {
    position: relative;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 56px;
    height: 40px;
    border-radius: 8px;
    background: #1d2230;
    overflow: hidden;
  }
  .letter { font-size: 16px; font-weight: 800; color: color-mix(in srgb, var(--text) 35%, transparent); }
  .logo img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; padding: 4px; }
  .copy { display: grid; gap: 2px; flex: 0 0 210px; min-width: 0; }
  .name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    font-weight: 650;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .meta { font-size: 12px; color: var(--muted); }
  .now { display: grid; gap: 3px; flex: 1 1 auto; min-width: 0; }
  .title { font-size: 13px; font-weight: 600; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .range { font-size: 11px; font-variant-numeric: tabular-nums; }
  .progress {
    display: block;
    max-width: 260px;
    height: 3px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 12%, transparent);
    overflow: hidden;
  }
  .progress span { display: block; height: 100%; border-radius: 999px; background: var(--accent); }
  .play { display: grid; place-items: center; color: var(--muted); }
  .row:hover .play { color: var(--accent); }
  .more { display: flex; justify-content: center; padding: 18px 0 4px; }
  .end { margin: 14px 0 0; }
  .empty { padding: 20px; border: 1px solid var(--border); border-radius: 12px; background: var(--surface); }

  @media (max-width: 720px) {
    .row { flex-wrap: wrap; gap: 10px; }
    .copy { flex: 1 1 140px; }
    .now { flex: 1 1 100%; }
    .play { display: none; }
    .count { margin-left: 0; }
  }
</style>
