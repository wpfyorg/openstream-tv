<script>
  // A channel/programme card whose artwork is the background, matching the hero:
  // programme poster when the guide has one, the channel logo when it does not.
  let { href = "", title = "", subtitle = "", time = "", progress = null, art = "", fit = "cover" } = $props();
</script>

<a
  class="poster-card"
  class:has-art={!!art}
  class:logo={fit === "logo"}
  href={href}
  style={art ? `--card-art: url("${art}")` : ""}
>
  <span class="card-body">
    {#if time}<span class="card-time">{time}</span>{/if}
    <span class="card-title">{title}</span>
    {#if subtitle}<span class="card-sub">{subtitle}</span>{/if}
    {#if progress !== null}
      <span class="card-bar" aria-hidden="true"><span style:width={`${progress}%`}></span></span>
    {/if}
  </span>
</a>

<style>
  .poster-card {
    position: relative;
    isolation: isolate;
    display: flex;
    align-items: flex-end;
    min-height: 142px;
    height: 100%;
    padding: 12px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    color: var(--text);
    text-decoration: none;
  }
  .poster-card:hover { border-color: color-mix(in srgb, var(--accent) 55%, var(--border)); }
  .poster-card.has-art::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    background-image:
      linear-gradient(0deg, rgba(7, 10, 15, 0.97) 0%, rgba(7, 10, 15, 0.82) 38%, rgba(7, 10, 15, 0.24) 100%),
      var(--card-art);
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
  }
  /* A channel logo is not artwork: keep its shape instead of stretching it. */
  .poster-card.logo::before {
    background-image:
      linear-gradient(0deg, rgba(7, 10, 15, 0.97) 0%, rgba(7, 10, 15, 0.88) 42%, rgba(7, 10, 15, 0.5) 100%),
      var(--card-art);
    background-size: cover, 46% auto;
    background-position: center, center 34%;
  }
  .card-body { display: grid; gap: 4px; min-width: 0; width: 100%; }
  .card-time { font-size: 11px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .card-title {
    display: -webkit-box;
    overflow: hidden;
    font-size: 15px;
    font-weight: 650;
    line-height: 1.25;
    overflow-wrap: anywhere;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-clamp: 2;
    text-shadow: 0 1px 10px rgba(0, 0, 0, 0.5);
  }
  .card-sub {
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-bar { display: block; height: 4px; margin-top: 2px; overflow: hidden; border-radius: 999px; background: color-mix(in srgb, var(--text) 16%, transparent); }
  .card-bar span { display: block; height: 100%; border-radius: 999px; background: var(--ok); }
</style>
