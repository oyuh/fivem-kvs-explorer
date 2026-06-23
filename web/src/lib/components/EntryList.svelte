<script lang="ts">
	import type { EntryRow } from '$lib/types';
	import Icon from './Icon.svelte';
	import VirtualList from './VirtualList.svelte';

	interface Props {
		entries: EntryRow[];
		selectedKey: string | null;
		modifiedKeys: Set<string>;
		selected: Set<string>;
		onselect: (key: string) => void;
		ontoggle: (key: string) => void;
	}

	let { entries, selectedKey, modifiedKeys, selected, onselect, ontoggle }: Props = $props();

	const badgeClass = (t: string) =>
		['string', 'int', 'float', 'bool', 'bytes', 'json'].includes(t) ? t : '';
</script>

<div class="list">
	{#if entries.length === 0}
		<div class="empty">No entries match.</div>
	{:else}
		<VirtualList items={entries} itemHeight={54}>
			{#snippet row(entry: EntryRow)}
				<div class="entry-row" class:active={entry.rawKey === selectedKey}>
					<span
						class="bulk-check"
						class:on={selected.has(entry.rawKey)}
						role="checkbox"
						aria-checked={selected.has(entry.rawKey)}
						tabindex="0"
						title="Select"
						onclick={(e) => {
							e.stopPropagation();
							ontoggle(entry.rawKey);
						}}
						onkeydown={(e) => {
							if (e.key === ' ' || e.key === 'Enter') {
								e.preventDefault();
								ontoggle(entry.rawKey);
							}
						}}
					>
						{#if selected.has(entry.rawKey)}<Icon name="check" size={12} />{/if}
					</span>
					<button class="entry" onclick={() => onselect(entry.rawKey)}>
						<div class="top">
							<span class="key" title={entry.rawKey}>{entry.key}</span>
							{#if modifiedKeys.has(entry.rawKey)}<span class="dot" title="Modified"></span>{/if}
							<span class="type-badge {badgeClass(entry.valueType)}">{entry.valueType}</span>
						</div>
						<div class="preview" title={entry.preview}>{entry.preview}</div>
					</button>
				</div>
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
	.entry-row {
		display: flex;
		align-items: center;
		height: 100%;
		border-bottom: 1px solid var(--border);
	}
	.entry-row:hover {
		background: var(--bg-elev);
	}
	.entry-row.active {
		background: var(--bg-elev-2);
		box-shadow: inset 3px 0 0 var(--accent);
	}
	.bulk-check {
		flex: none;
		width: 16px;
		height: 16px;
		margin: 0 4px 0 11px;
		border: 1px solid var(--border-strong);
		border-radius: 4px;
		display: grid;
		place-items: center;
		color: var(--accent-text);
		cursor: pointer;
	}
	.bulk-check.on {
		background: var(--accent);
		border-color: var(--accent);
	}
	.entry {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 2px;
		height: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-radius: 0;
		padding: 8px 12px 8px 8px;
	}
	.entry:hover {
		background: transparent;
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
