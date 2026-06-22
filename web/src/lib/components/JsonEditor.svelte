<script lang="ts">
	import { formatJson, highlightJson, tryParseJson } from '$lib/json';
	import Icon from './Icon.svelte';

	interface Props {
		text: string;
	}
	let { text = $bindable('') }: Props = $props();

	let ta: HTMLTextAreaElement | undefined = $state();
	let pre: HTMLPreElement | undefined = $state();

	const highlighted = $derived(highlightJson(text));
	const parse = $derived(tryParseJson(text));

	function sync() {
		if (pre && ta) {
			pre.scrollTop = ta.scrollTop;
			pre.scrollLeft = ta.scrollLeft;
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Tab' && ta) {
			e.preventDefault();
			const start = ta.selectionStart;
			const end = ta.selectionEnd;
			text = text.slice(0, start) + '  ' + text.slice(end);
			queueMicrotask(() => {
				if (ta) ta.selectionStart = ta.selectionEnd = start + 2;
			});
		}
	}

	function format() {
		try {
			text = formatJson(text);
		} catch {
			/* invalid — leave as typed */
		}
	}
</script>

<div class="json-editor">
	<div class="toolbar">
		<button type="button" onclick={format} disabled={!parse.ok} title="Beautify">
			<Icon name="wand" size={14} /> Format
		</button>
		<div class="spacer"></div>
		{#if parse.ok}
			<span class="ok"><Icon name="check" size={13} /> valid</span>
		{:else}
			<span class="bad"><Icon name="alert" size={13} /> {parse.error}</span>
		{/if}
	</div>
	<div class="surface">
		<pre bind:this={pre} class="highlight" aria-hidden="true"><code>{@html highlighted}</code></pre>
		<textarea
			bind:this={ta}
			bind:value={text}
			onscroll={sync}
			onkeydown={onKeydown}
			spellcheck="false"
			autocomplete="off"
			autocapitalize="off"
		></textarea>
	</div>
</div>

<style>
	.json-editor {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.toolbar .spacer {
		flex: 1;
	}
	.toolbar button {
		padding: 3px 9px;
		font-size: 12px;
	}
	.ok,
	.bad {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font-size: 11px;
		font-family: var(--mono);
	}
	.ok {
		color: var(--text-faint);
	}
	.bad {
		color: var(--danger);
		max-width: 60%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.surface {
		position: relative;
		height: 320px;
		min-height: 160px;
		resize: vertical;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--bg-input);
	}
	.surface pre,
	.surface textarea {
		margin: 0;
		box-sizing: border-box;
		position: absolute;
		inset: 0;
		padding: 10px 12px;
		font-family: var(--mono);
		font-size: 12.5px;
		line-height: 1.55;
		white-space: pre;
		overflow: auto;
		border: 0;
		tab-size: 2;
	}
	.surface pre {
		pointer-events: none;
	}
	.surface pre code {
		font: inherit;
	}
	.surface textarea {
		width: 100%;
		background: transparent;
		color: transparent;
		caret-color: var(--text);
		resize: none;
		outline: none;
	}
	.surface textarea::selection {
		background: rgba(255, 255, 255, 0.18);
	}
</style>
