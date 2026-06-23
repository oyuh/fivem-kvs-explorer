<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import type { EntryRow, GetResult, Group, ValueKind } from '$lib/types';
	import {
		deleteValue,
		detectBrave,
		directoryFromDrop,
		downloadBackup,
		ensureRead,
		ensureReadWrite,
		exportEntriesJson,
		getRawValue,
		getValue,
		isSupported,
		listEntries,
		loadSample,
		looksLikeLevelDb,
		openImportSource,
		openSession,
		pickDirectory,
		pickImportDirectory,
		putRawValue,
		putValue,
		readFolder,
		saveToDisk,
		type KvsSession
	} from '$lib/kvs';
	import UnsupportedBrowser from '$lib/components/UnsupportedBrowser.svelte';
	import FolderDrop from '$lib/components/FolderDrop.svelte';
	import ResourceTree from '$lib/components/ResourceTree.svelte';
	import EntryList from '$lib/components/EntryList.svelte';
	import ValueDetail from '$lib/components/ValueDetail.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Splitter from '$lib/components/Splitter.svelte';
	import ImportDialog from '$lib/components/ImportDialog.svelte';
	import AddEntryDialog from '$lib/components/AddEntryDialog.svelte';

	let supported = $state<boolean | null>(null);
	let isBrave = $state(false);
	let loading = $state(false);
	let loadError = $state<string | null>(null);

	let session = $state<KvsSession | null>(null);
	let folderName = $state('');
	let entries = $state<EntryRow[]>([]);

	let selectedGroup = $state('all');
	let search = $state('');
	let selectedKey = $state<string | null>(null);
	let detail = $state<GetResult | null>(null);
	let modifiedKeys = $state<Set<string>>(new Set());

	let saving = $state(false);
	let toast = $state<{ kind: 'ok' | 'err'; msg: string } | null>(null);

	// Resizable pane widths (persisted).
	let leftW = $state(240);
	let midW = $state(340);

	// Import.
	let importOpen = $state(false);
	let importSource = $state<KvsSession | null>(null);
	let importEntries = $state<EntryRow[]>([]);
	let importFolderName = $state('');
	let dragImport = $state(false);
	let dragDepth = 0;

	// The bundled sample data is a dev-only convenience; never shipped to prod.
	const dev = import.meta.env.DEV;

	// Add-entry dialog.
	let addOpen = $state(false);
	let addDefaultResource = $state('');

	// Theme + bulk selection.
	let theme = $state<'dark' | 'light'>('dark');
	let bulkSelected = $state<Set<string>>(new Set());

	onMount(() => {
		theme = document.documentElement.dataset.theme === 'light' ? 'light' : 'dark';
		const force = new URLSearchParams(location.search).get('force');
		if (force === 'brave' || force === 'unsupported') {
			supported = false;
			isBrave = force === 'brave';
			return;
		}
		supported = isSupported();
		detectBrave().then((b) => (isBrave = b));
		const l = Number(localStorage.getItem('kvs.leftW'));
		const m = Number(localStorage.getItem('kvs.midW'));
		if (l) leftW = l;
		if (m) midW = m;
	});

	$effect(() => {
		// Persist pane widths.
		localStorage.setItem('kvs.leftW', String(leftW));
		localStorage.setItem('kvs.midW', String(midW));
	});

	const clampW = (v: number, min: number, max: number) => Math.max(min, Math.min(max, v));

	async function handlePick(dir: FileSystemDirectoryHandle) {
		loading = true;
		loadError = null;
		try {
			if (!(await ensureReadWrite(dir))) {
				loadError =
					'Read & write permission was denied. If you picked the folder from inside AppData, copy it to your Desktop first — browsers block AppData.';
				return;
			}
			const files = await readFolder(dir);
			if (!looksLikeLevelDb(files)) {
				loadError =
					'That folder has no CURRENT file, so it isn’t a FiveM kvs (LevelDB) database. Pick the “kvs” folder itself.';
				return;
			}
			const s = await openSession(dir, files);
			session = s;
			folderName = dir.name;
			entries = listEntries(s);
			selectedGroup = 'all';
			selectedKey = null;
			search = '';
			modifiedKeys = new Set();
			bulkSelected = new Set();
		} catch (err) {
			loadError = `Couldn’t open the database: ${err}`;
		} finally {
			loading = false;
		}
	}

	async function openFolder() {
		try {
			await handlePick(await pickDirectory());
		} catch (err) {
			if ((err as DOMException)?.name !== 'AbortError') loadError = String(err);
		}
	}

	async function loadSampleData() {
		loading = true;
		loadError = null;
		try {
			const files = await loadSample(base);
			const s = await openSession(null, files);
			session = s;
			folderName = 'sample data';
			entries = listEntries(s);
			selectedGroup = 'all';
			selectedKey = null;
			search = '';
			modifiedKeys = new Set();
			bulkSelected = new Set();
		} catch (err) {
			loadError = `Couldn’t load sample data: ${err}`;
		} finally {
			loading = false;
		}
	}

	function refreshEntries() {
		if (session) entries = listEntries(session);
	}

	$effect(() => {
		const key = selectedKey;
		detail = session && key ? getValue(session, key) : null;
	});

	function onsave(rawKey: string, value: unknown, kind: ValueKind) {
		if (!session) return;
		try {
			putValue(session, rawKey, value, kind);
			modifiedKeys = new Set(modifiedKeys).add(rawKey);
			refreshEntries();
			detail = getValue(session, rawKey);
			flash('ok', 'Change staged — Save to disk to write it.');
		} catch (err) {
			flash('err', `Edit failed: ${err}`);
		}
	}

	function ondelete(rawKey: string) {
		if (!session) return;
		try {
			deleteValue(session, rawKey);
			modifiedKeys = new Set(modifiedKeys).add(rawKey);
			if (selectedKey === rawKey) selectedKey = null;
			refreshEntries();
			flash('ok', 'Key removed — Save to disk to apply.');
		} catch (err) {
			flash('err', `Delete failed: ${err}`);
		}
	}

	async function save() {
		if (!session || modifiedKeys.size === 0) return;
		saving = true;
		try {
			if (!session.backedUp) downloadBackup(session);
			const res = await saveToDisk(session);
			modifiedKeys = new Set();
			flash('ok', `Saved. ${res.written} file(s) written, ${res.deleted} removed.`);
		} catch (err) {
			flash('err', `Save failed: ${err}`);
		} finally {
			saving = false;
		}
	}

	function closeFolder() {
		if (
			modifiedKeys.size > 0 &&
			!confirm('You have unsaved changes. Discard them and close this folder?')
		)
			return;
		session = null;
		entries = [];
		selectedKey = null;
		detail = null;
		modifiedKeys = new Set();
		bulkSelected = new Set();
		loadError = null;
	}

	// --- import ---
	async function startImport(dir: FileSystemDirectoryHandle) {
		if (!session) return;
		try {
			if (!(await ensureRead(dir))) {
				flash('err', 'Read permission was denied for that folder.');
				return;
			}
			const files = await readFolder(dir);
			if (!looksLikeLevelDb(files)) {
				flash('err', 'That folder isn’t a kvs (LevelDB) database.');
				return;
			}
			const src = await openImportSource(files);
			importSource = src;
			importFolderName = dir.name;
			importEntries = listEntries(src);
			importOpen = true;
		} catch (err) {
			flash('err', `Couldn’t open import folder: ${err}`);
		}
	}

	async function openImport() {
		try {
			await startImport(await pickImportDirectory());
		} catch (err) {
			if ((err as DOMException)?.name !== 'AbortError') flash('err', String(err));
		}
	}

	function confirmImport(keys: string[]) {
		if (!session || !importSource) return;
		const imported: string[] = [];
		for (const key of keys) {
			const bytes = getRawValue(importSource, key);
			if (bytes) {
				putRawValue(session, key, bytes);
				imported.push(key);
			}
		}
		modifiedKeys = new Set([...modifiedKeys, ...imported]);
		refreshEntries();
		closeImport();
		flash('ok', `Imported ${imported.length} key${imported.length === 1 ? '' : 's'} — Save to disk to write them.`);
	}

	function closeImport() {
		importOpen = false;
		importSource = null;
		importEntries = [];
		importFolderName = '';
	}

	function openAddKey() {
		addDefaultResource = selectedGroup.startsWith('res:') ? selectedGroup.slice(4) : '';
		addOpen = true;
	}
	function openAddResource() {
		addDefaultResource = '';
		addOpen = true;
	}
	function confirmAdd(rawKey: string, value: unknown, kind: ValueKind) {
		if (!session) return;
		try {
			putValue(session, rawKey, value, kind);
			modifiedKeys = new Set(modifiedKeys).add(rawKey);
			refreshEntries();
			if (rawKey.startsWith('res:')) selectedGroup = `res:${rawKey.slice(4).split(':')[0]}`;
			search = '';
			selectedKey = rawKey;
			addOpen = false;
			flash('ok', 'Entry added — Save to disk to write it.');
		} catch (err) {
			flash('err', `Add failed: ${err}`);
		}
	}

	function toggleTheme() {
		theme = theme === 'dark' ? 'light' : 'dark';
		document.documentElement.dataset.theme = theme;
		try {
			localStorage.setItem('kvs.theme', theme);
		} catch {
			/* ignore */
		}
	}

	function onsaveraw(rawKey: string, bytes: Uint8Array) {
		if (!session) return;
		try {
			putRawValue(session, rawKey, bytes);
			modifiedKeys = new Set(modifiedKeys).add(rawKey);
			refreshEntries();
			detail = getValue(session, rawKey);
			flash('ok', 'Raw bytes staged — Save to disk to write.');
		} catch (err) {
			flash('err', `Edit failed: ${err}`);
		}
	}

	function toggleBulk(key: string) {
		const next = new Set(bulkSelected);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		bulkSelected = next;
	}
	function toggleAllFiltered() {
		const all = filtered.length > 0 && filtered.every((e) => bulkSelected.has(e.rawKey));
		const next = new Set(bulkSelected);
		for (const e of filtered) {
			if (all) next.delete(e.rawKey);
			else next.add(e.rawKey);
		}
		bulkSelected = next;
	}
	function clearBulk() {
		bulkSelected = new Set();
	}
	function bulkExport() {
		if (session && bulkSelected.size) {
			exportEntriesJson(session, [...bulkSelected]);
			flash('ok', `Exported ${bulkSelected.size} entr${bulkSelected.size === 1 ? 'y' : 'ies'}.`);
		}
	}
	function bulkDelete() {
		if (!session || bulkSelected.size === 0) return;
		const n = bulkSelected.size;
		if (!confirm(`Delete ${n} selected key${n === 1 ? '' : 's'}?\nApplied on the next Save to disk.`))
			return;
		const next = new Set(modifiedKeys);
		for (const key of bulkSelected) {
			deleteValue(session, key);
			next.add(key);
			if (selectedKey === key) selectedKey = null;
		}
		modifiedKeys = next;
		bulkSelected = new Set();
		refreshEntries();
		flash('ok', `Removed ${n} key${n === 1 ? '' : 's'} — Save to disk to apply.`);
	}

	function onDragEnter(e: DragEvent) {
		if (!e.dataTransfer) return;
		dragDepth++;
		dragImport = true;
	}
	function onDragLeave() {
		dragDepth--;
		if (dragDepth <= 0) {
			dragDepth = 0;
			dragImport = false;
		}
	}
	async function onImportDrop(e: DragEvent) {
		e.preventDefault();
		dragImport = false;
		dragDepth = 0;
		if (!e.dataTransfer) return;
		const dir = await directoryFromDrop(e.dataTransfer);
		if (dir) startImport(dir);
		else flash('err', 'Drag a folder, not a file.');
	}

	function flash(kind: 'ok' | 'err', msg: string) {
		toast = { kind, msg };
		setTimeout(() => (toast = null), 4000);
	}

	const groups = $derived.by<Group[]>(() => {
		const res = new Map<string, number>();
		let rv = 0;
		let other = 0;
		for (const e of entries) {
			if (e.namespace === 'res' && e.resource) res.set(e.resource, (res.get(e.resource) ?? 0) + 1);
			else if (e.namespace === 'rv') rv++;
			else other++;
		}
		const list: Group[] = [{ id: 'all', label: 'All entries', count: entries.length, type: 'all' }];
		for (const name of [...res.keys()].sort((a, b) => a.localeCompare(b))) {
			list.push({ id: `res:${name}`, label: name, count: res.get(name) ?? 0, type: 'res' });
		}
		if (rv) list.push({ id: 'ns:rv', label: 'resource versions', count: rv, type: 'rv' });
		if (other) list.push({ id: 'ns:other', label: 'other keys', count: other, type: 'other' });
		return list;
	});

	const filtered = $derived.by<EntryRow[]>(() => {
		const q = search.trim().toLowerCase();
		return entries.filter((e) => {
			if (selectedGroup === 'ns:rv') {
				if (e.namespace !== 'rv') return false;
			} else if (selectedGroup === 'ns:other') {
				if (e.namespace !== 'other') return false;
			} else if (selectedGroup.startsWith('res:')) {
				if (!(e.namespace === 'res' && e.resource === selectedGroup.slice(4))) return false;
			}
			if (q && !(e.rawKey.toLowerCase().includes(q) || e.preview.toLowerCase().includes(q)))
				return false;
			return true;
		});
	});

	const selectedEntry = $derived(entries.find((e) => e.rawKey === selectedKey) ?? null);
	const readOnly = $derived(!session?.dir);
	const existingKeys = $derived(new Set(entries.map((e) => e.rawKey)));
	const resourceNames = $derived(groups.filter((g) => g.type === 'res').map((g) => g.label));
	const allFilteredSelected = $derived(
		filtered.length > 0 && filtered.every((e) => bulkSelected.has(e.rawKey))
	);
	const someFilteredSelected = $derived(filtered.some((e) => bulkSelected.has(e.rawKey)));
