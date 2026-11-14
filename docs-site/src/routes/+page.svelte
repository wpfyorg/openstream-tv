<script lang="ts">
	import { base } from '$app/paths';

	const docs = `${base}/docs/`;
</script>

<svelte:head>
	<title>OpenStream TV Help Centre</title>
	<meta http-equiv="refresh" content="0; url={docs}" />
	<link rel="canonical" href={docs} />
</svelte:head>

<p class="p-8 text-sm">
	The OpenStream TV documentation lives at <a class="underline" href={docs}>{docs}</a>.
</p>
