<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<title>KVS Explorer · FiveM Key-Value Store</title>
	<meta
		name="description"
		content="Explore and edit your FiveM KVS folder in the browser. The bytes never leave your machine."
	/>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
