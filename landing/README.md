# OpenStream TV landing page

The marketing page for [OpenStream TV](https://github.com/wpfyorg/openstream-tv):
a light, white-and-orange product page with a framed web-player mock in the
hero, a provider-support grid and the real install commands. It is a Vite +
Svelte site on a small Hono Worker; there is no backend and no app behind it.

It uses the same Svelte 5 and Vite setup as the embedded web UI, so the
repository keeps one frontend framework.

## Run it

```sh
bun install
bun run dev        # http://localhost:5173 (Vite + the Worker in workerd)
bun run build      # dist/client (assets) + dist/openstream_tv_landing/index.js (Worker)
bun test tests
```

Routes: `/` is the landing page; anything else shows the 404.

## Edit it

- `src/content.js`: every landing-page string — hero, stats, features, steps,
  providers, install commands, FAQ, closing CTA and footer. Links to the repo
  and docs are defined once at the top of that file.
- `src/sections/*` + `src/pages/Landing.svelte`: the page is a stack of
  sections. `HeroMock.svelte` is a still of the web player rendered at a fixed
  1018×554, scaled to fit and tilted in on scroll; it is inert and has no state.
- `src/components/*`: the small design system (Badge, Logo, Reveal,
  SectionHeading, WindowFrame). Shared class builders live in `src/lib/ui.js`,
  the scroll-reveal action in `src/lib/reveal.js`, and tokens in
  `src/styles.css`.
- `src/worker/index.js`: static assets plus `/api/health`; other `/api/*` paths
  return JSON 404.

### Tokens (`src/styles.css`)

`brand` `#d63f0f` for text and buttons, `brand-bright` `#f24e1e` for the
wordmark and display type, `brand-soft` for the hero highlight; neutrals
`ink` / `fg` / `body` / `muted` / `line` / `canvas`; `navy` for the dark bands.
Plus Jakarta Sans, small radii, and a reveal-on-scroll transition (`.reveal`,
driven by an IntersectionObserver action) that honours
`prefers-reduced-motion`.

## Deploy

GitHub Pages serves this page at https://wpfyorg.github.io/openstream-tv/ and the
help centre under `/docs`. The `Documentation` workflow builds it with
`LANDING_BASE=/openstream-tv/` (so `/assets/...` resolves under the project
subpath) and copies `dist/client` over the docs build. `homeUrl` in
`src/lib/ui.js` wraps Vite's base for the page's own links.

Freebuff Sites is still an option: `deploy.json` points at the Worker build, so
the Worker serves `/api/health` — an endpoint GitHub Pages cannot provide.

After `bun run build`, deploy this directory with the `sites_deploy` tool. The
Worker serves `/api/health` and returns 404 JSON for other `/api/*` paths; every
other path falls through to the single-page app.
