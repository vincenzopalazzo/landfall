import { test, expect } from "@playwright/test";

// QA-210: the server already holds a seed but setup never reached the offer
// (the script pre-seeds the fixture phrase and starts httpd with no `.offer`
// file). The wizard must offer to reveal and back up THAT phrase and continue,
// rather than demanding 24 words the user may never have seen.

const TEST_MNEMONIC =
  "music mystery deliver gospel profit blanket leaf tell photo segment letter degree " +
  "nice plastic duty canyon mammal marble bicycle economy unique find cream dune";
const KNOWN_ADDRESS = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";

test("QA-210 a stored seed without an offer is recoverable: reveal, back up, continue", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText(/already set up on this server/)).toBeVisible({ timeout: 20_000 });
  // The dead end this replaces: no 24 empty boxes.
  await expect(page.locator(".wz-import input")).toHaveCount(0);

  await page.getByRole("button", { name: /Reveal and back up/ }).click();
  const words = page.locator(".wz-word .wt");
  await expect(words).toHaveCount(24, { timeout: 20_000 });
  const cont = page.getByRole("button", { name: "Continue" });
  await expect(cont).toBeDisabled();
  await page.locator(".wz-blur").click();
  const phrase = (await words.allTextContents()).map((w) => w.trim());
  expect(phrase.join(" ")).toBe(TEST_MNEMONIC);
  await page.locator(".wz-check input[type=checkbox]").check();
  await cont.click();

  // The normal backup check follows, against the revealed phrase.
  const questions = page.locator(".wz-confirm-q");
  await expect(questions).toHaveCount(3);
  for (let qi = 0; qi < 3; qi++) {
    const label = (await questions.nth(qi).locator("label").textContent()) ?? "";
    const idx = parseInt(/#(\d+)/.exec(label)?.[1] ?? "0", 10) - 1;
    await questions.nth(qi).locator("input").fill(phrase[idx]);
  }
  await cont.click();

  await expect(page.getByRole("heading", { name: "Your wallet is ready" })).toBeVisible({
    timeout: 30_000,
  });
  const field = page.locator(".wz-copy").filter({ hasText: "Your payout address" }).first();
  expect((await field.locator(".val").textContent())?.trim()).toBe(KNOWN_ADDRESS);
});
