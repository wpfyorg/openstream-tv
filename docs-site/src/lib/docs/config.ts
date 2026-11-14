import type { DocsConfig } from './types.js';
import { sidebarSections } from './sidebar.generated.js';

export const docsConfig: DocsConfig = {
	site: {
		title: 'OpenStream TV Help Centre',
		description:
			'Install, connect and use OpenStream TV, then troubleshoot common problems with live TV, catch-up, the guide and IPTV playlists.',
		url: 'https://wpfyorg.github.io/openstream-tv',
		social: {
			github: 'https://github.com/wpfyorg/openstream-tv',
			discord: 'https://discord.gg/Xn3bUsBCZU'
		}
	},
	sidebar: sidebarSections.map((section) => ({
		label: section.label,
		items: section.items
	})),
	toc: {
		minDepth: 2,
		maxDepth: 3
	}
};
