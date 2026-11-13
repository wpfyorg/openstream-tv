<script>
  import { onMount } from "svelte";
  import { api, invalidateChannels } from "./lib/api.js";
  import { route } from "./lib/router.svelte.js";
  import Icon from "./lib/Icon.svelte";
  import SearchOverlay from "./lib/SearchOverlay.svelte";
  import Login from "./views/Login.svelte";
  import Home from "./views/Home.svelte";
  import Channels from "./views/Channels.svelte";
  import Guide from "./views/Guide.svelte";
  import Watch from "./views/Watch.svelte";
  import Settings from "./views/Settings.svelte";
  import OnDemand from "./views/OnDemand.svelte";
  import Show from "./views/Show.svelte";
  import VodPlayer from "./views/VodPlayer.svelte";

  // The UI is served under /ui/, so brand assets are resolved from the same base.
  const brandBase = `${import.meta.env.BASE_URL}brand/`;

  // On a phone the header carries a search button that opens a results sheet,
  // so search never costs a page jump.
  let searchOpen = $state(false);

  let auth = $state({ loading: true, passwordSet: false, authenticated: false });
  // The sidebar collapses to an icon rail; the choice survives a reload.
  let collapsed = $state(false);
  const SIDEBAR_KEY = "sidebarCollapsed";

  function toggleSidebar() {
    collapsed = !collapsed;
    try {
      localStorage.setItem(SIDEBAR_KEY, String(collapsed));
    } catch {}
  }
  let activeProduct = $state("");
  let providerConnected = $state(false);
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

  const navItems = [
    { name: "home", href: "#/", label: "Live TV", icon: "live_tv" },
    { name: "channels", href: "#/channels", label: "Channels", icon: "grid_view" },
    { name: "guide", href: "#/guide", label: "Guide", icon: "calendar_month" },
    { name: "ott", href: "#/ott", label: "On demand", icon: "movie" },
    { name: "settings", href: "#/settings", label: "Settings", icon: "settings" },
  ];
  const titles = {
    home: "Live TV",
    channels: "Channels",
    guide: "Guide",
    ott: "On demand",
    settings: "Settings",
    watch: "Live player",
    show: "Show",
    play: "Player",
  };
  const ottRoutes = ["ott", "show", "play"];
  const current = $derived(
    navItems.some((n) => n.name === route.name)
      ? route.name
      : ottRoutes.includes(route.name)
        ? "ott"
        : "",
  );

  function connectedFor(status, product) {
    if (!status) return false;
    if (product === "extras") return !!status.extras?.connected;
    if (!product || product === "tv") return !!status.jiotv?.loggedIn;
    return !!status[product]?.connected;
  }

  async function refreshProviderStatus() {
    if (!auth.authenticated) {
      activeProduct = "";
      providerConnected = false;
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
      providerConnected = connectedFor(status, next);
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
      providerConnected = false;
    }
  }

  async function signOut() {
    await api("/api/auth/logout", { method: "POST" }).catch(() => {});
    auth = { ...auth, authenticated: false };
    activeProduct = "";
    providerConnected = false;
  }

  onMount(() => {
    try {
      collapsed = localStorage.getItem(SIDEBAR_KEY) === "true";
    } catch {}
  });

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
      providerConnected = false;
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
  <div class="shell" class:collapsed>
    <aside class="sidebar" class:collapsed>
      <div class="brand">
        <a class="wordmark" href="#/" aria-label="OpenStream TV">
          <img class="wordmark-lockup" src={`${brandBase}openstream-lockup-inverse.svg`} alt="OpenStream TV" width="127" height="19" />
          <img class="wordmark-mark" src={`${brandBase}openstream-mark-inverse.svg`} alt="OpenStream TV" width="26" height="26" />
          {#if alternateActive}
            <span class="providers-dot" role="img" aria-label={`${activeProviderLabel} active`} title={`${activeProviderLabel} active`}></span>
          {/if}
        </a>
        <button
          class="collapse"
          onclick={toggleSidebar}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          aria-expanded={!collapsed}
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          <Icon name={collapsed ? "chevron_right" : "chevron_left"} size={16} />
        </button>
      </div>
      <nav aria-label="App">
        {#each navItems as item (item.name)}
          <a
            class="nav"
            class:active={current === item.name}
            href={item.href}
            aria-label={collapsed ? item.label : undefined}
            title={collapsed ? item.label : undefined}
            aria-current={current === item.name ? "page" : undefined}
          >
            <Icon name={item.icon} size={19} />
            <span class="nav-label">{item.label}</span>
          </a>
        {/each}
      </nav>

      <div class="sidebar-foot">
        <a class="provider" href="#/settings" aria-label={`Provider: ${activeProviderLabel || "none"}`}>
          <span class="status-dot" class:on={providerConnected}></span>
          <span class="provider-copy">
            <strong>{activeProviderLabel || "No provider"}</strong>
            <span class="muted">{providerConnected ? "connected" : "not connected"}</span>
          </span>
        </a>
        <div class="account">
          <span class="avatar" aria-hidden="true">A</span>
          <span class="account-copy">
            <strong>Admin</strong>
            <span class="muted">Web session</span>
          </span>
          <button class="logout" aria-label="Sign out" title="Sign out" onclick={signOut}>
            <Icon name="logout" size={18} />
          </button>
        </div>
      </div>
    </aside>

    <div class="column">
      <header class="topbar">
        <h1>{titles[route.name] ?? "OpenStream TV"}</h1>
        {#if activeProviderLabel}
          <span class="provider-pill" class:on={providerConnected}>
            <span class="status-dot" class:on={providerConnected}></span>{activeProviderLabel}
          </span>
        {/if}
        <span class="topbar-spacer"></span>
        <button class="topbar-search" onclick={() => (searchOpen = true)} aria-label="Search channels" title="Search channels">
          <Icon name="search" size={20} />
        </button>
        <a class="topbar-link" href="#/settings">{providerConnected ? "Manage" : "Connect a provider"}</a>
      </header>

      <main>
        {#if route.name === "home"}
          <Home />
        {:else if route.name === "channels"}
          <Channels />
        {:else if route.name === "guide"}
          <Guide />
        {:else if route.name === "watch"}
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
          <div class="panel">
            <h2>Nothing here</h2>
            <p class="muted">That page does not exist. Try <a href="#/">Live TV</a>.</p>
          </div>
        {/if}
      </main>
    </div>

    <SearchOverlay open={searchOpen} onclose={() => (searchOpen = false)} />

    <nav class="tabbar" aria-label="App">
      {#each navItems as item (item.name)}
        <a
          class="tab"
          class:active={current === item.name}
          href={item.href}
          aria-current={current === item.name ? "page" : undefined}
        >
          <Icon name={item.icon} size={21} />
          <span>{item.label}</span>
        </a>
      {/each}
    </nav>
  </div>
{/if}

<style>
  .center { text-align: center; margin-top: 30vh; }

  .shell {
    display: grid;
    grid-template-columns: 236px minmax(0, 1fr);
    min-height: 100dvh;
    background: var(--bg);
  }

  .sidebar {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    gap: 18px;
    height: 100dvh;
    padding: 18px 14px;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }
  .brand { display: flex; align-items: center; gap: 6px; padding: 0 0 6px 6px; }
  .wordmark {
    display: flex;
    flex: 1 1 auto;
    align-items: center;
    gap: 6px;
    padding: 2px 4px;
    color: var(--accent);
    font-size: 14px;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-decoration: none;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .wordmark-lockup { display: block; height: 19px; width: auto; }
  .wordmark-mark { display: none; height: 26px; width: auto; }
  .collapse {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--muted);
    cursor: pointer;
  }
  .collapse:hover { color: var(--text); border-color: var(--muted); }
  .providers-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
  }
  nav { display: grid; gap: 3px; }
  .nav {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 10px;
    border-radius: 10px;
    color: var(--muted);
    font-size: 14px;
    font-weight: 550;
    text-decoration: none;
  }
  .nav:hover { color: var(--text); background: var(--surface-2); }
  .nav.active {
    color: var(--text);
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .nav.active :global(svg) { color: var(--accent); }

  /* Collapsed rail: icons only, every label kept as its tooltip and name. */
  .shell.collapsed { grid-template-columns: 76px minmax(0, 1fr); }
  .sidebar.collapsed { padding: 18px 10px; }
  .sidebar.collapsed .brand { flex-direction: column; gap: 10px; padding: 0; }
  .sidebar.collapsed .wordmark { flex: 0 0 auto; justify-content: center; padding: 0; }
  .sidebar.collapsed .wordmark-lockup { display: none; }
  .sidebar.collapsed .wordmark-mark { display: block; }
  .sidebar.collapsed .nav { justify-content: center; padding: 0; }
  .sidebar.collapsed .nav-label { display: none; }
  .sidebar.collapsed .provider { justify-content: center; padding: 10px 0; }
  .sidebar.collapsed .provider-copy,
  .sidebar.collapsed .account-copy,
  .sidebar.collapsed .logout { display: none; }
  .sidebar.collapsed .account { justify-content: center; padding: 6px 0; }

  .sidebar-foot { display: grid; gap: 10px; margin-top: auto; }
  .provider {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface-2);
    text-decoration: none;
  }
  .status-dot {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
  }
  .status-dot.on { background: var(--ok); }
  .provider-copy { display: grid; min-width: 0; font-size: 12px; }
  .provider-copy strong { font-size: 12px; font-weight: 700; }

  .account { display: flex; align-items: center; gap: 10px; padding: 6px 8px; }
  .avatar {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    font-weight: 700;
  }
  .account-copy { display: grid; min-width: 0; font-size: 12px; }
  .account-copy strong { font-weight: 650; }
  .logout {
    margin-left: auto;
    padding: 6px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .logout:hover { color: var(--text); background: var(--surface-2); }

  .column { display: flex; flex-direction: column; min-width: 0; }
  .topbar {
    position: sticky;
    top: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 72px;
    padding: 0 24px;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 90%, transparent);
    backdrop-filter: blur(12px);
  }
  .topbar h1 { margin: 0; font-size: 22px; font-weight: 750; letter-spacing: -0.02em; }
  .provider-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 700;
  }
  .provider-pill.on { color: var(--text); }
  .topbar-spacer { flex: 1 1 auto; }
  .topbar-search { display: none; }
  .topbar-link { color: var(--accent); font-size: 13px; font-weight: 650; text-decoration: none; }
  .topbar-link:hover { text-decoration: underline; }

  main {
    width: 100%;
    max-width: 1500px;
    margin: 0 auto;
    padding: 22px 24px 72px;
  }
  .panel {
    display: grid;
    gap: 8px;
    padding: 22px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
  }
  .panel h2 { margin: 0; font-size: 18px; }
  .panel p { margin: 0; }
  .panel a { color: var(--accent); font-weight: 650; }

  .tabbar { display: none; }

  @media (max-width: 900px) {
    .shell { grid-template-columns: minmax(0, 1fr); }
    .sidebar { display: none; }
    .topbar { height: 58px; padding: 0 14px; gap: 8px; }
    .topbar h1 { font-size: 19px; }
    .topbar-link { display: none; }
    /* The magnifier sits opposite the screen title, where a desktop header has text. */
    .topbar-search {
      display: grid;
      flex: 0 0 auto;
      place-items: center;
      width: 36px;
      height: 36px;
      padding: 0;
      border: 1px solid var(--border);
      border-radius: 10px;
      color: var(--text);
      background: var(--surface);
    }
    .topbar-search:active { background: var(--surface-2); }
    main { padding: 14px 12px calc(88px + env(safe-area-inset-bottom)); }
    .tabbar {
      position: fixed;
      inset: auto 0 0 0;
      z-index: 30;
      display: grid;
      grid-auto-flow: column;
      padding-bottom: env(safe-area-inset-bottom);
      border-top: 1px solid var(--border);
      background: color-mix(in srgb, var(--surface) 97%, transparent);
      backdrop-filter: blur(14px);
    }
    .tab {
      display: grid;
      justify-items: center;
      gap: 2px;
      padding: 9px 0 8px;
      color: var(--muted);
      font-size: 10px;
      font-weight: 650;
      text-decoration: none;
    }
    .tab.active { color: var(--accent); }
  }
</style>
