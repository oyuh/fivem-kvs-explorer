// Pure client-side SPA. We disable SSR because the whole app relies on
// browser-only APIs (the File System Access API and WebAssembly), and every
// byte of the user's KVS stays on their machine — nothing is ever sent to a
// server. Combined with adapter-static's `fallback`, this builds to plain
// static files hostable anywhere.
export const ssr = false;
export const prerender = false;
