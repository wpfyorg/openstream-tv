<script>
  import { onMount } from "svelte";
  import { api, invalidateChannels } from "./lib/api.js";
  import { route } from "./lib/router.svelte.js";
  import Login from "./views/Login.svelte";
  import Channels from "./views/Channels.svelte";
  import Watch from "./views/Watch.svelte";
  import Settings from "./views/Settings.svelte";
  import OnDemand from "./views/OnDemand.svelte";
  import Show from "./views/Show.svelte";
  import VodPlayer from "./views/VodPlayer.svelte";

  let auth = $state({ loading: true, passwordSet: false, authenticated: false });
  let activeProduct = $state("");
  let providerStatusPromise = null;

  const providerLabels = {
    tv: "JioTV",
    extras: "JioTV+",
    fastway: "Fastway",
    playbox: "PlayboxTV",
    vzy: "VZY",
    airtel: "Airtel Xstream",
    tataplay: "Tata Play",
  };
  const activeProviderLabel = $derived(providerLabels[activeProduct] || "");
  const alternateActive = $derived(!!activeProduct && activeProduct !== "tv");

  async function refreshProviderStatus() {
    if (!auth.authenticated) {
      activeProduct = "";
      return;
    }
    if (providerStatusPromise) return providerStatusPromise;
    providerStatusPromise = (async () => {
      const status = await api("/api/status").catch(() => null);
      // A failed status request must not clear or change the current provider.
      if (!status) return;
      const next = status?.catalogue?.activeProduct || "";
      const changed = !!activeProduct && activeProduct !== next;
      activeProduct = next;
      if (changed) {
        // The Channels view may be unmounted, so its listener cannot clear the
        // shared cache; do it here before notifying it.
        invalidateChannels();
        window.dispatchEvent(new CustomEvent("openstream:provider-changed"));
      }
    })();
    try {
      await providerStatusPromise;
    } finally {
      providerStatusPromise = null;
    }
  }

  async function refreshAuth() {
    try {
      const s = await api("/api/auth/state");
      auth = { loading: false, ...s };
      await refreshProviderStatus();
    } catch {
      auth = { loading: false, passwordSet: true, authenticated: false };
      activeProduct = "";
    }
  }

  async function signOut() {
    await api("/api/auth/logout", { method: "POST" }).catch(() => {});
    auth = { ...auth, authenticated: false };
    activeProduct = "";
  }

  onMount(() => {
    let disposed = false;
    let statusTimer = null;
    const scheduleStatusRefresh = () => {
      if (disposed) return;
      statusTimer = window.setTimeout(async () => {
        await refreshProviderStatus();
        scheduleStatusRefresh();
      }, 15000);
    };
    refreshAuth().finally(scheduleStatusRefresh);
    window.addEventListener("openstream:extras-changed", refreshProviderStatus);
    const onSignedOut = () => {
      auth = { ...auth, authenticated: false };
      activeProduct = "";
    };
    window.addEventListener("openstream:signed-out", onSignedOut);
    return () => {
      disposed = true;
      window.clearTimeout(statusTimer);
      window.removeEventListener("openstream:extras-changed", refreshProviderStatus);
      window.removeEventListener("openstream:signed-out", onSignedOut);
    };
  });
</script>

{#if auth.loading}
  <p class="muted center">Loading…</p>
{:else if !auth.authenticated}
  <Login passwordSet={auth.passwordSet} onsignedin={refreshAuth} />
{:else}
  <header class="bar">
    <div class="brand-group">
      <a class="brand" href="#/">OpenStream TV</a>
      {#if alternateActive}
        <span
          class="provider-active"
          role="img"
          aria-label={`${activeProviderLabel} active`}
          title={`${activeProviderLabel} active`}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
        </span>
      {/if}
    </div>
    <nav>
      <a href="#/" aria-current={route.name === "channels" ? "page" : undefined}>Channels</a>
      <a href="#/ott" aria-current={["ott", "show", "play"].includes(route.name) ? "page" : undefined}>On demand</a>
      <a href="#/settings" aria-current={route.name === "settings" ? "page" : undefined}>Settings</a>
      <button class="link" onclick={signOut}>Sign out</button>
    </nav>
  </header>
  <main>
    {#if route.name === "watch"}
      <Watch id={route.param} />
    {:else if route.name === "ott"}
      <OnDemand />
    {:else if route.name === "show"}
      <Show id={route.param} />
    {:else if route.name === "play"}
      <VodPlayer id={route.param} />
    {:else if route.name === "settings"}
      <Settings />
    {:else}
      <Channels />
    {/if}
  </main>
{/if}

<style>
  .center { text-align: center; margin-top: 30vh; }
  .bar {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-width: 0;
    padding: 11px max(16px, calc((100vw - 1500px) / 2 + 16px));
    background: color-mix(in srgb, var(--bg) 88%, transparent);
    backdrop-filter: blur(14px);
    border-bottom: 1px solid var(--border);
  }
  .brand-group { display: flex; flex: 0 0 auto; align-items: center; gap: 6px; }
  .brand { font-weight: 800; font-size: 17px; letter-spacing: -.025em; text-decoration: none; }
  .provider-active {
    display: grid;
    width: 16px;
    height: 16px;
    place-items: center;
    border-radius: 50%;
    color: var(--ok);
    background: color-mix(in srgb, currentColor 15%, transparent);
  }
  .provider-active svg { width: 10px; height: 10px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  nav {
    display: flex;
    min-width: 0;
    align-items: center;
    /* flex-end would clip the first links when the row overflows on narrow phones. */
    justify-content: flex-start;
    gap: 3px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  nav::-webkit-scrollbar { display: none; }
  nav a, .link {
    flex: 0 0 auto;
    padding: 6px 10px;
    border-radius: 8px;
    text-decoration: none;
    color: var(--muted);
    background: none;
    border: 0;
    cursor: pointer;
    font-size: 13px;
    font-weight: 550;
  }
  nav a[aria-current="page"] { color: var(--text); background: var(--surface-2); }
  nav a:hover, .link:hover { color: var(--text); }
  main { width: 100%; max-width: 1500px; margin: 0 auto; padding: 24px 18px 56px; }

  @media (max-width: 640px) {
    .bar { gap: 10px; padding: 9px 12px; }
    .brand-group { gap: 4px; }
    .brand { font-size: 15px; }
    .provider-active { width: 14px; height: 14px; }
    .provider-active svg { width: 9px; height: 9px; }
    nav a, .link { display: inline-flex; align-items: center; min-height: 40px; padding: 0 8px; font-size: 12px; }
    main { padding: 14px 10px 40px; }
  }
  @media (max-width: 380px) {
    .bar { gap: 6px; }
    nav { gap: 0; }
    nav a, .link { padding: 0 6px; }
  }
</style>
