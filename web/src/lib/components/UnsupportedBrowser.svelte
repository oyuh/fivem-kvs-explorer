<script lang="ts">
	import Icon from './Icon.svelte';

	// Shown when the File System Access API is missing. Brave is a special case:
	// it ships the API but disabled behind a flag, so we give it tailored steps.
	interface Props {
		isBrave?: boolean;
	}
	let { isBrave = false }: Props = $props();

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
	<div class="card">
		<div class="icon"><Icon name={isBrave ? 'settings' : 'alert'} size={32} strokeWidth={1.5} /></div>

		{#if isBrave}
			<h1>Brave needs one setting enabled</h1>
			<p>
				Brave ships with the <strong>File System Access API</strong> turned off for privacy. KVS
				Explorer needs it to read your folder. Turn it on in a few seconds:
			</p>
			<ol class="steps">
				<li>
					Open <code>brave://flags/#file-system-access-api</code>
					<button class="copy" onclick={copyFlag}>{copied ? 'Copied ✓' : 'Copy'}</button>
					<span class="dim">(paste it in a new tab — links to <code>brave://</code> can't be clicked)</span>
				</li>
				<li>Set <strong>“File System Access API”</strong> to <strong>Enabled</strong>.</li>
				<li>Click <strong>Relaunch</strong>, then reload this page.</li>
			</ol>
			<p class="fine">Prefer not to change Brave settings? Chrome or Edge work out of the box.</p>
		{:else}
			<h1>This browser isn't supported</h1>
			<p>
				KVS Explorer reads and writes your FiveM KVS folder directly on your machine using the
				<strong>File System Access API</strong>, which only Chromium-based browsers provide.
			</p>
			<p class="browsers">Please open this page in one of:</p>
			<ul>
				<li><strong>Google Chrome</strong></li>
				<li><strong>Microsoft Edge</strong></li>
				<li>Opera</li>
				<li>Brave <span class="dim">(needs a quick flag enabled)</span></li>
			</ul>
			<p class="fine">
				Your data never leaves your computer — the requirement is purely about the browser API needed
				to read the folder.
			</p>
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
	.card {
		max-width: 480px;
		background: var(--bg-elev);
		border: 1px solid var(--border);
		border-radius: 12px;
		padding: 32px;
		text-align: center;
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
		border-radius: 4px;
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
