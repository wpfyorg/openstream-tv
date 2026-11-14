// The site is fully static: GitHub Pages serves the prerendered HTML.
export const prerender = true;

// Article links are authored relative to the current page (`../<slug>/`), which
// only resolves correctly when every page URL ends with a slash.
export const trailingSlash = 'always';
