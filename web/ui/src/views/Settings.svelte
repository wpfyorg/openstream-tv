<script>
  import { onMount, onDestroy } from "svelte";
  import { api, loadChannels } from "../lib/api.js";
  import CopyField from "../lib/CopyField.svelte";
  import JioTVLogin from "../lib/JioTVLogin.svelte";
  import ExtrasLogin from "../lib/ExtrasLogin.svelte";
  import FastwayLogin from "../lib/FastwayLogin.svelte";
  import PlayboxLogin from "../lib/PlayboxLogin.svelte";
  import AirtelLogin from "../lib/AirtelLogin.svelte";
  import TataPlayLogin from "../lib/TataPlayLogin.svelte";
  import { plainHttpOrigin } from "../lib/origin.js";

  let status = $state(null);
  let error = $state("");
  let pw = $state({ current: "", next: "", message: "", ok: false });
  let providerBusy = $state("");

  const activeProvider = $derived.by(() => {
    const product = status?.catalogue?.activeProduct;
    if (product === "extras") return "jiotv_plus";
    if (!product || product === "tv") return "jiotv";
    return product;
  });

  async function refresh() {
    try {
      status = await api("/api/status");
      error = "";
    } catch (err) {
      error = err.message;
    }
  }

  function providerChanged() {
    window.dispatchEvent(new CustomEvent("openstream:provider-changed"));
    window.dispatchEvent(new CustomEvent("openstream:extras-changed"));
  }

  async function activate(provider) {
    if (providerBusy) return;
    providerBusy = provider;
    error = "";
    try {
      await api("/api/providers/activate", { method: "POST", body: { provider } });
      await loginChanged();
    } catch (err) {
      error = err.message;
    } finally {
      providerBusy = "";
    }
  }

  async function loginChanged() {
    await refresh();
    loadChannels(true);
    providerChanged();
  }

  async function logout(which) {
    const labels = {
      jiotv: "JioTV",
      extras: "JioTV+",
      fastway: "Fastway",
      playbox: "PlayboxTV",
      vzy: "VZY",
      airtel: "Airtel Xstream",
      tataplay: "Tata Play",
    };
    if (!confirm(`Log out of ${labels[which] ?? which}?`)) return;
    try {
      await api(`/api/${which}/logout`, { method: "POST" });
      await loginChanged();
    } catch (err) {
      error = err.message;
    }
  }

  // Typing the unlock code into the channel search box (Channels.svelte)
  // dispatches this once it succeeds, so this section can appear without a
  // reload.
  window.addEventListener("openstream:extras-changed", refresh);
  onDestroy(() => window.removeEventListener("openstream:extras-changed", refresh));

  async function rotateKey() {
    if (!confirm("Make a new access key? Every player using the current playlist URL will stop working until you give it the new one.")) return;
    try {
      status = await api("/api/key/rotate", { method: "POST" });
    } catch (err) {
      error = err.message;
    }
  }

  async function changePassword(event) {
    event.preventDefault();
    try {
      await api("/api/account/password", { method: "POST", body: { current: pw.current, new: pw.next } });
      pw = { current: "", next: "", message: "Password changed. Other browsers are signed out.", ok: true };
    } catch (err) {
      pw.message = err.message;
      pw.ok = false;
    }
  }

  // On phones the sidebar is hidden, so this is the only web-session sign-out.
  async function signOutBrowser() {
    try {
      await api("/api/auth/logout", { method: "POST" });
    } catch {}
    location.reload();
  }

  // IPTV clients often reject a self-signed certificate, so when this page was
  // opened on the server's own HTTPS port, offer the plain-HTTP origin instead.
  let iptvOrigin = $derived(plainHttpOrigin(location, status));

  onMount(refresh);
</script>

