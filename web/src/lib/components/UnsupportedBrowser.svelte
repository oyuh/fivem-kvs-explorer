<script lang="ts">
	import Icon from './Icon.svelte';

	// Shown when the File System Access API is missing. Brave is a special case:
	// it ships the API but disabled behind a flag, so we give it tailored steps.
	interface Props {
		isBrave?: boolean;
		onsample?: () => void;
	}
	let { isBrave = false, onsample }: Props = $props();

	let copied = $state(false);
	async function copyFlag() {
		try {
			await navigator.clipboard.writeText('brave://flags/#file-system-access-api');
			copied = true;
			setTimeout(() => (copied = false), 1500);
		} catch {
			/* clipboard blocked */
		}
	}
</script>

<div class="wrap">
	<div class="content">
		<div class="icon"><Icon name={isBrave ? 'settings' : 'alert'} size={32} strokeWidth={1.5} /></div>

		{#if isBrave}
			<h1>Brave needs one setting enabled</h1>
			<p>
				Brave ships the <strong>File System Access API</strong> turned off for privacy, and KVS
				Explorer needs it to read your folder. Three steps turn it back on.
			</p>
			<ol class="steps">
				<li>
					Open <code>brave://flags/#file-system-access-api</code>
					<button class="copy" onclick={copyFlag}>{copied ? 'Copied' : 'Copy'}</button>
					<span class="dim">(paste it into a new tab; <code>brave://</code> links aren't clickable)</span>
				</li>
				<li>Set <strong>"File System Access API"</strong> to <strong>Enabled</strong>.</li>
				<li>Click <strong>Relaunch</strong>, then reload this page.</li>
			</ol>
			<p class="fine">Chrome and Edge read the folder without any setting changes.</p>
		{:else}
			<h1>This browser isn't supported</h1>
			<p>
				KVS Explorer reads and writes your FiveM KVS folder on your own machine through the
				<strong>File System Access API</strong>. Only Chromium browsers ship that API.
			</p>
			<p class="browsers">Open this page in one of these instead:</p>
			<ul>
				<li><strong>Google Chrome</strong></li>
				<li><strong>Microsoft Edge</strong></li>
				<li>Opera</li>
				<li>Brave <span class="dim">(needs one flag enabled)</span></li>
			</ul>
			<p class="fine">
				Your data never leaves your computer either way. The only blocker is which browsers ship
				the API that reads the folder.
			</p>
		{/if}

		{#if onsample}
			<div class="demo">
				<button class="primary" onclick={onsample}>Try the demo</button>
				<p class="fine">
					The demo reads a bundled sample store over HTTP, so it runs in this browser as it is.
					Only your own folder needs the API.
				</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.wrap {
		display: grid;
		place-items: center;
		min-height: 100vh;
		padding: 24px;
	}
	.content {
		max-width: 480px;
		text-align: center;
	}
	.demo {
		margin-top: 28px;
		padding-top: 24px;
		border-top: 1px solid var(--border);
	}
	.demo .fine {
		margin-top: 12px;
	}
	.icon {
		display: flex;
		justify-content: center;
		color: var(--text-dim);
	}
	h1 {
		font-size: 20px;
		margin: 12px 0 16px;
	}
	p {
		color: var(--text-dim);
		margin: 10px 0;
	}
	.steps {
		text-align: left;
		color: var(--text);
		margin: 16px 0;
		padding-left: 20px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	code {
		font-family: var(--mono);
		font-size: 12px;
		background: var(--bg-input);
		padding: 2px 6px;
		border-radius: 2px;
		color: var(--text);
		word-break: break-all;
	}
	.copy {
		font-size: 11px;
		padding: 2px 8px;
		margin-left: 6px;
		vertical-align: middle;
	}
	.dim {
		color: var(--text-faint);
		font-size: 12px;
	}
	.browsers {
		margin-top: 18px;
		color: var(--text);
	}
	ul {
		display: inline-block;
		text-align: left;
		color: var(--text);
		margin: 4px 0 0;
	}
	.fine {
		margin-top: 20px;
		font-size: 12px;
		color: var(--text-faint);
	}
</style>