</script>

{#if supported === null}
	<div class="boot"></div>
{:else if !supported}
	<UnsupportedBrowser {isBrave} />
{:else}
	<div class="app">
		<header class="toolbar">
			<div class="brand">KVS Explorer</div>
			{#if session}
				<div class="folder" title={folderName}><Icon name="folder" size={14} /> {folderName}</div>
				<div class="stats">{entries.length.toLocaleString()} keys</div>
				<div class="search-wrap">
					<Icon name="search" size={14} />
					<input class="search" placeholder="Search keys & values…" bind:value={search} spellcheck="false" />
				</div>
				<div class="grow"></div>
				{#if readOnly}
					<span class="ro-badge" title="Sample data is read-only.">sample · read-only</span>
				{:else if modifiedKeys.size > 0}
					<span class="modified">{modifiedKeys.size} unsaved</span>
				{/if}
				<button onclick={openAddKey}><Icon name="plus" size={14} /> Add key</button>
				<button onclick={openImport}><Icon name="import" size={14} /> Import…</button>
				<button
					class="primary"
					disabled={readOnly || modifiedKeys.size === 0 || saving}
					title={readOnly ? 'Read-only sample data' : ''}
					onclick={save}
				>
					<Icon name="save" size={14} />
					{saving ? 'Saving…' : 'Save to disk'}
				</button>
				<button onclick={closeFolder}><Icon name="x" size={14} /> Close</button>
			{:else}
				<div class="grow"></div>
				<button class="primary" onclick={openFolder}><Icon name="folder" size={14} /> Open folder</button>
				{#if dev}<button onclick={loadSampleData}>Explore sample</button>{/if}
			{/if}
			<button class="icon-btn" title="Toggle light / dark" onclick={toggleTheme}>
				<Icon name={theme === 'dark' ? 'sun' : 'moon'} size={15} />
			</button>
		</header>

		{#if session}
			<div
				class="loaded"
				role="region"
				aria-label="entries"
				ondragenter={onDragEnter}
				ondragover={(e) => e.preventDefault()}
				ondragleave={onDragLeave}
				ondrop={onImportDrop}
			>
				<div class="panes">
					<aside class="col tree" style="width:{leftW}px">
						<ResourceTree
							{groups}
							selected={selectedGroup}
							onselect={(id) => (selectedGroup = id)}
							onaddresource={openAddResource}
						/>
					</aside>
					<Splitter onresize={(dx) => (leftW = clampW(leftW + dx, 160, 520))} />
					<section class="col entries" style="width:{midW}px">
						<div class="bulkbar">
							<span
								class="bulk-check"
								class:on={allFilteredSelected}
								class:some={someFilteredSelected && !allFilteredSelected}
								role="checkbox"
								aria-checked={allFilteredSelected}
								tabindex="0"
								title="Select all shown"
								onclick={toggleAllFiltered}
								onkeydown={(e) => {
									if (e.key === ' ' || e.key === 'Enter') {
										e.preventDefault();
										toggleAllFiltered();
									}
								}}
							>
								{#if allFilteredSelected}<Icon name="check" size={12} />{/if}
							</span>
							{#if bulkSelected.size > 0}
								<span class="bulk-count">{bulkSelected.size} selected</span>
								<div class="grow"></div>
								<button class="sm" onclick={bulkExport}><Icon name="download" size={13} /> Export</button>
								<button class="sm danger" onclick={bulkDelete}><Icon name="trash" size={13} /> Delete</button>
								<button class="sm" title="Clear selection" onclick={clearBulk}><Icon name="x" size={13} /></button>
							{:else}
								<span class="bulk-hint">{filtered.length.toLocaleString()} shown</span>
							{/if}
						</div>
						<div class="entries-list">
							<EntryList
								entries={filtered}
								{selectedKey}
								{modifiedKeys}
								selected={bulkSelected}
								onselect={(k) => (selectedKey = k)}
								ontoggle={toggleBulk}
							/>
						</div>
					</section>
					<Splitter onresize={(dx) => (midW = clampW(midW + dx, 220, 820))} />
					<section class="col detail">
						<ValueDetail entry={selectedEntry} {detail} {onsave} {onsaveraw} {ondelete} />
					</section>
				</div>
				{#if dragImport}
					<div class="drop-overlay">
						<div class="drop-card">
							<Icon name="import" size={36} strokeWidth={1.5} />
							<div>Drop a kvs folder to import from</div>
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<FolderDrop
				busy={loading}
				error={loadError}
				onpick={handlePick}
				onbrowse={openFolder}
				onsample={dev ? loadSampleData : undefined}
			/>
		{/if}

		{#if toast}
			<div class="toast {toast.kind}">{toast.msg}</div>
		{/if}
	</div>

	{#if importOpen}
		<ImportDialog
			folderName={importFolderName}
			sourceEntries={importEntries}
			{existingKeys}
			onconfirm={confirmImport}
			oncancel={closeImport}
		/>
	{/if}

	{#if addOpen}
		<AddEntryDialog
			resources={resourceNames}
			defaultResource={addDefaultResource}
			{existingKeys}
			onconfirm={confirmAdd}
			oncancel={() => (addOpen = false)}
		/>
	{/if}
{/if}

<style>
	.boot {
		min-height: 100vh;
		background: var(--bg);
	}
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		overflow: hidden;
	}
	.toolbar {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-elev);
		flex: none;
	}
	.brand {
		font-weight: 700;
		letter-spacing: -0.01em;
	}
	.folder {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-family: var(--mono);
		font-size: 12px;
		color: var(--text-dim);
		max-width: 240px;
		overflow: hidden;
		white-space: nowrap;
	}
	.folder :global(svg) {
		flex: none;
	}
	.stats {
		font-size: 12px;
		color: var(--text-faint);
	}
	.search-wrap {
		position: relative;
		display: flex;
		align-items: center;
	}
	.search-wrap :global(svg) {
		position: absolute;
		left: 9px;
		color: var(--text-faint);
		pointer-events: none;
	}
	.search {
		width: 280px;
		max-width: 34vw;
		padding-left: 30px;
	}
	.grow {
		flex: 1;
	}
	.modified {
		font-size: 12px;
		color: var(--warn);
	}
	.ro-badge {
		font-size: 11px;
		color: var(--text-dim);
		border: 1px solid var(--border-strong);
		border-radius: 10px;
		padding: 2px 9px;
		white-space: nowrap;
	}
	.loaded {
		position: relative;
		flex: 1;
		min-height: 0;
	}
	.panes {
		height: 100%;
		display: flex;
	}
	.col {
		min-height: 0;
		overflow: hidden;
	}
	.tree {
		flex: none;
		border-right: 1px solid var(--border);
		background: var(--bg-elev);
	}
	.entries {
		flex: none;
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--border);
	}
	.entries-list {
		flex: 1;
		min-height: 0;
	}
	.bulkbar {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: none;
		min-height: 38px;
		padding: 6px 10px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-elev);
	}
	.bulk-check {
		flex: none;
		width: 16px;
		height: 16px;
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
	.bulk-check.some {
		background: var(--accent-dim);
	}
	.bulk-count {
		font-size: 12px;
		color: var(--text);
	}
	.bulk-hint {
		font-size: 12px;
		color: var(--text-faint);
	}
	.bulkbar button.sm {
		font-size: 12px;
		padding: 4px 9px;
	}
	.icon-btn {
		padding: 6px;
		border-color: transparent;
		background: transparent;
	}
	.icon-btn:hover {
		border-color: var(--border-strong);
		background: var(--bg-elev-2);
	}
	.detail {
		flex: 1;
		min-width: 0;
		background: var(--bg-elev);
	}
	.drop-overlay {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: grid;
		place-items: center;
		z-index: 10;
		pointer-events: none;
	}
	.drop-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		padding: 30px 44px;
		border: 2px dashed var(--accent);
		border-radius: 14px;
		background: var(--bg-elev);
		color: var(--text);
		font-size: 15px;
	}
	.toast {
		position: fixed;
		bottom: 18px;
		left: 50%;
		transform: translateX(-50%);
		padding: 10px 16px;
		border-radius: 8px;
		font-size: 13px;
		box-shadow: 0 6px 24px rgba(0, 0, 0, 0.4);
		z-index: 60;
	}
	.toast.ok {
		background: var(--bg-elev-2);
		border: 1px solid var(--border-strong);
		color: var(--text);
	}
	.toast.err {
		background: var(--danger-dim);
		border: 1px solid var(--danger);
		color: var(--text);
	}
</style>
