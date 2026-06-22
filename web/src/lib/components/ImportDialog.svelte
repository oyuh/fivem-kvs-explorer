<script lang="ts">
	import { onMount } from 'svelte';
	import type { EntryRow } from '$lib/types';
	import Icon from './Icon.svelte';
	import VirtualList from './VirtualList.svelte';

	interface Props {
		folderName: string;
		sourceEntries: EntryRow[];
		existingKeys: Set<string>;
		onconfirm: (rawKeys: string[]) => void;
		oncancel: () => void;
	}
	let { folderName, sourceEntries, existingKeys, onconfirm, oncancel }: Props = $props();

	// Default: everything that doesn't already exist in the current store.
	let selected = $state<Set<string>>(new Set());
	onMount(() => {
		selected = new Set(
			sourceEntries.filter((e) => !existingKeys.has(e.rawKey)).map((e) => e.rawKey)
		);
	});
	let overwrite = $state(false);
	let filter = $state('');

	const conflictCount = $derived(sourceEntries.filter((e) => existingKeys.has(e.rawKey)).length);

	const shown = $derived.by(() => {
		const q = filter.trim().toLowerCase();
		if (!q) return sourceEntries;
		return sourceEntries.filter(
			(e) => e.rawKey.toLowerCase().includes(q) || e.preview.toLowerCase().includes(q)
		);
	});

	const badgeClass = (t: string) =>
		['string', 'int', 'float', 'bool', 'bytes', 'json'].includes(t) ? t : '';

	function canSelect(e: EntryRow): boolean {
		return overwrite || !existingKeys.has(e.rawKey);
	}

	function toggle(e: EntryRow) {
		if (!canSelect(e)) return;
		const next = new Set(selected);
		if (next.has(e.rawKey)) next.delete(e.rawKey);
		else next.add(e.rawKey);
		selected = next;
	}

	function toggleOverwrite() {
		overwrite = !overwrite;
		if (!overwrite) {
			const next = new Set(selected);
			for (const e of sourceEntries) if (existingKeys.has(e.rawKey)) next.delete(e.rawKey);
			selected = next;
		}
	}

	function selectAll() {
		const next = new Set(selected);
		for (const e of shown) if (canSelect(e)) next.add(e.rawKey);
		selected = next;
	}
	function selectNone() {
		const next = new Set(selected);
		for (const e of shown) next.delete(e.rawKey);
		selected = next;
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') oncancel();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onpointerdown={oncancel}>
	<div class="panel" role="dialog" aria-modal="true" tabindex="-1" onpointerdown={(e) => e.stopPropagation()}>
		<header>
			<div class="title">
				<Icon name="folder" size={16} /> Import from <span class="src">{folderName}</span>
			</div>
			<button class="icon-btn" title="Close" onclick={oncancel}><Icon name="x" size={16} /></button>
		</header>

		<div class="controls">
			<div class="search-wrap">
				<Icon name="search" size={14} />
				<input placeholder="Filter keys…" bind:value={filter} spellcheck="false" />
			</div>
			<button onclick={selectAll}>Select all</button>
			<button onclick={selectNone}>None</button>
			<label class="ow" title="Allow importing keys that already exist, replacing them">
				<input type="checkbox" checked={overwrite} onchange={toggleOverwrite} />
				Overwrite existing
			</label>
		</div>

		{#if conflictCount > 0 && !overwrite}
			<div class="conflict-note">
				{conflictCount} key{conflictCount === 1 ? '' : 's'} already exist and are skipped — enable
				“Overwrite existing” to include them.
			</div>
		{/if}

		<div class="list">
			{#if shown.length === 0}
				<div class="empty">No keys match.</div>
			{:else}
				<VirtualList items={shown} itemHeight={46}>
					{#snippet row(e: EntryRow)}
						{@const exists = existingKeys.has(e.rawKey)}
						<button
							class="row"
							class:disabled={!canSelect(e)}
							onclick={() => toggle(e)}
							disabled={!canSelect(e)}
						>
							<span class="check" class:on={selected.has(e.rawKey)}>
								{#if selected.has(e.rawKey)}<Icon name="check" size={12} />{/if}
							</span>
							<span class="rk" title={e.rawKey}>
								{#if e.resource}<span class="res">{e.resource}</span><span class="sep">/</span>{/if}{e.key}
							</span>
							{#if exists}<span class="exists">exists</span>{/if}
							<span class="type-badge {badgeClass(e.valueType)}">{e.valueType}</span>
						</button>
					{/snippet}
				</VirtualList>
			{/if}
		</div>

		<footer>
			<span class="count">{selected.size} selected</span>
			<div class="spacer"></div>
			<button onclick={oncancel}>Cancel</button>
			<button class="primary" disabled={selected.size === 0} onclick={() => onconfirm([...selected])}>
				<Icon name="check" size={14} /> Import {selected.size}
			</button>
		</footer>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		display: grid;
		place-items: center;
		z-index: 50;
		padding: 24px;
	}
	.panel {
		display: flex;
		flex-direction: column;
		width: min(640px, 100%);
		height: min(620px, 90vh);
		background: var(--bg-elev);
		border: 1px solid var(--border-strong);
		border-radius: 12px;
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 16px;
		border-bottom: 1px solid var(--border);
	}
	.title {
		display: flex;
		align-items: center;
		gap: 8px;
		font-weight: 600;
	}
	.src {
		font-family: var(--mono);
		color: var(--text-dim);
		font-weight: 400;
	}
	.icon-btn {
		padding: 5px;
		border-color: transparent;
		background: transparent;
	}
	.controls {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 16px;
		border-bottom: 1px solid var(--border);
	}
	.search-wrap {
		position: relative;
		display: flex;
		align-items: center;
		flex: 1;
	}
	.search-wrap :global(svg) {
		position: absolute;
		left: 9px;
		color: var(--text-faint);
		pointer-events: none;
	}
	.search-wrap input {
		padding-left: 30px;
	}
	.controls button {
		font-size: 12px;
		padding: 5px 10px;
	}
	.ow {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: var(--text-dim);
		white-space: nowrap;
		width: auto;
	}
	.ow input {
		width: auto;
	}
	.conflict-note {
		padding: 8px 16px;
		font-size: 12px;
		color: var(--warn);
		background: var(--bg-elev-2);
		border-bottom: 1px solid var(--border);
	}
	.list {
		flex: 1;
		min-height: 0;
	}
	.empty {
		padding: 24px;
		text-align: center;
		color: var(--text-faint);
	}
	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		height: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
		border-radius: 0;
		padding: 0 14px;
	}
	.row:hover:not(:disabled) {
		background: var(--bg-elev-2);
	}
	.row.disabled {
		opacity: 0.5;
	}
	.check {
		flex: none;
		width: 16px;
		height: 16px;
		border: 1px solid var(--border-strong);
		border-radius: 4px;
		display: grid;
		place-items: center;
		color: var(--accent-text);
	}
	.check.on {
		background: var(--accent);
		border-color: var(--accent);
	}
	.rk {
		flex: 1;
		min-width: 0;
		font-family: var(--mono);
		font-size: 12.5px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.res {
		color: var(--text-dim);
	}
	.sep {
		color: var(--text-faint);
		margin: 0 3px;
	}
	.exists {
		flex: none;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--warn);
		border: 1px solid var(--border-strong);
		border-radius: 4px;
		padding: 1px 5px;
	}
	footer {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 16px;
		border-top: 1px solid var(--border);
	}
	.count {
		font-size: 12px;
		color: var(--text-dim);
	}
	.spacer {
		flex: 1;
	}
</style>
