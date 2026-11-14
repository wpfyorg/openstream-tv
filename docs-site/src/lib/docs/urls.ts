import { docsConfig } from './config.js';

/**
 * The deployment subpath that `site.url` already carries, e.g. `/openstream-tv`
 * on GitHub Pages and `''` when the site is served from a domain root.
 */
export const siteBase = docsConfig.site.url
	? new URL(docsConfig.site.url).pathname.replace(/\/$/, '')
	: '';

/**
 * Turns a request path or a `doc.href` into a site-relative path.
 *
 * Both arrive with the deployment base already applied, while `site.url` ends
 * with that same base, so joining them directly would publish URLs such as
 * `https://host/openstream-tv/openstream-tv/docs/account/`.
 */
export function sitePath(href: string): string {
	if (siteBase && (href === siteBase || href.startsWith(`${siteBase}/`))) {
		return href.slice(siteBase.length) || '/';
	}
	return href;
}

/** Absolute, published URL for a path that may already include the base. */
export function absoluteUrl(href: string): string {
	return `${docsConfig.site.url ?? ''}${sitePath(href)}`;
}
