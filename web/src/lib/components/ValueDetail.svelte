<script lang="ts">
	import type { EntryRow, GetResult, ValueKind } from '$lib/types';
	import { formatJson, highlightJson, isJsonString, minifyJson, tryParseJson } from '$lib/json';
	import Icon from './Icon.svelte';
	import JsonEditor from './JsonEditor.svelte';

	interface Props {
		entry: EntryRow | null;
		detail: GetResult | null;
		onsave: (rawKey: string, value: unknown, kind: ValueKind) => void;
		ondelete: (rawKey: string) => void;
	}
	let { entry, detail, onsave, ondelete }: Props = $props();

	let draftStr = $state('');
	let draftBool = $state(false);
	let jsonText = $state('');
	let localErr = $state<string | null>(null);

	const isJson = $derived(
		detail?.type === 'string' && typeof detail.value === 'string' && isJsonString(detail.value)
	);
	const editable = $derived(['string', 'int', 'float', 'bool'].includes(detail?.type ?? ''));
	const structured = $derived(detail?.type === 'array' || detail?.type === 'map');
	const typeLabel = $derived(isJson ? 'json' : (detail?.type ?? ''));

	// Reset drafts whenever the selection (or its freshly-loaded value) changes.
	$effect(() => {
		localErr = null;
		const d = detail;
		if (d?.type === 'bool') draftBool = Boolean(d.value);
		else draftStr = d?.value == null ? '' : String(d.value);
		if (d?.type === 'string' && typeof d.value === 'string' && isJsonString(d.value)) {
			try {
				jsonText = formatJson(d.value);
			} catch {
				jsonText = d.value;
			}
		} else {
			jsonText = '';
		}
	});

	function safeMinify(s: string): string {
		try {
			return minifyJson(s);
		} catch {
			return s;
		}
	}

	const dirty = $derived.by(() => {
		if (!detail) return false;
		if (isJson) {
			const r = tryParseJson(jsonText);
			return r.ok && safeMinify(jsonText) !== safeMinify(String(detail.value));
		}
		if (detail.type === 'bool') return draftBool !== Boolean(detail.value);
		return draftStr !== (detail.value == null ? '' : String(detail.value));
	});

	function apply() {
		if (!entry || !detail) return;
		if (isJson) {
			const r = tryParseJson(jsonText);
			if (!r.ok) {
				localErr = 'Invalid JSON — fix it before saving.';
				return;
			}
			localErr = null;
			onsave(entry.rawKey, minifyJson(jsonText), 'string'); // store compact
			return;
		}
		let value: unknown;
		let kind: ValueKind;
		switch (detail.type) {
			case 'int': {
				const n = Number(draftStr);
				if (!Number.isInteger(n)) {
					localErr = 'Enter a whole number.';
					return;
				}
				value = n;
				kind = 'int';
				break;
			}
			case 'float': {
				const n = Number(draftStr);
				if (!Number.isFinite(n)) {
					localErr = 'Enter a valid number.';
					return;
				}
				value = n;
				kind = 'float';
				break;
			}
			case 'bool':
				value = draftBool;
				kind = 'bool';
				break;
			default:
				value = draftStr;
				kind = 'string';
		}
		localErr = null;
		onsave(entry.rawKey, value, kind);
	}

	function revert() {
		if (!detail) return;
		if (isJson) {
			try {
				jsonText = formatJson(String(detail.value));
			} catch {
				jsonText = String(detail.value);
			}
		} else if (detail.type === 'bool') {
			draftBool = Boolean(detail.value);
		} else {
			draftStr = detail.value == null ? '' : String(detail.value);
		}
		localErr = null;
	}

	function confirmDelete() {
		if (entry && confirm(`Delete "${entry.rawKey}"?\nApplied on the next Save to disk.`)) {
			ondelete(entry.rawKey);
		}
	}

	function safePretty(v: unknown): string {
		try {
			return JSON.stringify(v, null, 2);
		} catch {
			return String(v);
		}
	}
	const structuredHtml = $derived(structured && detail ? highlightJson(safePretty(detail.value)) : '');
</script>

