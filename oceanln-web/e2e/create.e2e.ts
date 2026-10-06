import { test, expect, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

// QA-201 … QA-205 (docs/QA-SCENARIOS.md): the create flow of the wizard,
// driven exactly as a miner would click through it, against a real
// oceanln-httpd (--mock-wallet: Lexe stubbed, seed + BIP-322 real). The
// signature the wizard shows is written to OCEANLN_QA_OUT so
// scripts/qa/web-e2e.sh can check it with `oceanln verify` — the same
// check OCEAN runs on submission.

const OUT = process.env.OCEANLN_QA_OUT ?? "";

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

  // ── Confirm backup (QA-203: a wrong pick is flagged and blocks) ──
  const questions = page.locator(".wz-confirm-q");
  await expect(questions).toHaveCount(3);
  for (let qi = 0; qi < 3; qi++) {
    const q = questions.nth(qi);
    const label = (await q.locator(".q").textContent()) ?? "";
    const idx = parseInt(/#(\d+)/.exec(label)?.[1] ?? "0", 10) - 1;
    const correct = phrase[idx];
    const opts = q.locator(".wz-opt");
    const texts = (await opts.allTextContents()).map((t) => t.trim());
    expect(texts).toContain(correct);
    if (qi === 0) {
      const wrong = texts.findIndex((t) => t !== correct);
      await opts.nth(wrong).click();
      await expect(q.getByText(/not the right word/)).toBeVisible();
      await expect(cont).toBeDisabled();
    }
    await opts.nth(texts.indexOf(correct)).click();
  }
  await expect(cont).toBeEnabled();
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
