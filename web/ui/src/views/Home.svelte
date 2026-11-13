<script>
  import { onMount } from "svelte";
  import { loadChannels } from "../lib/api.js";
  import { invalidateGuides, loadGuides } from "../lib/epg.js";
  import { formatClock, formatRange, posterUrl, programAt, programProgress } from "../lib/guide.js";
  import { orderByHistory, readHistory } from "../lib/history.js";
  import { homeSelection } from "../lib/home.js";
  import PosterCard from "../lib/PosterCard.svelte";
  import Icon from "../lib/Icon.svelte";

  // Remembering the last channel makes the hero the one you actually watch,
  // which is also what the click-to-play card promises.
  const LAST_CHANNEL = "lastChannel";

  let channels = $state([]);
  let guides = $state({});
  let loading = $state(true);
  let error = $state("");
  let now = $state(Date.now());
  let heroId = $state("");
  let history = $state([]);
  // Bumped by every load, so a load that started before a provider change
  // cannot overwrite the catalogue or guides of the newer one.
  let loadGeneration = 0;

  const playable = $derived(channels.filter((c) => c.playable));
  const hero = $derived(playable.find((c) => c.id === heroId) ?? playable[0] ?? null);
  // Most recently watched channels lead the rail; unseen ones keep catalogue
  // order behind them, so a fresh install looks the same as before.
  const rail = $derived(orderByHistory(playable.filter((c) => c.id !== hero?.id), history).slice(0, 6));
  // On now and the category rails share one selection, so the guide prefetch
  // and the rendered rows can never disagree.
  const selection = $derived(homeSelection(playable, hero?.id ?? "", history));
  const onNow = $derived(selection.onNow);
  const categoryRows = $derived(selection.rails);
  const heroProgram = $derived(programAt(guides[hero?.id]?.guide, now));
  // The hero thumbnail shows tonight's programme artwork when the provider
  // publishes a poster, and the channel logo when it does not.
  // Tonight's artwork becomes the card's background, like a TV poster wall.
  // JioTV reports it as a bare CDN path, so it is routed through the proxy.
  const heroArt = $derived(posterUrl(heroProgram?.episodePoster));

  function guideOf(id) {
    return guides[id]?.guide ?? null;
  }

  function guideFailed(id) {
    return Boolean(guides[id]?.error);
  }

  function watchHref(id) {
    return "#/watch/" + encodeURIComponent(id);
  }

  async function load() {
    const mine = ++loadGeneration;
    loading = true;
    error = "";
    try {
      const list = await loadChannels();
      if (mine !== loadGeneration) return;
      channels = list;
      const next = list.filter((c) => c.playable);
      let saved = "";
      try {
        saved = localStorage.getItem(LAST_CHANNEL) || "";
      } catch {}
      heroId = next.some((c) => c.id === saved) ? saved : (next[0]?.id ?? "");
      // Everything rendered below reads this same snapshot, including the
      // guide prefetch, so a recently watched channel in On now is requested
      // alongside the catalogue-first rows.
      history = readHistory();
      const ids = homeSelection(next, heroId, history).ids;
      // Each rail is six cards, so several rows' guides would hold the whole
      // screen back. Paint the rows first with channel artwork, then let the
      // programme posters fill in for exactly the channels shown.
      loading = false;
      if (ids.length) {
        try {
          const nextGuides = await loadGuides(ids, 6);
          if (mine === loadGeneration) guides = nextGuides;
        } catch {}
      }
    } catch (err) {
      if (mine !== loadGeneration) return;
      error = err.message;
    } finally {
      if (mine === loadGeneration) loading = false;
    }
  }

  onMount(() => {
    // The clock advances locally; guides are fetched once and refreshed when
    // the provider changes, not on every tick.
    const tick = setInterval(() => (now = Date.now()), 15000);
    const onProvider = () => {
      invalidateGuides();
      guides = {};
      load();
    };
    window.addEventListener("openstream:provider-changed", onProvider);
    load();
    return () => {
      clearInterval(tick);
      window.removeEventListener("openstream:provider-changed", onProvider);
    };
  });
</script>

