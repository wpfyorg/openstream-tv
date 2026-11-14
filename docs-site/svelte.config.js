import adapter from '@sveltejs/adapter-static';
import { mdsvex } from 'mdsvex';
import { createHighlighter } from 'shiki';
import { transformerNotationHighlight, transformerMetaHighlight } from '@shikijs/transformers';
import { remarkCallouts } from './src/lib/markdown/remark-callouts.js';

const highlighter = await createHighlighter({
	themes: ['github-dark', 'github-light'],
	langs: [
		'typescript',
		'javascript',
		'svelte',
		'bash',
		'shell',
		'json',
		'css',
		'html',
		'markdown',
		'yaml',
		'toml',
		'powershell',
		'diff',
		'text'
	]
});

/**
 * @param {string} code
 * @param {string | undefined} lang
 * @param {string | undefined} meta
 */
function codeHighlighter(code, lang, meta) {
	// Parse title from meta: title="filename.ts"
	const titleMatch = meta?.match(/title="([^"]+)"/);
	const title = titleMatch?.[1];

	const html = highlighter.codeToHtml(code, {
		lang: lang || 'text',
		themes: { light: 'github-light', dark: 'github-dark' },
		meta: meta ? { __raw: meta } : undefined,
		transformers: [transformerMetaHighlight(), transformerNotationHighlight()]
	});

	let result = html;

	// Wrap with title header if present
	if (title) {
		result = `<div class="code-block-titled"><div class="code-block-title">${title}</div>${result}</div>`;
	}

	return `{@html \`${result.replace(/`/g, '\\`').replace(/\$/g, '\\$')}\`}`;
}

/** @type {import('@sveltejs/kit').Config} */
const config = {
	extensions: ['.svelte', '.md', '.svx'],
	preprocess: [
		mdsvex({
			extensions: ['.md', '.svx'],
			remarkPlugins: [remarkCallouts],
			highlight: {
				highlighter: codeHighlighter
			}
		})
	],
	kit: {
		// GitHub Pages serves this repository from /openstream-tv, so the
		// workflow builds with DOCS_BASE=/openstream-tv. Local builds stay at /.
		adapter: adapter({ pages: 'build', assets: 'build', fallback: undefined }),
		paths: { base: process.env.DOCS_BASE ?? '' },
		prerender: {
			// The sidebar links every article, so crawling from the docs entry
			// point renders the whole site. The feed and crawler files are not
			// linked from any page, so they are listed explicitly.
			entries: ['/', '/docs', '/sitemap.xml', '/rss.xml', '/llms.txt', '/llms-full.txt'],
			handleHttpError: 'warn',
			handleUnseenRoutes: 'warn'
		}
	},
	vitePlugin: {
		dynamicCompileOptions: ({ filename }) =>
			filename.includes('node_modules') ? undefined : { runes: true }
	}
};

export default config;
