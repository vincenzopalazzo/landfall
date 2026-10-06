import { test, expect, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { join } from "node:path";

// Not a QA scenario: walks the create flow once and saves a PNG of every
// onboarding step for the README (`scripts/screenshots.sh`). Runs against the
// same `landfall-httpd --mock-wallet` the QA runner uses, so the words, the
// address and the signature in the images are real output of the real code
// and the fixture is a throwaway. The default web-e2e run never picks this
// file up: it names its specs explicitly.

const OUT = process.env.LANDFALL_SHOTS_DIR ?? "";
test.skip(!OUT, "set LANDFALL_SHOTS_DIR");
test.use({ viewport: { width: 1280, height: 800 } });

async function shot(page: Page, name: string) {
  await page.waitForTimeout(250); // let the fade-in settle
  await page.screenshot({ path: join(OUT, `${name}.png`) });
}

test("capture every step of the onboarding", async ({ page }) => {
  mkdirSync(OUT, { recursive: true });
  await page.goto("/");
  await expect(page.getByRole("button", { name: /Create a new wallet/ })).toBeVisible();
  await shot(page, "01-welcome");

  await page.getByRole("button", { name: /Create a new wallet/ }).click();
  const words = page.locator(".wz-word .wt");
  await expect(words).toHaveCount(24, { timeout: 20_000 });
  await shot(page, "02-phrase-hidden");
  await page.locator(".wz-blur").click();
  const phrase = (await words.allTextContents()).map((w) => w.trim());
  await page.locator(".wz-check input[type=checkbox]").check();
  await shot(page, "03-phrase-revealed");
  const cont = page.getByRole("button", { name: "Continue" });
  await cont.click();

  const questions = page.locator(".wz-confirm-q");
  await expect(questions).toHaveCount(3);
  for (let qi = 0; qi < 3; qi++) {
    const label = (await questions.nth(qi).locator("label").textContent()) ?? "";
    const idx = parseInt(/#(\d+)/.exec(label)?.[1] ?? "0", 10) - 1;
    await questions.nth(qi).locator("input").fill(phrase[idx]);
  }
  await shot(page, "04-confirm");
  await cont.click();

  await expect(page.getByRole("heading", { name: "Your wallet is ready" })).toBeVisible({
    timeout: 30_000,
  });
  await shot(page, "05-wallet-ready");
  const offer =
    (await page.locator(".wz-copy").filter({ hasText: "Lightning address (offer)" }).locator(".val").textContent())?.trim() ?? "";
  await cont.click();

  await expect(page.getByRole("heading", { name: /Sign OCEAN's verification message/ })).toBeVisible();
  await shot(page, "06-sign-empty");
  await page.locator("textarea.wz-input").fill(`Configure OCEAN payout to ${offer} at block 840000`);
  await page.getByRole("button", { name: "Sign message" }).click();
  await expect(page.getByTestId("signed-message")).toBeVisible({ timeout: 20_000 });
  await shot(page, "07-signed");
  await cont.click();

  await expect(page.getByRole("heading", { name: "Submit your details to OCEAN" })).toBeVisible();
  await shot(page, "08-handoff");
  await page.getByRole("button", { name: /submitted these to OCEAN/ }).click();
  await expect(page.getByRole("heading", { name: /ready for Lightning payouts/ })).toBeVisible();
  await shot(page, "09-done");
  // The profile page after a reload is deliberately not captured: it loads
  // live OCEAN data, so offline it renders fetch-error banners.
});
