<script>
  // The app's handful of icons, inline so the UI needs no icon font.
  let { name = "", size = 20 } = $props();
</script>

<svg
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if name === "live_tv"}
    <rect x="2.5" y="6" width="19" height="13" rx="3" />
    <path d="M8 2.6 12 6l4-3.4" />
  {:else if name === "grid_view"}
    <rect x="3" y="3" width="7.5" height="7.5" rx="2" />
    <rect x="13.5" y="3" width="7.5" height="7.5" rx="2" />
    <rect x="3" y="13.5" width="7.5" height="7.5" rx="2" />
    <rect x="13.5" y="13.5" width="7.5" height="7.5" rx="2" />
  {:else if name === "calendar_month"}
    <rect x="3" y="5" width="18" height="16" rx="3" />
    <path d="M3 10h18M8 3v4M16 3v4" />
  {:else if name === "movie"}
    <rect x="2.5" y="5" width="19" height="14" rx="3" />
    <path d="M7 5v3M12 5v3M17 5v3" />
  {:else if name === "settings"}
    <circle cx="12" cy="12" r="3.2" />
    <path d="M12 2.6v2.4M12 19v2.4M21.4 12H19M5 12H2.6M18.6 5.4 17 7M7 17l-1.6 1.6M18.6 18.6 17 17M7 7 5.4 5.4" />
  {:else if name === "logout"}
    <path d="M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 8l-4 4 4 4M6 12h9" />
  {:else if name === "play_arrow"}
    <path d="M8 5.5v13l11-6.5z" fill="currentColor" stroke="none" />
  {:else if name === "chevron_right"}
    <path d="m9 5 7 7-7 7" />
  {:else if name === "chevron_left"}
    <path d="m15 5-7 7 7 7" />
  {:else if name === "menu"}
    <path d="M4 7h16M4 12h16M4 17h16" />
  {:else if name === "search"}
    <circle cx="11" cy="11" r="6.5" />
    <path d="m16 16 4.5 4.5" />
  {/if}
</svg>
