import { test, expect } from "@playwright/test";

// QA-207 then QA-206: restore an existing phrase. The script starts ONE fresh
// httpd (empty seed file) per spec file, and tests run in file order, so the
// invalid-phrase case (which persists nothing) must come first; QA-206 then
// imports the known test mnemonic and checks the wizard derives the address
// that phrase is pinned to in every other test suite of the repo.

const TEST_MNEMONIC =
  "music mystery deliver gospel profit blanket leaf tell photo segment letter degree " +
  "nice plastic duty canyon mammal marble bicycle economy unique find cream dune";
const KNOWN_ADDRESS = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";

test("QA-207 an invalid phrase is refused with an error, not a dead end", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /I already have a phrase/ }).click();
  const inputs = page.locator(".wz-import input");
  for (let i = 0; i < 24; i++) await inputs.nth(i).fill("abandon");
  const cont = page.getByRole("button", { name: "Continue" });
  await expect(cont).toBeEnabled();
  await cont.click();
  // Bad checksum → server 400 → the wizard shows the error and stays here.
  await expect(page.locator(".wz-fade").getByText(/mnemonic|checksum|invalid/i).first()).toBeVisible({
    timeout: 20_000,
  });
  await expect(inputs).toHaveCount(24);
});

test("QA-206 import an existing recovery phrase and derive its payout address", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /I already have a phrase/ }).click();

  const inputs = page.locator(".wz-import input");
  await expect(inputs).toHaveCount(24);
  const cont = page.getByRole("button", { name: "Continue" });
  await expect(cont).toBeDisabled();

  const words = TEST_MNEMONIC.split(" ");
  for (let i = 0; i < 24; i++) await inputs.nth(i).fill(words[i]);
  await expect(cont).toBeEnabled();
  await cont.click();

  // Import mode skips the reveal/confirm steps and goes straight to the wallet.
  await expect(page.getByRole("heading", { name: "Your wallet is ready" })).toBeVisible({
    timeout: 30_000,
  });
  const field = page.locator(".wz-copy").filter({ hasText: "Your payout address" }).first();
  expect((await field.locator(".val").textContent())?.trim()).toBe(KNOWN_ADDRESS);
});
