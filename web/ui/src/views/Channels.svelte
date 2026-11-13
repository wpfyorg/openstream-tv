<script>
  import { onMount } from "svelte";
  import { api, invalidateChannels, loadChannels, looksLikeUnlockCode } from "../lib/api.js";
  import { latestOnly } from "../lib/latest.js";

  const saved = (() => {
    try {
      return JSON.parse(localStorage.getItem("channelFilters") || "{}");
    } catch {
      return {};
    }
  })();

  let channels = $state([]);
  // When every listed channel comes from one alternate provider the pill tells
  // the viewer nothing.
  const mixedSources = $derived(
    channels.some((c) => c.extras || c.fastway || c.playbox || c.vzy) &&
      channels.some((c) => !c.extras && !c.fastway && !c.playbox && !c.vzy),
  );
  let error = $state("");
  let loading = $state(true);
  let query = $state("");
  let category = $state(saved.category ?? "");
  let language = $state(saved.language ?? "");
  let hdOnly = $state(saved.hdOnly ?? false);
  let showUnplayable = $state(saved.showUnplayable ?? false);

  $effect(() => {
    try {
      localStorage.setItem(
        "channelFilters",
        JSON.stringify({ category, language, hdOnly, showUnplayable }),
      );
    } catch {}
  });

  // If (and only if) the search box holds something shaped like the extras
  // unlock code, try it against the server. An ordinary search never
  // matches this shape, so it never leaves the browser.
  // The initial load and the reload after an unlock can overlap; only the one
  // started last may publish, so a late pre-unlock result cannot overwrite the
  // extras catalogue (or restore an error the reload cleared).
  const loads = latestOnly();
  let triedCode = "";
  // Set once a code is accepted. A wrong code stays silent on purpose: it must
  // not reveal that this box does anything other than search.
  let unlockNotice = $state(null);
  $effect(() => {
    const q = query.trim();
    // A rejected or failed attempt is remembered only while the box still holds
    // that value, so it is not resent on every edit but can be tried again once
    // the viewer has cleared or changed it.
    if (triedCode && q !== triedCode) triedCode = "";
    if (q && q !== triedCode && looksLikeUnlockCode(q)) {
      triedCode = q;
      api("/api/extras/unlock", { method: "POST", body: { code: q } })
        .then(async (d) => {
          // The code is not a search term; clear it so the list is not empty,
          // unless the viewer has meanwhile typed something else.
          if (query.trim() === q) query = "";
          triedCode = "";
          unlockNotice = {
            connected: !!d?.extras?.connected,
            active: d?.extras?.activeProduct === "extras",
          };
          window.dispatchEvent(new CustomEvent("openstream:extras-changed"));
          const isLatest = loads.start();
          try {
            const list = await loadChannels(true);
            if (isLatest()) {
              channels = list;
              // A failed first load must not keep hiding a list that now loaded,
              // and this refresh may finish before the older request does.
              error = "";
              loading = false;
            }
          } catch (err) {
            // The code was accepted but the list could not be refreshed: keep what
            // is shown and say so, instead of claiming extra channels are listed.
            if (isLatest()) {
              if (unlockNotice) unlockNotice = { ...unlockNotice, refreshFailed: true };
              // The older initial request can no longer publish, so end the loading
              // state here; with nothing to show, report the failure instead.
              if (!channels.length && !error) error = err.message;
              loading = false;
            }
          }
        })
        .catch(() => {});
    }
  });

  const categories = $derived([...new Set(channels.map((c) => c.category).filter(Boolean))].sort());
  const languages = $derived([...new Set(channels.map((c) => c.language).filter(Boolean))].sort());
  const playableCount = $derived(channels.filter((c) => c.playable).length);
  const nonePlayable = $derived(channels.length > 0 && playableCount === 0);

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return channels.filter(
      (c) =>
        (showUnplayable || nonePlayable || c.playable) &&
        (!hdOnly || c.hd) &&
        (!category || c.category === category) &&
        (!language || c.language === language) &&
        (!q || c.name.toLowerCase().includes(q)),
    );
  });

  onMount(async () => {
    const isLatest = loads.start();
    try {
      const list = await loadChannels();
      if (isLatest()) channels = list;
    } catch (err) {
      if (isLatest()) error = err.message;
    } finally {
      // An unlock refresh that started meanwhile owns the loading state.
      if (isLatest()) loading = false;
    }
  });

  // The server exposes one provider at a time; when another tab, the Android
  // app or Settings switches it, replace the cached list instead of showing
  // the previous provider's channels.
  onMount(() => {
    const reload = () => {
      // App drops the cache before notifying, but a mount load may still be in
      // flight from before the switch; drop it too so only one fetch runs and
      // it cannot write the previous provider's channels.
      invalidateChannels();
      const isLatest = loads.start();
      loadChannels()
        .then((list) => {
          if (!isLatest()) return;
          channels = list;
          error = "";
          loading = false;
        })
        .catch((err) => {
          if (!isLatest()) return;
          error = err.message;
          loading = false;
        });
    };
    window.addEventListener("openstream:provider-changed", reload);
    return () => window.removeEventListener("openstream:provider-changed", reload);
  });