{#if error}<p class="error" role="alert">{error}</p>{/if}

{#if status}
  <section class="group" aria-labelledby="group-providers">
    <header class="group-head">
      <h2 id="group-providers">Provider login</h2>
      <p class="muted">Sign in to the provider whose channels you want. Only the active provider's catalogue is listed.</p>
    </header>
    <div class="cols">
    <section class="card">
      <div class="card-head">
        <h2>JioTV</h2>
        <span class="pill stable">Stable</span>
        {#if activeProvider === "jiotv"}<span class="pill active">Active</span>{/if}
      </div>
      {#if status.jiotv.loggedIn}
        <p class="ok">Logged in</p>
        {#if activeProvider !== "jiotv"}
          <button class="btn" disabled={providerBusy !== ""} onclick={() => activate("jiotv")}>Use JioTV</button>
        {/if}
        {#if !status.logoutDisabled}<button class="btn" onclick={() => logout("jiotv")}>Log out</button>{/if}
      {:else}
        <p class="muted">Log in with a JioTV mobile number.</p>
        <JioTVLogin ondone={loginChanged} />
      {/if}
    </section>

      <section class="card">
        <div class="card-head">
          <h2>JioTV+ <span class="pill stable">Stable</span></h2>
          {#if activeProvider === "jiotv_plus"}<span class="pill active">Active</span>{/if}
        </div>
        {#if status.extras.connected}
          <p class="ok">Connected</p>
          {#if activeProvider !== "jiotv_plus"}
            <button class="btn" disabled={providerBusy !== ""} onclick={() => activate("jiotv_plus")}>Use JioTV+</button>
          {/if}
          {#if !status.logoutDisabled}<button class="btn" onclick={() => logout("extras")}>Disconnect</button>{/if}
        {:else}
          <p class="muted">Use the JioFiber or AirFiber mobile number.</p>
          <ExtrasLogin ondone={loginChanged} />
        {/if}
      </section>

      <section class="card">
        <div class="card-head">
          <h2>Fastway <span class="pill stable">Stable</span></h2>
          {#if activeProvider === "fastway"}<span class="pill active">Active</span>{/if}
        </div>
        {#if status.fastway.connected}
          <p class="ok">Connected</p>
          {#if activeProvider !== "fastway"}
            <button class="btn" disabled={providerBusy !== ""} onclick={() => activate("fastway")}>Use Fastway</button>
          {/if}
          {#if !status.logoutDisabled}<button class="btn" onclick={() => logout("fastway")}>Disconnect</button>{/if}
        {:else}
          <p class="muted">Use the Fastway username and password for your subscription.</p>
          <FastwayLogin ondone={loginChanged} />
        {/if}
      </section>

      <section class="card">
        <div class="card-head">
          <h2>PlayboxTV <span class="pill beta">Beta</span></h2>
          {#if activeProvider === "playbox"}<span class="pill active">Active</span>{/if}
        </div>
        {#if status.playbox.connected}
          <p class="ok">Connected</p>
          {#if activeProvider !== "playbox"}
            <button class="btn" disabled={providerBusy !== ""} onclick={() => activate("playbox")}>Use PlayboxTV</button>
          {/if}
          {#if !status.logoutDisabled}<button class="btn" onclick={() => logout("playbox")}>Disconnect</button>{/if}
        {:else}
          <p class="muted">Use the PlayboxTV mobile number.</p>
          <PlayboxLogin ondone={loginChanged} />
        {/if}
      </section>

      <section class="card">
        <div class="card-head">
          <h2>VZY <span class="pill beta">Beta</span></h2>
          {#if activeProvider === "vzy"}<span class="pill active">Active</span>{/if}
        </div>
        <p class="muted">Free-tier channels only; no account is required.</p>
        {#if status.vzy.connected}
          <p class="ok">Free tier available</p>
          {#if activeProvider !== "vzy"}
            <button class="btn" disabled={providerBusy !== ""} onclick={() => activate("vzy")}>Use VZY</button>
          {/if}
        {:else}
          <p class="muted">The anonymous session is not ready. It is retried automatically.</p>
        {/if}
      </section>

      <section class="card">
        <div class="card-head">
          <h2>Airtel Xstream <span class="pill beta">Beta</span></h2>
          {#if activeProvider === "airtel"}<span class="pill active">Active</span>{/if}
        </div>
        {#if status.airtel.connected}
          <p class="ok">Connected</p>
          {#if activeProvider !== "airtel"}
            <button class="btn" disabled={providerBusy !== ""} onclick={() => activate("airtel")}>Use Airtel Xstream</button>
          {/if}
          {#if !status.logoutDisabled}<button class="btn" onclick={() => logout("airtel")}>Disconnect</button>{/if}
        {:else}
          <p class="muted">Use the Airtel mobile number for your Xstream subscription.</p>
          <AirtelLogin ondone={loginChanged} />
        {/if}
      </section>

      <section class="card">
        <div class="card-head">
          <h2>Tata Play <span class="pill beta">Beta</span></h2>
          {#if activeProvider === "tataplay"}<span class="pill active">Active</span>{/if}
        </div>
        {#if status.tataplay.connected}
          <p class="ok">Connected</p>
          {#if activeProvider !== "tataplay"}
            <button class="btn" disabled={providerBusy !== ""} onclick={() => activate("tataplay")}>Use Tata Play</button>
          {/if}
          {#if !status.logoutDisabled}<button class="btn" onclick={() => logout("tataplay")}>Disconnect</button>{/if}
        {:else}
          <p class="muted">Use the Tata Play mobile number for your subscription.</p>
          <TataPlayLogin ondone={loginChanged} />
        {/if}
      </section>

    </div>
  </section>

  <section class="group" aria-labelledby="group-playlist">
    <header class="group-head">
      <h2 id="group-playlist">IPTV playlist &amp; EPG</h2>
      <p class="muted">Add these to your TV's IPTV app. Anyone with the playlist URL can watch, so keep it private.</p>
    </header>
    <div class="cols">
      <section class="card">
        <h2>Playlist and guide links</h2>
        <CopyField label="Playlist" value={iptvOrigin + status.playlistPath} />
        <CopyField label="EPG" value={iptvOrigin + status.epgPath} />
        {#if !status.epg}<p class="muted small">The EPG file is off on this server (option <code>epg</code>).</p>{/if}
        <button class="btn danger" onclick={rotateKey}>Make a new key</button>
      </section>
    </div>
  </section>

  <section class="group" aria-labelledby="group-security">
    <header class="group-head">
      <h2 id="group-security">Security</h2>
      <p class="muted">The password you use to sign in to this web UI.</p>
    </header>
    <div class="cols">
      <section class="card">
        <h2>This browser</h2>
        <p class="muted">Ends the admin session on this device. Connected providers keep their saved sign-in.</p>
        <button class="btn" onclick={signOutBrowser}>Sign out</button>
      </section>
      <section class="card">
        <h2>Admin password</h2>
        <form onsubmit={changePassword}>
          <input class="input" type="password" placeholder="Current password" autocomplete="current-password" bind:value={pw.current} required />
          <input class="input" type="password" placeholder="New password (8+ characters)" autocomplete="new-password" minlength="8" bind:value={pw.next} required />
          <button class="btn">Change password</button>
          <p class:ok={pw.ok} class:error={!pw.ok} role="status">{pw.message}</p>
        </form>
      </section>
    </div>
  </section>
  <p class="muted small">
    OpenStream TV {status.version} ·
    <a class="discord" href="https://discord.gg/Xn3bUsBCZU" target="_blank" rel="noreferrer">Join the Discord</a>
  </p>
{:else if !error}
  <p class="muted">Loading…</p>
{/if}

<style>
  /* Stretch so cards sharing a row match the tallest one instead of leaving
     a hole beside a short card. */
  .cols { display: grid; gap: 16px; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); align-items: stretch; }
  section { display: flex; flex-direction: column; gap: 12px; align-items: flex-start; }
  section > :global(*) { max-width: 100%; }
  .group { align-items: stretch; gap: 14px; margin-bottom: 26px; }
  .group-head { display: grid; gap: 3px; padding-bottom: 10px; border-bottom: 1px solid var(--border); }
  .group-head h2 {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    font-weight: 750;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .group-head p { margin: 0; font-size: 13px; }
  .discord { color: var(--accent); font-weight: 650; }
  .card-head { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  h2 { margin: 0; font-size: 17px; }
  .pill {
    padding: 1px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-size: 10px;
    font-weight: 750;
    letter-spacing: 0.04em;
    line-height: 1.6;
    text-transform: uppercase;
  }
  .pill.active { color: var(--ok); border-color: color-mix(in srgb, var(--ok) 45%, transparent); }
  .pill.stable { color: var(--text); border-color: color-mix(in srgb, var(--text) 30%, transparent); }
  .pill.beta {
    color: var(--extras);
    border-color: color-mix(in srgb, var(--extras) 45%, transparent);
    background: color-mix(in srgb, var(--extras) 12%, transparent);
  }
  p { margin: 0; }
  form { display: flex; flex-direction: column; gap: 10px; width: 100%; align-items: flex-start; }
  .small { font-size: 13px; margin-top: 16px; }
  code { background: var(--surface-2); padding: 1px 5px; border-radius: 5px; }
</style>
