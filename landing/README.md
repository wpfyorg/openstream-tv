# OpenStream TV landing page

The marketing page for [OpenStream TV](https://github.com/wpfyorg/openstream-tv):
a light, white-and-orange product page with a framed web-player mock in the
hero, a provider-support grid and the real install commands. It is a Vite +
React site on a small Hono Worker; there is no backend and no app behind it.

Built from the Freebuff **Desk booking** starter: the layout, design tokens and
landing-page sections come from that template, and everything else — the copy,
the hero mock, the providers and install sections, the 404 — is OpenStream TV's.

## Run it

```sh
bun install
bun run dev        # http://localhost:5173 (Vite + the Worker in workerd)
bun run build      # dist/client (assets) + dist/openstream_tv_landing/index.js (Worker)
bun run typecheck
bun test tests
```

Routes: `/` is the landing page; anything else shows the 404.

## Edit it

- `src/content.ts`: every landing-page string — hero, stats, features, steps,
  providers, install commands, FAQ, closing CTA and footer. Links to the repo
  and docs are defined once at the top of that file.
- `src/sections/*` + `src/pages/Landing.tsx`: the page is a stack of sections.
  `HeroMock` is a still of the web player rendered at a fixed 1018×554 and
  scaled to fit; it is inert and has no state.
- `src/components/*`: the small design system (Button, Card, Badge, Reveal,
  SectionHeading, WindowFrame, Logo, Spinner). Tokens live in `src/styles.css`.
- `src/worker/index.ts`: static assets plus `/api/health`; other `/api/*` paths
  return JSON 404.

### Tokens (`src/styles.css`)

`brand` `#d63f0f` for text and buttons, `brand-bright` `#f24e1e` for the
wordmark and display type, `brand-soft` for the hero highlight; neutrals
`ink` / `fg` / `body` / `muted` / `line` / `canvas`; `navy` for the dark bands.
Plus Jakarta Sans, small radii, and a reveal-on-scroll motion set in
`src/lib/motion.ts` that honours `prefers-reduced-motion`.

## Deploy with Freebuff Sites

`deploy.json` points at the build output. After `bun run build`, deploy this
directory with the `sites_deploy` tool. The Worker serves `/api/health` and
returns 404 JSON for other `/api/*` paths; every other path falls through to
the single-page app.