</script>

{#if unlockNotice}
  <div class="unlock-notice" role="status">
    <span class="unlock-icon" aria-hidden="true">✓</span>
    <div class="unlock-copy">
      <strong>JioTV+ unlocked</strong>
      <span>
        {#if unlockNotice.refreshFailed}
          The channel list could not be refreshed. Reload the page to see the active provider's channels.
        {:else if unlockNotice.connected && unlockNotice.active}
          JioTV+ is connected and active; its channels are listed below.
        {:else if unlockNotice.connected}
          JioTV+ is connected. Make it the active provider in Settings to see its channels.
        {:else}
          Sign in with a number that has JioTV+ access in Settings to play its channels.
        {/if}
      </span>
    </div>
    {#if !unlockNotice.connected}<a class="unlock-link" href="#/settings">Open Settings</a>{/if}
    <button class="unlock-dismiss" aria-label="Dismiss" onclick={() => (unlockNotice = null)}>×</button>
  </div>
{/if}

<section class="filters" aria-label="Filters">
  <input class="input search" type="search" placeholder="Search channels" bind:value={query} aria-label="Search channels" />
  <select class="input" bind:value={category} aria-label="Category">
    <option value="">All categories</option>
    {#each categories as c}<option>{c}</option>{/each}
  </select>
  <select class="input" bind:value={language} aria-label="Language">
    <option value="">All languages</option>
    {#each languages as l}<option>{l}</option>{/each}
  </select>
  <label class="toggle"><input type="checkbox" bind:checked={hdOnly} /> HD</label>
  <label class="toggle"><input type="checkbox" bind:checked={showUnplayable} /> Show unavailable</label>
</section>

{#if loading}
  <p class="muted">Loading channels…</p>
{:else if error}
  <p class="error">Couldn't load channels: {error}</p>
{:else}
  {#if nonePlayable}
    <p class="availability-note">
      Nothing in this catalogue is currently available to play. Showing the channels so the catalogue stays visible.
      Check <a href="#/settings">Settings</a> to connect a provider.
    </p>
  {/if}
  <p class="muted count">{visible.length} of {channels.length} channels</p>
  <ul class="grid">
    {#each visible as c (c.id)}
      <li>
        <a class="tile" class:off={!c.playable} href={"#/watch/" + encodeURIComponent(c.id)} title={c.playable ? c.name : c.name + " (unavailable for the current account)"}>
          <span class="logo">
            <img src={c.logo} alt="" loading="lazy" decoding="async" />
            {#if c.hd}<span class="hd-pill">HD</span>{/if}
            {#if mixedSources && (c.extras || c.fastway || c.playbox || c.vzy)}
              <span class="flags">
                {#if mixedSources && c.extras}<span class="flag flag-extras">TV+</span>{/if}
                {#if mixedSources && c.fastway}<span class="flag flag-fastway">Fastway</span>{/if}
                {#if mixedSources && c.playbox}<span class="flag flag-playbox">Playbox</span>{/if}
                {#if mixedSources && c.vzy}<span class="flag flag-vzy">VZY</span>{/if}
              </span>
            {/if}
            {#if c.requiresSubscription}
              <span class="ribbon">Subscription may be required</span>
            {/if}
          </span>
          <span class="name">{c.name}</span>
        </a>
      </li>
    {/each}
  </ul>
  {#if visible.length === 0}<p class="muted">No channels match these filters.</p>{/if}
{/if}

<style>
  .unlock-notice {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 0 14px;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--extras) 45%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--extras) 12%, var(--surface));
  }
  .unlock-icon {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    color: #0b1020;
    background: var(--extras);
    font-size: 14px;
    font-weight: 800;
  }
  .unlock-copy { display: grid; gap: 2px; min-width: 0; flex: 1 1 auto; font-size: 13px; }
  .unlock-copy span { color: var(--muted); }
  .unlock-link { color: var(--extras); font-size: 13px; font-weight: 650; white-space: nowrap; }
  .unlock-dismiss {
    flex: 0 0 auto;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 8px;
    color: var(--muted);
    background: transparent;
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
  }
  .unlock-dismiss:hover { color: var(--text); background: color-mix(in srgb, var(--text) 8%, transparent); }

  .filters { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 12px; }
  .filters .input { width: auto; }
  .search { flex: 1 1 240px; }
  .toggle { display: inline-flex; gap: 6px; align-items: center; min-height: 40px; color: var(--muted); padding: 0 6px; }
  .count { margin: 4px 0 12px; font-size: 13px; }
  .availability-note {
    margin: 0 0 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
    font-size: 13px;
  }
  .availability-note a { color: var(--text); font-weight: 600; }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
    padding: 12px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    text-decoration: none;
    transition: transform 0.12s ease, border-color 0.12s ease;
  }
  .tile:hover, .tile:focus-visible { transform: translateY(-2px); border-color: var(--accent); }
  .tile.off { opacity: 0.45; }
  /* Many logos are white on transparent, so keep the tile dark in both themes. */
  .logo {
    position: relative;
    flex: 0 0 auto;
    aspect-ratio: 16 / 10;
    display: grid;
    place-items: center;
    background: #1d2230;
    border-radius: 8px;
    overflow: hidden;
  }
  .logo img { max-width: 80%; max-height: 72%; object-fit: contain; }
  /* The ribbon sits on the plate's bottom edge, so artwork moves up to clear it. */
  .logo:has(.ribbon) img { max-height: 64%; margin-bottom: 14px; }
  .name {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.3;
    min-height: 2.6em;
    overflow: hidden;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  /* The HD mark floats on the artwork instead of taking a row under the name. */
  .hd-pill {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 1px 7px;
    border: 1px solid rgba(255, 255, 255, 0.22);
    border-radius: 999px;
    color: #fff;
    background: rgba(8, 10, 14, 0.72);
    backdrop-filter: blur(6px);
    box-shadow: 0 1px 8px rgba(0, 0, 0, 0.35);
    font-size: 9.5px;
    font-weight: 750;
    line-height: 1.5;
    letter-spacing: 0.04em;
  }
  /* Option 3: an amber ribbon across the bottom of the artwork plate, so the
     requirement is readable without a chip floating over the channel logo. */
  .ribbon {
    position: absolute;
    inset: auto 0 0 0;
    padding: 4px 8px 5px;
    border-top: 1px solid color-mix(in srgb, #f0a320 35%, transparent);
    background: color-mix(in srgb, #f0a320 16%, #0b1018);
    color: #f5b740;
    font-size: 9.5px;
    font-weight: 750;
    letter-spacing: 0.01em;
    line-height: 1.3;
    text-align: center;
    white-space: nowrap;
  }
  .flags { position: absolute; top: 8px; right: 8px; display: flex; flex-direction: column; align-items: flex-end; gap: 3px; max-width: calc(100% - 16px); }
  .flag {
    padding: 1px 7px;
    border-radius: 999px;
    box-shadow: 0 1px 8px rgba(0, 0, 0, 0.35);
    font-size: 9.5px;
    font-weight: 750;
    line-height: 1.5;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }
  .flag-extras { background: #7ea3ff; color: #0b1020; }
  .flag-fastway { background: var(--ok); color: #062015; }
  .flag-playbox { background: #c084fc; color: #1c0b2e; }
  .flag-vzy { background: #f472b6; color: #2b0a1a; }

  @media (max-width: 640px) {
    /* Two across on a phone: enough tiles to browse, still readable names. */
    .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
    .tile { padding: 10px; gap: 7px; }
    .ribbon { font-size: 9px; }
    .name { min-height: 0; }
    .flag, .hd-pill { font-size: 9px; }
    .hd-pill { top: 6px; left: 6px; padding: 1px 6px; }
  }
</style>
