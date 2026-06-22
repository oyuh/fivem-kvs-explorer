<script lang="ts">
	import { directoryFromDrop } from '$lib/kvs';
	import Icon from './Icon.svelte';

	interface Props {
		busy?: boolean;
		error?: string | null;
		onpick: (dir: FileSystemDirectoryHandle) => void;
		onbrowse: () => void;
		onsample?: () => void;
	}
	let { busy = false, error = null, onpick, onbrowse, onsample }: Props = $props();

	let dragging = $state(false);
	let localError = $state<string | null>(null);

	const KVS_PARENT = '%appdata%\\CitizenFX';
	let copiedPath = $state(false);
	async function copyPath() {
		try {
			await navigator.clipboard.writeText(KVS_PARENT);
			copiedPath = true;
			setTimeout(() => (copiedPath = false), 1500);
		} catch {
			/* clipboard blocked */
		}
	}

	async function onDrop(e: DragEvent) {
		e.preventDefault();
		dragging = false;
		localError = null;
		if (!e.dataTransfer) return;
		try {
			const dir = await directoryFromDrop(e.dataTransfer);
			if (!dir) {
				localError = 'That doesn’t look like a folder. Drag the whole kvs folder, not a file.';
				return;
			}
			onpick(dir);
		} catch (err) {
			localError = String(err);
		}
	}
</script>

<div
	class="empty"
	class:dragging
	role="button"
	tabindex="-1"
	ondragover={(e) => {
		e.preventDefault();
		dragging = true;
	}}
	ondragleave={() => (dragging = false)}
	ondrop={onDrop}
>
	<div class="main">
		<div class="cta">
			{#if busy}
				<div class="spinner"></div>
				<h2>Opening…</h2>
			{:else}
				<div class="dz-icon"><Icon name="folder-open" size={52} strokeWidth={1.25} /></div>
				<h2>Open your KVS folder</h2>
				<p class="sub">Drag it anywhere here, or use the buttons.</p>
				<div class="actions">
					<button class="primary" onclick={onbrowse}>
						<Icon name="folder" size={15} /> Open folder
					</button>
					{#if onsample}
						<button onclick={onsample}>Explore sample data</button>
					{/if}
				</div>
			{/if}
			{#if error || localError}
				<div class="error">{error ?? localError}</div>
			{/if}
		</div>
	</div>

	<aside class="info">
		<h3>Getting started</h3>
		<div class="callout">
			<div class="callout-title"><Icon name="alert" size={15} /> Copy the folder out of AppData</div>
			<p>Browsers block <code>AppData</code>, so work on a copy:</p>
			<ol class="steps">
				<li><strong>Close FiveM.</strong></li>
				<li>
					<kbd>Win</kbd>+<kbd>R</kbd> → <code>{KVS_PARENT}</code>
					<button type="button" class="copy" onclick={copyPath}>
						{copiedPath ? 'Copied' : 'Copy'}
					</button>
				</li>
				<li>Copy the <code>kvs</code> folder onto your <strong>Desktop</strong>.</li>
				<li>Open that copy here, edit, then <strong>Save</strong>.</li>
				<li>Copy it back into <code>{KVS_PARENT}</code> when done.</li>
			</ol>
		</div>
		<ul class="notes">
			<li>Everything runs locally — nothing is uploaded.</li>
			<li>Your real folder stays untouched until you copy it back.</li>
			<li>A backup zip is offered before your first save.</li>
		</ul>
	</aside>
</div>

<style>
	.empty {
		display: flex;
		height: 100%;
		min-height: 0;
	}
	.main {
		flex: 1;
		min-width: 0;
		display: grid;
		place-items: center;
		padding: 24px;
		border: 2px dashed transparent;
		margin: 12px;
		border-radius: 14px;
		transition: border-color 0.15s, background 0.15s;
	}
	.empty.dragging .main {
		border-color: var(--accent);
		background: var(--bg-elev);
	}
	.cta {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		max-width: 380px;
		text-align: center;
	}
	.dz-icon {
		color: var(--text-faint);
		display: flex;
	}
	h2 {
		font-size: 20px;
		margin: 6px 0 0;
	}
	.sub {
		color: var(--text-dim);
		margin: 0 0 8px;
	}
	.actions {
		display: flex;
		gap: 10px;
	}
	.error {
		margin-top: 12px;
		background: var(--danger-dim);
		border: 1px solid var(--danger);
		border-radius: 8px;
		padding: 10px 12px;
		font-size: 13px;
		white-space: pre-wrap;
	}
	.info {
		flex: none;
		width: 340px;
		border-left: 1px solid var(--border);
		background: var(--bg-elev);
		padding: 18px;
		overflow-y: auto;
	}
	.info h3 {
		margin: 0 0 14px;
		font-size: 13px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-faint);
	}
	.callout {
		background: var(--bg-elev-2);
		border: 1px solid var(--border-strong);
		border-radius: 10px;
		padding: 14px;
	}
	.callout-title {
		display: flex;
		align-items: center;
		gap: 7px;
		font-weight: 600;
		color: var(--warn);
		margin-bottom: 8px;
	}
	.callout p {
		margin: 0 0 8px;
		color: var(--text-dim);
		font-size: 13px;
	}
	.steps {
		margin: 0;
		padding-left: 18px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		font-size: 13px;
	}
	code {
		font-family: var(--mono);
		font-size: 12px;
		background: var(--bg-input);
		padding: 2px 6px;
		border-radius: 4px;
		color: var(--text);
		word-break: break-all;
	}
	kbd {
		font-family: var(--mono);
		font-size: 11px;
		background: var(--bg-input);
		border: 1px solid var(--border-strong);
		border-bottom-width: 2px;
		border-radius: 4px;
		padding: 1px 5px;
	}
	.copy {
		font-size: 11px;
		padding: 1px 8px;
	}
	.notes {
		margin: 16px 0 0;
		padding-left: 18px;
		color: var(--text-dim);
		font-size: 12.5px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.spinner {
		width: 34px;
		height: 34px;
		border: 3px solid var(--border-strong);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
