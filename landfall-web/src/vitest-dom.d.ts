// Makes the @testing-library/jest-dom matchers (toBeInTheDocument, toBeDisabled,
// …) visible to svelte-check / tsc, not just at runtime via the setup file.
import "@testing-library/jest-dom/vitest";
