import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/svelte";

// Unmount components between tests (the store is a shared singleton).
afterEach(() => cleanup());
