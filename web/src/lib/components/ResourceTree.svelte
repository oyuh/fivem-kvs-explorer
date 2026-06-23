<script lang="ts">
	import type { Group } from '$lib/types';
	import Icon from './Icon.svelte';

	interface Props {
		groups: Group[];
		selected: string;
		onselect: (id: string) => void;
		onaddresource?: () => void;
	}

	let { groups, selected, onselect, onaddresource }: Props = $props();

	let filter = $state('');

	const shown = $derived(
		filter.trim()
			? groups.filter(
					(g) => g.type === 'all' || g.label.toLowerCase().includes(filter.trim().toLowerCase())
				)
			: groups
	);
</script>

<div class="tree">
	<div class="head">
		<span class="head-label">Resources</span>
		{#if onaddresource}
			<button class="add-btn" title="New resource" onclick={onaddresource}><Icon name="plus" size={14} /></button>
		{/if}
	</div>
	<div class="filter">
		<input placeholder="Filter resources…" bind:value={filter} spellcheck="false" />
	</div>
	<div class="list">
		{#each shown as g (g.id)}
			<button
				class="item"
				class:active={selected === g.id}
				class:special={g.type !== 'res'}
				onclick={() => onselect(g.id)}
				title={g.label}
			>
				<span class="label">{g.label}</span>
				<span class="count">{g.count.toLocaleString()}</span>
			</button>
		{/each}
		{#if shown.length === 0}
			<div class="empty">No matches</div>
		{/if}
	</div>
</div>

<style>
	.tree {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 7px 8px 7px 12px;
		border-bottom: 1px solid var(--border);
	}
	.head-label {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-faint);
	}
	.add-btn {
		padding: 3px 7px;
	}
	.filter {
		padding: 8px;
		border-bottom: 1px solid var(--border);
	}
	.list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 6px;
	}
	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		width: 100%;
		text-align: left;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 6px;
		padding: 6px 9px;
		margin-bottom: 1px;
	}
	.item:hover {
		background: var(--bg-elev-2);
		border-color: transparent;
	}
	.item.active {
		background: var(--accent-dim);
		border-color: var(--accent);
	}
	.item.special .label {
		color: var(--text-dim);
		font-style: italic;
	}
	.label {
		font-family: var(--mono);
		font-size: 12.5px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.count {
		font-size: 11px;
		color: var(--text-faint);
		background: var(--bg);
		border-radius: 10px;
		padding: 0 7px;
		min-width: 22px;
		text-align: center;
	}
	.item.active .count {
		color: var(--text);
	}
	.empty {
		color: var(--text-faint);
		text-align: center;
		padding: 16px;
		font-size: 13px;
	}
</style>
