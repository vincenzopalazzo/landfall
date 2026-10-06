import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/svelte";

// The test runtime's `localStorage` is an incomplete stub (no setItem). Replace
// it with a simple in-memory Storage so code that persists UI state (e.g. the
// "submitted to OCEAN" marker) works under test.
const memStore = new Map<string, string>();
vi.stubGlobal("localStorage", {
  getItem: (k: string) => (memStore.has(k) ? memStore.get(k)! : null),
  setItem: (k: string, v: string) => void memStore.set(k, String(v)),
  removeItem: (k: string) => void memStore.delete(k),
  clear: () => memStore.clear(),
  key: (i: number) => Array.from(memStore.keys())[i] ?? null,
  get length() {
    return memStore.size;
  },
});

// Unmount components between tests (the store is a shared singleton).
afterEach(() => cleanup());
