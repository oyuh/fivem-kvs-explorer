<script lang="ts" generics="T">
	import type { Snippet } from 'svelte';

	interface Props {
		items: T[];
		/** Fixed row height in px. */
		itemHeight: number;
		/** Snippet rendering one row, given the item and its absolute index. */
		row: Snippet<[T, number]>;
		overscan?: number;
	}

	let { items, itemHeight, row, overscan = 8 }: Props = $props();

	let viewport: HTMLDivElement;
	let scrollTop = $state(0);
	let viewportH = $state(400);

	const total = $derived(items.length * itemHeight);
	const start = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - overscan));
	const count = $derived(Math.ceil(viewportH / itemHeight) + overscan * 2);
	const end = $derived(Math.min(items.length, start + count));
	const slice = $derived(items.slice(start, end));

	function onscroll() {
		scrollTop = viewport.scrollTop;
	}
</script>

<div
	class="viewport"
	bind:this={viewport}
	bind:clientHeight={viewportH}
	{onscroll}
>
	<div class="spacer" style="height:{total}px">
		<div class="items" style="transform:translateY({start * itemHeight}px)">
			{#each slice as item, i (start + i)}
				<div class="row" style="height:{itemHeight}px">
					{@render row(item, start + i)}
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.viewport {
		height: 100%;
		overflow-y: auto;
		overflow-x: hidden;
	}
	.spacer {
		position: relative;
		width: 100%;
	}
	.items {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		will-change: transform;
	}
	.row {
		overflow: hidden;
	}
</style>
