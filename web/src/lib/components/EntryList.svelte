<script lang="ts">
	import type { EntryRow } from '$lib/types';
	import VirtualList from './VirtualList.svelte';

	interface Props {
		entries: EntryRow[];
		selectedKey: string | null;
		modifiedKeys: Set<string>;
		onselect: (key: string) => void;
	}

	let { entries, selectedKey, modifiedKeys, onselect }: Props = $props();

	const badgeClass = (t: string) =>
		['string', 'int', 'float', 'bool', 'bytes', 'json'].includes(t) ? t : '';
</script>

<div class="list">
	{#if entries.length === 0}
		<div class="empty">No entries match.</div>
	{:else}
		<VirtualList items={entries} itemHeight={54}>
			{#snippet row(entry: EntryRow)}
				<button
					class="entry"
					class:active={entry.rawKey === selectedKey}
					onclick={() => onselect(entry.rawKey)}
				>
					<div class="top">
						<span class="key" title={entry.rawKey}>{entry.key}</span>
						{#if modifiedKeys.has(entry.rawKey)}<span class="dot" title="Modified"></span>{/if}
						<span class="type-badge {badgeClass(entry.valueType)}">{entry.valueType}</span>
					</div>
					<div class="preview" title={entry.preview}>{entry.preview}</div>
				</button>
			{/snippet}
		</VirtualList>
	{/if}
</div>

<style>
	.list {
		height: 100%;
		min-height: 0;
	}
	.empty {
		color: var(--text-faint);
		text-align: center;
		padding: 24px;
	}
	.entry {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 100%;
		height: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
		border-radius: 0;
		padding: 8px 12px;
	}
	.entry:hover {
		background: var(--bg-elev);
	}
	.entry.active {
		background: var(--bg-elev-2);
		box-shadow: inset 3px 0 0 var(--accent);
	}
	.top {
		display: flex;
		align-items: center;
		gap: 7px;
	}
	.key {
		font-family: var(--mono);
		font-size: 13px;
		color: var(--text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
		min-width: 0;
	}
	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--warn);
		flex: none;
	}
	.preview {
		font-family: var(--mono);
		font-size: 11.5px;
		color: var(--text-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
