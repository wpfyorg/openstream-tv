<script>
  import { loadChannels } from "./api.js";
  import Icon from "./Icon.svelte";

  let { open = false, onclose = () => {} } = $props();

  let query = $state("");
  let channels = $state([]);
  let loading = $state(false);
  let failed = $state("");
  let input = $state(null);
  let sheet = $state(null);
  // One fetch per opening: the catalogue is cached by api.js for the rest of
  // the session, and this sheet never needs a second request.
  let started = false;
  // Bumped by every request and by a provider reset, so a request that outlived
  // the reset cannot write the previous provider's catalogue into the new state.
  let generation = 0;
  let restoreFocus = null;

  const results = $derived.by(() => {
    const list = channels.filter((c) => c.playable);
    const q = query.trim().toLowerCase();
    if (!q) return list.slice(0, 12);
    return list
      .filter(
        (c) =>
          c.name.toLowerCase().includes(q) ||
          (c.category ?? "").toLowerCase().includes(q) ||
          (c.language ?? "").toLowerCase().includes(q),
      )
      .slice(0, 30);
  });

  async function start() {
    if (started) return;
    started = true;
    const mine = ++generation;
    loading = true;
    try {
      const list = await loadChannels();
      if (mine !== generation) return;
      channels = list;
      failed = "";
    } catch (err) {
      if (mine !== generation) return;
      failed = err.message;
      started = false;
    } finally {
      if (mine === generation) loading = false;
    }
  }

  function close() {
    open = false;
    query = "";
    onclose();
  }

  // The sheet is modal, so Tab and Shift+Tab cycle through its own controls
  // instead of reaching the page behind the scrim.
  function focusables() {
    if (!sheet) return [];
    return [...sheet.querySelectorAll("a[href], button, input")].filter(
      (el) => !el.disabled && el.getClientRects().length > 0,
    );
  }

  function onKey(event) {
    if (event.key === "Escape") {
      close();
      return;
    }
    if (event.key !== "Tab") return;
    const items = focusables();
    if (!items.length) return;
    const first = items[0];
    const last = items[items.length - 1];
    if (!sheet.contains(document.activeElement)) {
      event.preventDefault();
      first.focus();
    } else if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  // A different provider means a different catalogue, so the next opening must
  // fetch again instead of serving the previous provider's channels.
  $effect(() => {
    const reset = () => {
      generation += 1;
      started = false;
      channels = [];
      failed = "";
      loading = false;
      // Reopening would fetch anyway; reload now when the sheet is still open.
      if (open) start();
    };
    window.addEventListener("openstream:provider-changed", reset);
    return () => window.removeEventListener("openstream:provider-changed", reset);
  });

  $effect(() => {
    if (!open) return;
    start();
    // The sheet is opened by a tap, so focusing its field is expected. The
    // element that had focus keeps it for when the sheet closes.
    restoreFocus =
      document.activeElement instanceof HTMLElement && document.activeElement !== document.body
        ? document.activeElement
        : null;
    requestAnimationFrame(() => input?.focus());
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      if (restoreFocus instanceof HTMLElement && restoreFocus.isConnected) {
        restoreFocus.focus();
      }
      restoreFocus = null;
    };
  });
</script>

{#if open}
  <div class="scrim" role="presentation" onclick={close}></div>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Search channels" bind:this={sheet}>
    <div class="row">
      <Icon name="search" size={18} />
      <input bind:this={input} bind:value={query} type="search" placeholder="Search channels" aria-label="Search channels" />
      <button class="cancel" onclick={close}>Cancel</button>
    </div>

    {#if failed}
      <p class="note error">Couldn't load channels: {failed}</p>
    {:else if loading}
      <p class="note">Loading channels…</p>
    {:else if results.length === 0}
      <p class="note">No channel matches “{query.trim()}”.</p>
    {:else}
      <ul>
        {#each results as c (c.id)}
          <li>
            <a href={"#/watch/" + encodeURIComponent(c.id)} onclick={close}>
              <span class="logo">
                {#if c.logo}<img src={c.logo} alt="" loading="lazy" decoding="async" />{/if}
              </span>
              <span class="copy">
                <span class="name">{c.name}{#if c.hd}<span class="hd">HD</span>{/if}</span>
                <span class="meta">{c.category || c.language || "Live TV"}</span>
              </span>
              <Icon name="play_arrow" size={16} />
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(4, 6, 10, 0.62);
    backdrop-filter: blur(2px);
  }
  .sheet {
    position: fixed;
    z-index: 61;
    inset: 0 0 auto 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 86vh;
    padding: 12px 12px 16px;
    border-bottom: 1px solid var(--border);
    border-radius: 0 0 18px 18px;
    background: var(--surface);
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.45);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface-2);
    color: var(--muted);
  }
  .row input {
    flex: 1 1 auto;
    min-width: 0;
    height: 42px;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 15px;
    outline: none;
  }
  .cancel {
    flex: 0 0 auto;
    padding: 0 4px;
    border: 0;
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }
  .note { margin: 4px 2px; color: var(--muted); font-size: 13px; }
  .note.error { color: var(--danger); }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  a {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-radius: 10px;
    color: var(--text);
    text-decoration: none;
  }
  a:hover, a:focus-visible { background: var(--surface-2); }
  .logo {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 44px;
    height: 34px;
    overflow: hidden;
    border-radius: 8px;
    background: #1d2230;
  }
  .logo img { max-width: 84%; max-height: 78%; object-fit: contain; }
  .copy { display: grid; gap: 1px; min-width: 0; flex: 1 1 auto; }
  .name { display: flex; align-items: center; gap: 5px; font-size: 14px; font-weight: 650; }
  .hd {
    padding: 0 5px;
    border: 1px solid rgba(255, 255, 255, 0.22);
    border-radius: 999px;
    color: var(--muted);
    font-size: 9px;
    font-weight: 750;
  }
  .meta { font-size: 11px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  a :global(svg) { color: var(--muted); flex: 0 0 auto; }
</style>
