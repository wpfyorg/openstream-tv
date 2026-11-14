# OpenStream TV documentation site

The public help centre at https://wpfyorg.github.io/openstream-tv/, built with
[SvelteKit](https://svelte.dev/docs/kit), [MDSvex](https://mdsvex.pngwn.io/) and
Tailwind. It started from
[code-gio/svelte-docs-starter](https://github.com/code-gio/svelte-docs-starter)
(MIT) and follows the same Svelte 5 setup as the rest of the repository.

## Where the content lives

`docs/` in the repository root is still the single source of truth. Nothing in
`src/content/docs/` is edited by hand or committed:

```sh
bun run sync:content   # python3 ../scripts/build-docs-site.py --sync-only
```

That script copies every `docs/<slug>/index.md` into `src/content/docs/`,
rewrites the `../<slug>/index.md` links into routes, copies the screenshots to
`static/images/`, and regenerates `src/lib/docs/sidebar.generated.ts` from
`docs/SUMMARY.md`. The help-centre page becomes the docs home at `/docs`.

## Run it

```sh
bun install
bun run dev      # http://localhost:5173, syncs the content first
bun run build    # prerenders to build/ and indexes search with Pagefind
bun run check    # svelte-check
```

## Deploy

The `Documentation` workflow runs `scripts/check-doc-links.py`, then builds this
directory with `DOCS_BASE=/openstream-tv` and uploads `docs-site/build` to
GitHub Pages. Trailing-slash URLs let the articles keep their relative links at
any base path; `paths.base` handles navigation, assets and canonical URLs.

The site lives at `/openstream-tv/docs/`; the root of that Pages site belongs to
the landing page in `landing/`, which the workflow copies over this build. This
app therefore has no `/` route — local builds link straight to the help centre.

Search is Pagefind: the postbuild step indexes the prerender and writes the
index into `build/pagefind`, so it ships with the uploaded artifact. The search
box loads it from `${base}/pagefind/pagefind.js` beside the deployed pages.
