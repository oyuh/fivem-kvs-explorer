<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<title>KVS Explorer — FiveM Key-Value Store</title>
	<meta
		name="description"
		content="Explore and edit your FiveM KVS folder entirely in the browser. Nothing is uploaded."
	/>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