<section class="home">
  <div class="main">
    {#if loading}
      <div class="hero skeleton" aria-busy="true"><span class="sr-only">Loading live TV</span></div>
      <div class="on-now-row">
        {#each [0, 1, 2] as i (i)}
          <div class="show-card skeleton" aria-hidden="true"></div>
        {/each}
      </div>
    {:else if error}
      <div class="panel">
        <h2>Live TV is unavailable</h2>
        <p class="muted">The channel catalogue could not be loaded: {error}</p>
        <button class="btn" onclick={load}>Try again</button>
      </div>
    {:else if !hero}
      <div class="panel">
        <h2>No channels yet</h2>
        <p class="muted">
          This account has no playable channels. Connect a provider in Settings, then come back here.
        </p>
        <a class="btn primary" href="#/settings">Open Settings</a>
      </div>
    {:else}
      <a
        class="hero"
        class:has-art={!!heroArt}
        href={watchHref(hero.id)}
        aria-label={`Watch ${hero.name} live`}
        style={heroArt ? `--hero-art: url("${heroArt}")` : ""}
      >
        <div class="hero-top">
          <span class="live-pill"><span class="live-dot"></span>LIVE</span>
          {#if hero.hd}<span class="tag">HD</span>{/if}
          {#if hero.requiresSubscription}<span class="tag warn">Subscription may be required</span>{/if}
        </div>
        <span class="hero-play"><Icon name="play_arrow" size={32} /></span>
        <div class="hero-meta">
          <p class="hero-name">{hero.name}</p>
          <p class="hero-program">
            {#if heroProgram}
              {heroProgram.showname}
            {:else if guideFailed(hero.id)}
              Guide unavailable for this channel
            {:else}
              {hero.category || "Live"}
            {/if}
          </p>
          {#if heroProgram}
            <span class="bar" aria-hidden="true">
              <span class="fill" style:width={`${programProgress(heroProgram, now)}%`}></span>
            </span>
            <span class="hero-times">
              <span>{formatRange(heroProgram)}</span>
              <span class="muted">{formatClock(now)}</span>
            </span>
          {/if}
          <span class="hero-cta">Watch live <Icon name="chevron_right" size={16} /></span>
        </div>
      </a>

      {#if onNow.length}
        <section class="on-now" aria-labelledby="on-now-title">
          <div class="section-head">
            <h2 id="on-now-title">On now</h2>
            <a class="link" href="#/guide">Full guide</a>
          </div>
          <ul class="poster-row on-now-row">
            {#each onNow as c (c.id)}
              {@const program = programAt(guideOf(c.id), now)}
              <li>
                <PosterCard
                  href={watchHref(c.id)}
                  time={program ? formatRange(program) : guideFailed(c.id) ? "No guide" : "Guide loading"}
                  title={program?.showname ?? c.name}
                  subtitle={program ? c.name : c.category || c.language || "Live TV"}
                  progress={program ? programProgress(program, now) : null}
                  art={posterUrl(program?.episodePoster) || c.logo || ""}
                  fit={program?.episodePoster ? "cover" : "logo"}
                />
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#each categoryRows as row (row.title)}
        <section class="category" aria-labelledby={`row-${row.title}`}>
          <div class="section-head">
            <h2 id={`row-${row.title}`}>{row.title}</h2>
            <a class="link" href="#/channels">All channels</a>
          </div>
          <ul class="poster-row category-row">
            {#each row.items as c (c.id)}
              {@const program = programAt(guideOf(c.id), now)}
              <li>
                <PosterCard
                  href={watchHref(c.id)}
                  title={c.name}
                  subtitle={program?.showname ?? c.language ?? c.category ?? "Live TV"}
                  art={posterUrl(program?.episodePoster) || c.logo || ""}
                  fit={program?.episodePoster ? "cover" : "logo"}
                />
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    {/if}
  </div>

  <aside class="rail" aria-labelledby="rail-title">
    <div class="rail-head">
      <h2 id="rail-title">Channels</h2>
      {#if channels.length}<span class="muted small">{channels.length}</span>{/if}
    </div>
    {#if rail.length}
      <ul>
        {#each rail as c (c.id)}
          {@const program = programAt(guideOf(c.id), now)}
          <li>
            <a class="row" href={watchHref(c.id)}>
              <span class="row-logo">
                <span class="row-letter" aria-hidden="true">{c.name.slice(0, 1)}</span>
                {#if c.logo}<img src={c.logo} alt="" loading="lazy" decoding="async" onerror={(event) => event.currentTarget.remove()} />{/if}
              </span>
              <span class="row-copy">
                <span class="row-name">{c.name}{#if c.hd}<span class="badge">HD</span>{/if}</span>
                <span class="row-meta">
                  {program?.showname ?? (guideFailed(c.id) ? "Guide unavailable" : c.category || c.language || "Live")}
                </span>
              </span>
              <span class="row-play" aria-hidden="true"><Icon name="play_arrow" size={16} /></span>
            </a>
          </li>
        {/each}
      </ul>
      <a class="link rail-more" href="#/channels">All channels</a>
    {:else if !loading}
      <p class="muted small">No other channels available.</p>
    {/if}
    <p class="rail-note muted small">M3U and XMLTV links for VLC, Kodi and TiviMate are in Settings.</p>
  </aside>
</section>

<style>
  .home { display: grid; gap: 20px; grid-template-columns: minmax(0, 1fr) 272px; align-items: start; }
  .main { display: grid; gap: 22px; min-width: 0; }

  .hero {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    min-height: 340px;
    gap: 16px;
    padding: 18px;
    border-radius: 14px;
    border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
    background:
      radial-gradient(120% 90% at 15% 0%, #1f2a44 0%, transparent 60%),
      radial-gradient(120% 120% at 85% 100%, #2a1a20 0%, transparent 55%),
      #121a2c;
    color: var(--text);
    text-decoration: none;
    overflow: hidden;
    isolation: isolate;
  }
  .hero:hover .hero-play,
  .hero:focus-visible .hero-play { background: color-mix(in srgb, var(--accent) 92%, transparent); border-color: transparent; }
  .hero-top { display: flex; align-items: center; gap: 8px; }
  .live-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.06em;
  }
  .live-dot { width: 5px; height: 5px; border-radius: 50%; background: #fff; }
  .tag {
    padding: 2px 8px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.14);
    font-size: 10px;
    font-weight: 700;
  }
  .tag.warn { background: color-mix(in srgb, #f4b740 22%, transparent); color: #f4b740; }
  .hero-play {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: grid;
    place-items: center;
    width: 66px;
    height: 66px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.35);
    background: rgba(255, 255, 255, 0.16);
    color: #fff;
    backdrop-filter: blur(6px);
    transition: background-color 0.15s ease;
  }
  .hero-play :global(svg) { margin-left: 3px; }
  /* The programme poster fills the card; the scrim keeps the copy readable. */
  .hero.has-art {
    background-image:
      linear-gradient(90deg, rgba(7, 10, 15, 0.9) 0%, rgba(7, 10, 15, 0.5) 48%, rgba(7, 10, 15, 0.08) 100%),
      linear-gradient(0deg, rgba(7, 10, 15, 0.97) 0%, rgba(7, 10, 15, 0.72) 26%, rgba(7, 10, 15, 0) 62%),
      var(--hero-art);
    background-position: center;
    background-size: cover;
    background-repeat: no-repeat;
  }
  .hero-meta { display: grid; gap: 6px; }
  .hero-name { margin: 0; font-size: 26px; font-weight: 800; letter-spacing: -0.02em; }
  .hero.has-art .hero-name,
  .hero.has-art .hero-program,
  .hero.has-art .hero-times { text-shadow: 0 1px 12px rgba(0, 0, 0, 0.55); }
  .hero-program { margin: 0; font-size: 13px; color: #d7dce7; }
  .hero-times { display: flex; justify-content: space-between; font-size: 11px; color: var(--muted); }
  .hero-cta {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-top: 4px;
    font-size: 13px;
    font-weight: 700;
    color: var(--accent);
  }

  .bar { display: block; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.2); overflow: hidden; }
  .fill { display: block; height: 100%; border-radius: 999px; background: var(--accent); }

  .section-head { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 10px; }
  .section-head h2 { margin: 0; font-size: 13px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  .link { color: var(--accent); font-size: 12px; font-weight: 700; text-decoration: none; }
  .link:hover { text-decoration: underline; }

  .poster-row { list-style: none; margin: 0; padding: 0; display: grid; gap: 12px; grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .poster-row li { min-width: 0; }
  /* Browsing rows are denser than the on-now strip: six across on desktop. */
  .category-row { grid-template-columns: repeat(6, minmax(0, 1fr)); }

  .rail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .rail-head { display: flex; align-items: baseline; justify-content: space-between; gap: 10px; }
  .rail-head h2 { margin: 0; font-size: 13px; font-weight: 700; }
  .rail ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-radius: 10px;
    color: var(--text);
    text-decoration: none;
    border: 1px solid transparent;
  }
  .row:hover, .row:focus-visible { background: var(--surface-2); border-color: var(--border); }
  .row-logo {
    position: relative;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 40px;
    height: 40px;
    border-radius: 8px;
    background: #1d2230;
    overflow: hidden;
  }
  .row-letter { font-size: 15px; font-weight: 800; color: color-mix(in srgb, var(--text) 35%, transparent); }
  .row-logo img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; padding: 4px; }
  .row-copy { display: grid; gap: 1px; min-width: 0; flex: 1 1 auto; }
  .row-name { display: flex; align-items: center; gap: 5px; font-size: 13px; font-weight: 650; }
  .row-meta { font-size: 11px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-play { display: grid; place-items: center; color: var(--muted); }
  .row:hover .row-play { color: var(--accent); }
  .rail-more { margin-top: 2px; }
  .rail-note { margin: 4px 0 0; }

  .panel {
    display: grid;
    gap: 10px;
    justify-items: start;
    padding: 22px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .panel h2 { margin: 0; font-size: 18px; }
  .panel p { margin: 0; max-width: 60ch; }

  .skeleton {
    background: linear-gradient(100deg, var(--surface) 20%, var(--surface-2) 40%, var(--surface) 60%) var(--surface);
    background-size: 220% 100%;
    animation: home-shimmer 1.5s linear infinite;
    border: 1px solid var(--border);
  }
  @keyframes home-shimmer {
    from { background-position: 120% 0; }
    to { background-position: -60% 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton { animation: none; }
  }

  @media (max-width: 1100px) {
    /* One column: the player stays first, the rail follows it. */
    .home { grid-template-columns: minmax(0, 1fr); }
  }
  @media (max-width: 1100px) {
    .category-row { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  }
  @media (max-width: 640px) {
    .hero { min-height: 300px; }
    .hero-play { top: 40%; }
    .hero-name { font-size: 21px; }
    .poster-row, .category-row { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  }
</style>
