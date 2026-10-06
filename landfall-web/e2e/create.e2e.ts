import { test, expect, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

// QA-201 … QA-205 (docs/QA-SCENARIOS.md): the create flow of the wizard,
// driven exactly as a miner would click through it, against a real
// landfall-httpd (--mock-wallet: Lexe stubbed, seed + BIP-322 real). The
// signature the wizard shows is written to LANDFALL_QA_OUT so
// scripts/qa/web-e2e.sh can check it with `landfall verify` — the same
// check OCEAN runs on submission.

const OUT = process.env.LANDFALL_QA_OUT ?? "";

/** Text of a CopyField's value, found by its label. */
async function copyValue(page: Page, label: string): Promise<string> {
  const field = page.locator(".wz-copy").filter({ hasText: label }).first();
  await expect(field).toBeVisible({ timeout: 20_000 });
  return (await field.locator(".val").textContent())?.trim() ?? "";
}

test("QA-201/202/203/204/205 create a wallet, back it up, sign for OCEAN, hand off", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Create a new wallet/ }).click();

  // ── Recovery phrase (QA-202: Continue gated on reveal + checkbox) ──
  const words = page.locator(".wz-word .wt");
  await expect(words).toHaveCount(24, { timeout: 20_000 });
  const cont = page.getByRole("button", { name: "Continue" });
  await expect(cont).toBeDisabled();
  await page.locator(".wz-blur").click();
  const phrase = (await words.allTextContents()).map((w) => w.trim());
  expect(phrase.every((w) => /^[a-z]+$/.test(w))).toBe(true);
  await expect(cont).toBeDisabled();
  await page.locator(".wz-check input[type=checkbox]").check();
  await expect(cont).toBeEnabled();
  await cont.click();

  // ── Confirm backup (QA-203: typed words, judged only on Continue) ──
  const questions = page.locator(".wz-confirm-q");
  await expect(questions).toHaveCount(3);
  const positions: number[] = [];
  for (let qi = 0; qi < 3; qi++) {
    const label = (await questions.nth(qi).locator("label").textContent()) ?? "";
    positions.push(parseInt(/#(\d+)/.exec(label)?.[1] ?? "0", 10) - 1);
  }
  // No options to click through: three text inputs, Continue off until all filled.
  await expect(page.locator(".wz-opt")).toHaveCount(0);
  await expect(cont).toBeDisabled();
  await questions.nth(0).locator("input").fill("wrongword");
  await questions.nth(1).locator("input").fill(phrase[positions[1]]);
  await questions.nth(2).locator("input").fill(phrase[positions[2]]);
  await expect(cont).toBeEnabled(); // filled, not yet judged
  await cont.click();
  await expect(page.getByText(/don't match your recovery phrase/)).toBeVisible();
  await expect(page.getByRole("heading", { name: "Confirm your backup" })).toBeVisible();
  for (let qi = 0; qi < 3; qi++) await expect(questions.nth(qi).locator("input")).toHaveValue("");
  for (let qi = 0; qi < 3; qi++) await questions.nth(qi).locator("input").fill(phrase[positions[qi]]);
  await cont.click();

  // ── Create wallet (mock node answers instantly) ──
  await expect(page.getByRole("heading", { name: "Your wallet is ready" })).toBeVisible({
    timeout: 30_000,
  });
  const offer = await copyValue(page, "Lightning address (offer)");
  const address = await copyValue(page, "Your payout address");
  expect(offer).toMatch(/^lno1/);
  expect(address).toMatch(/^bc1q/);
  await cont.click();

  // ── Sign (QA-204: gated until the pasted message embeds the offer) ──
  await expect(page.getByRole("heading", { name: /Sign OCEAN's verification message/ })).toBeVisible();
  expect(await copyValue(page, "Payout address")).toBe(address);
  expect(await copyValue(page, "Lightning offer")).toBe(offer);
  const textarea = page.locator("textarea.wz-input");
  const signBtn = page.getByRole("button", { name: "Sign message" });
  await expect(signBtn).toBeDisabled();
  await textarea.fill("Configure OCEAN payout to lno1notthisoffer at block 1");
  await expect(page.getByText(/doesn't contain your offer/)).toBeVisible();
  await expect(signBtn).toBeDisabled();
  const message = `Configure OCEAN payout to ${offer} at block 840000`;
  await textarea.fill(message);
  await expect(signBtn).toBeEnabled();
  await signBtn.click();
  const signature = await copyValue(page, "Your signature");
  expect(signature.length).toBeGreaterThan(80);
  await expect(textarea).toBeDisabled();
  // QA-213: what was signed, with which address, is shown verbatim …
  await expect(page.getByTestId("signed-message")).toHaveText(message);
  await expect(page.getByTestId("signed-address")).toHaveText(address);
  // … and the step can be redone without "Re-run setup".
  await page.getByRole("button", { name: /Sign a different message/ }).click();
  await expect(textarea).toBeEnabled();
  await expect(textarea).toHaveValue("");
  await expect(page.locator(".wz-copy").filter({ hasText: "Your signature" })).toHaveCount(0);
  await expect(cont).toBeDisabled();
  await textarea.fill(message);
  await signBtn.click();
  expect(await copyValue(page, "Your signature")).toBe(signature); // deterministic (RFC 6979)
  await cont.click();

  // ── Hand-off ──
  await expect(page.getByRole("heading", { name: "Submit your details to OCEAN" })).toBeVisible();
  expect(await copyValue(page, "Signature")).toBe(signature);
  await page.getByRole("button", { name: /submitted these to OCEAN/ }).click();
  await expect(page.getByRole("heading", { name: /ready for Lightning payouts/ })).toBeVisible();

  if (OUT) {
    writeFileSync(OUT, JSON.stringify({ address, offer, message, signature, phrase }, null, 2));
  }

  // ── QA-205: a relaunch finds the configured wallet and lands on the profile ──
  await page.reload();
  await expect(page.getByRole("button", { name: "Profile" })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText(address).first()).toBeVisible();
});
