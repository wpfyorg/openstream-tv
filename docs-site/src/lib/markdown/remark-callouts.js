// The articles use GitHub's alert syntax (`> [!NOTE]`), which Markdown renders
// as a blockquote with a literal marker. This rewrites those blockquotes into
// an <aside class="callout callout-note" data-label="Note"> so the stylesheet
// can present them properly.
//
// Remark parses `[!IMPORTANT]` on its own line as a shortcut link reference
// rather than text, so both shapes are recognised here.
const LABELS = {
	NOTE: 'Note',
	TIP: 'Tip',
	IMPORTANT: 'Important',
	WARNING: 'Warning',
	CAUTION: 'Caution'
};

const TEXT_MARKER = /^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*/;

export function remarkCallouts() {
	return (tree) => walk(tree);
}

function markerKind(node) {
	if (node?.type === 'text') {
		const match = node.value.match(TEXT_MARKER);
		return match ? match[1] : null;
	}
	if (node?.type === 'linkReference') {
		const name = (node.identifier || node.label || '').replace(/^!/, '').toUpperCase();
		return LABELS[name] ? name : null;
	}
	return null;
}

function trimLeadingBreak(children) {
	const next = children[0];
	if (next?.type === 'text') {
		next.value = next.value.replace(/^\s*\n/, '');
		if (!next.value) children.shift();
	}
}

function walk(node) {
	if (!node || !Array.isArray(node.children)) return;

	for (const child of node.children) {
		if (child.type === 'blockquote' && child.children?.length) {
			const paragraph = child.children[0];
			const lead = paragraph?.type === 'paragraph' ? paragraph.children?.[0] : undefined;
			const kind = markerKind(lead);

			if (kind && lead.type === 'text' && lead.value.match(TEXT_MARKER)) {
				lead.value = lead.value.replace(TEXT_MARKER, '');
				if (!lead.value) paragraph.children.shift();
			} else if (kind) {
				paragraph.children.shift();
				trimLeadingBreak(paragraph.children);
			}

			if (kind) {
				child.data = {
					hName: 'aside',
					hProperties: {
						className: ['callout', `callout-${kind.toLowerCase()}`],
						'data-label': LABELS[kind]
					}
				};
			}
		}
		walk(child);
	}
}
