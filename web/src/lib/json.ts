// Small, dependency-free JSON helpers: detection, validation, (de)formatting,
// and a syntax highlighter that emits class-tagged spans (see app.css .tok-*).

export type ParseResult = { ok: true; value: unknown } | { ok: false; error: string };

export function tryParseJson(s: string): ParseResult {
	try {
		return { ok: true, value: JSON.parse(s) };
	} catch (e) {
		return { ok: false, error: (e as Error).message };
	}
}

/** Whether a string is a JSON object/array (the shape FiveM resources store). */
export function isJsonString(s: string): boolean {
	const t = s.trim();
	if (!(t.startsWith('{') || t.startsWith('['))) return false;
	try {
		JSON.parse(t);
		return true;
	} catch {
		return false;
	}
}

export function formatJson(s: string, indent = 2): string {
	return JSON.stringify(JSON.parse(s), null, indent);
}

export function minifyJson(s: string): string {
	return JSON.stringify(JSON.parse(s));
}

function escapeHtml(s: string): string {
	return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

const TOKEN =
	/("(?:\\.|[^"\\])*")(\s*:)?|\b(true|false)\b|\b(null)\b|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|([{}[\],])/g;

/** Highlight JSON text into HTML with .tok-* spans. Safe for {@html} (escaped). */
export function highlightJson(text: string): string {
	const esc = escapeHtml(text);
	return esc.replace(TOKEN, (m, qstr, colon, bool, nul, num, punct) => {
		if (qstr !== undefined) {
			if (colon !== undefined) {
				const c = colon.replace(':', '<span class="tok-punct">:</span>');
				return `<span class="tok-key">${qstr}</span>${c}`;
			}
			return `<span class="tok-str">${qstr}</span>`;
		}
		if (bool !== undefined) return `<span class="tok-bool">${bool}</span>`;
		if (nul !== undefined) return `<span class="tok-null">${nul}</span>`;
		if (num !== undefined) return `<span class="tok-num">${num}</span>`;
		if (punct !== undefined) return `<span class="tok-punct">${punct}</span>`;
		return m;
	});
}
