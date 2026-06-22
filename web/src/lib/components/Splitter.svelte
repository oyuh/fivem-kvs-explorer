<script lang="ts">
	// A thin vertical drag handle. Reports horizontal pointer deltas; the parent
	// applies them to the adjacent pane's width.
	interface Props {
		onresize: (dx: number) => void;
	}
	let { onresize }: Props = $props();

	let dragging = $state(false);
	let lastX = 0;

	function down(e: PointerEvent) {
		dragging = true;
		lastX = e.clientX;
		try {
			(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		} catch {
			/* capture may be unavailable; dragging still works */
		}
	}
	function move(e: PointerEvent) {
		if (!dragging) return;
		const dx = e.clientX - lastX;
		lastX = e.clientX;
		if (dx) onresize(dx);
	}
	function up(e: PointerEvent) {
		dragging = false;
		try {
			(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
		} catch {
			/* ignore */
		}
	}
</script>

<div
	class="splitter"
	class:dragging
	role="separator"
	aria-orientation="vertical"
	onpointerdown={down}
	onpointermove={move}
	onpointerup={up}
	onpointercancel={up}
></div>

<style>
	.splitter {
		flex: none;
		width: 7px;
		margin: 0 -3px; /* overlap neighbours so the hit area is wide but the seam is thin */
		cursor: col-resize;
		position: relative;
		z-index: 2;
		touch-action: none;
	}
	.splitter::after {
		content: '';
		position: absolute;
		inset: 0 3px;
		background: var(--border);
		transition: background 0.1s;
	}
	.splitter:hover::after,
	.splitter.dragging::after {
		background: var(--accent);
	}
</style>
