<script lang="ts">
	import { onMount } from 'svelte';
	import type { ValueKind } from '$lib/types';
	import { minifyJson, tryParseJson } from '$lib/json';
	import Icon from './Icon.svelte';
	import JsonEditor from './JsonEditor.svelte';

	interface Props {
		resources: string[];
		defaultResource?: string;
		existingKeys: Set<string>;
		onconfirm: (rawKey: string, value: unknown, kind: ValueKind) => void;
		oncancel: () => void;
	}
	let { resources, defaultResource = '', existingKeys, onconfirm, oncancel }: Props = $props();

	let resource = $state('');
	onMount(() => (resource = defaultResource));
	let key = $state('');
	let vtype = $state<'string' | 'int' | 'float' | 'bool' | 'json'>('string');
	let vstr = $state('');
	let vbool = $state(false);
	let vjson = $state('{\n\t\n}');

	const rawKey = $derived(`res:${resource.trim()}:${key.trim()}`);
	const conflict = $derived(!!resource.trim() && !!key.trim() && existingKeys.has(rawKey));
	const jsonParse = $derived(vtype === 'json' ? tryParseJson(vjson) : null);

	const canAdd = $derived.by(() => {
		if (!resource.trim() || !key.trim()) return false;
		switch (vtype) {
			case 'int':
				return vstr.trim() !== '' && Number.isInteger(Number(vstr));
			case 'float':
				return vstr.trim() !== '' && Number.isFinite(Number(vstr));
			case 'json':
				return jsonParse?.ok ?? false;
			default:
				return true;
		}
	});

	function confirm() {
		if (!canAdd) return;
		let value: unknown;
		let kind: ValueKind;
		switch (vtype) {
			case 'int':
				value = Number(vstr);
				kind = 'int';
				break;
			case 'float':
				value = Number(vstr);
				kind = 'float';
				break;
			case 'bool':
				value = vbool;
				kind = 'bool';
				break;
			case 'json':
				value = minifyJson(vjson);
				kind = 'string';
				break;
			default:
				value = vstr;
				kind = 'string';
		}
		onconfirm(rawKey, value, kind);
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
			<div class="title"><Icon name="plus" size={16} /> Add entry</div>
			<button class="icon-btn" title="Close" onclick={oncancel}><Icon name="x" size={16} /></button>
		</header>

		<div class="body">
			<div class="row2">
				<div class="field">
					<label for="add-res">Resource</label>
					<input
						id="add-res"
						list="add-res-list"
						bind:value={resource}
						placeholder="my_resource"
						spellcheck="false"
						autocomplete="off"
					/>
					<datalist id="add-res-list">
						{#each resources as r (r)}<option value={r}></option>{/each}
					</datalist>
				</div>
				<div class="field">
					<label for="add-key">Key</label>
					<input
						id="add-key"
						bind:value={key}
						placeholder="my_key"
						spellcheck="false"
						autocomplete="off"
					/>
				</div>
			</div>

			<div class="field">
				<label for="add-type">Type</label>
				<select id="add-type" bind:value={vtype}>
					<option value="string">string</option>
					<option value="int">integer</option>
					<option value="float">float (32-bit)</option>
					<option value="bool">boolean</option>
					<option value="json">JSON</option>
				</select>
			</div>

			<div class="field">
				<div class="lbl">Value</div>
				{#if vtype === 'string'}
					<textarea rows="4" bind:value={vstr} spellcheck="false"></textarea>
				{:else if vtype === 'int'}
					<input type="number" step="1" bind:value={vstr} placeholder="0" />
				{:else if vtype === 'float'}
					<input type="number" step="any" bind:value={vstr} placeholder="0.0" />
				{:else if vtype === 'bool'}
					<label class="checkbox"><input type="checkbox" bind:checked={vbool} /> <span>{vbool ? 'true' : 'false'}</span></label>
				{:else if vtype === 'json'}
					<JsonEditor bind:text={vjson} />
				{/if}
			</div>

			{#if conflict}
				<div class="warn"><Icon name="alert" size={14} /> This key already exists — adding overwrites it.</div>
			{/if}
		</div>

		<footer>
			<code class="rawkey" title={rawKey}>{rawKey}</code>
			<div class="spacer"></div>
			<button onclick={oncancel}>Cancel</button>
			<button class="primary" disabled={!canAdd} onclick={confirm}>
				<Icon name="check" size={14} />
				{conflict ? 'Overwrite' : 'Add'}
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
		width: min(560px, 100%);
		max-height: 90vh;
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
	.icon-btn {
		padding: 5px;
		border-color: transparent;
		background: transparent;
	}
	.body {
		padding: 16px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}
	.row2 {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	label,
	.lbl {
		font-size: 12px;
		color: var(--text-dim);
	}
	.checkbox {
		display: flex;
		align-items: center;
		gap: 8px;
		font-family: var(--mono);
	}
	.checkbox input {
		width: auto;
	}
	.warn {
		display: flex;
		align-items: center;
		gap: 7px;
		font-size: 12px;
		color: var(--warn);
		background: var(--bg-elev-2);
		border: 1px solid var(--border-strong);
		border-radius: 6px;
		padding: 8px 10px;
	}
	footer {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 16px;
		border-top: 1px solid var(--border);
	}
	.rawkey {
		font-family: var(--mono);
		font-size: 11.5px;
		color: var(--text-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 55%;
	}
	.spacer {
		flex: 1;
	}
</style>