{#if !entry || !detail}
	<div class="placeholder">
		<Icon name="key" size={28} />
		<p>Select a key to view and edit its value.</p>
	</div>
{:else}
	<div class="detail">
		<header>
			<div class="keyline">
				{#if entry.resource}<span class="res">{entry.resource}</span><span class="sep">/</span>{/if}
				<span class="key">{entry.key}</span>
			</div>
			<div class="meta">
				<span class="type-badge {isJson ? 'json' : ''}">{typeLabel}</span>
				<span class="bytes">{detail.byteLen} bytes</span>
				<code class="raw">{entry.rawKey}</code>
			</div>
		</header>

		<div class="body">
			{#if detail.type === 'string' && isJson}
				<div class="lbl"><Icon name="braces" size={13} /> Value (JSON)</div>
				<JsonEditor bind:text={jsonText} />
			{:else if detail.type === 'string'}
				<label for="v">Value (string)</label>
				<textarea id="v" rows="8" bind:value={draftStr} spellcheck="false"></textarea>
			{:else if detail.type === 'int'}
				<label for="v">Value (integer)</label>
				<input id="v" type="number" step="1" bind:value={draftStr} />
			{:else if detail.type === 'float'}
				<label for="v">Value (float — stored as 32-bit)</label>
				<input id="v" type="number" step="any" bind:value={draftStr} />
			{:else if detail.type === 'bool'}
				<label class="checkbox">
					<input type="checkbox" bind:checked={draftBool} />
					<span>{draftBool ? 'true' : 'false'}</span>
				</label>
			{:else if structured}
				<div class="lbl"><Icon name="braces" size={13} /> Value ({detail.type})</div>
				<pre class="readonly"><code>{@html structuredHtml}</code></pre>
				<p class="note">Editing {detail.type} values isn't supported yet — view only.</p>
			{:else}
				<div class="lbl">Value ({detail.type})</div>
				<pre class="readonly">{safePretty(detail.value)}</pre>
			{/if}

			{#if localErr}<div class="err">{localErr}</div>{/if}

			<details class="hex">
				<summary>Raw bytes (hex)</summary>
				<pre>{detail.hex || '(empty)'}</pre>
			</details>
		</div>

		<footer>
			<button class="primary" disabled={!editable || !dirty} onclick={apply}>
				<Icon name="check" size={14} /> Apply change
			</button>
			<button disabled={!dirty} onclick={revert}>
				<Icon name="revert" size={14} /> Revert
			</button>
			<div class="spacer"></div>
			<button class="danger" onclick={confirmDelete}>
				<Icon name="trash" size={14} /> Delete key
			</button>
		</footer>
	</div>
{/if}

<style>
	.placeholder,
	.detail {
		height: 100%;
		min-height: 0;
	}
	.placeholder {
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 10px;
		color: var(--text-faint);
	}
	.detail {
		display: flex;
		flex-direction: column;
	}
	header {
		padding: 14px 16px;
		border-bottom: 1px solid var(--border);
	}
	.keyline {
		font-family: var(--mono);
		font-size: 15px;
		word-break: break-all;
	}
	.res {
		color: var(--text);
		font-weight: 600;
	}
	.sep {
		color: var(--text-faint);
		margin: 0 4px;
	}
	.key {
		color: var(--text);
	}
	.meta {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-top: 8px;
	}
	.bytes {
		font-size: 11px;
		color: var(--text-faint);
	}
	.raw {
		font-family: var(--mono);
		font-size: 11px;
		color: var(--text-faint);
		margin-left: auto;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 50%;
	}
	.body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	label,
	.lbl {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: var(--text-dim);
	}
	.checkbox {
		font-family: var(--mono);
	}
	.checkbox input {
		width: auto;
	}
	.readonly {
		font-family: var(--mono);
		font-size: 12px;
		background: var(--bg-input);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 10px;
		overflow: auto;
		max-height: 320px;
		white-space: pre-wrap;
		word-break: break-word;
	}
	.readonly code {
		font: inherit;
	}
	.note {
		font-size: 12px;
		color: var(--text-faint);
		margin: 0;
	}
	.err {
		background: var(--danger-dim);
		border: 1px solid var(--danger);
		border-radius: 6px;
		padding: 8px 10px;
		font-size: 12px;
	}
	.hex {
		margin-top: 4px;
	}
	.hex summary {
		cursor: pointer;
		font-size: 12px;
		color: var(--text-dim);
	}
	.hex pre {
		font-family: var(--mono);
		font-size: 11px;
		color: var(--text-faint);
		background: var(--bg-input);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 10px;
		overflow-x: auto;
		white-space: pre-wrap;
		word-break: break-all;
		max-height: 160px;
	}
	footer {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 16px;
		border-top: 1px solid var(--border);
	}
	.spacer {
		flex: 1;
	}
</style>
