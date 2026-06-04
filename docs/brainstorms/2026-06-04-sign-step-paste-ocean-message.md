# Brainstorm: Sign step — paste OCEAN's real message + show payout address

Date: 2026-06-04

## Problem

In the `oceanln-web` wizard, the **Sign for OCEAN** step fabricates the verification
message client-side (`signForOcean()` in `store.svelte.ts` builds an
"OCEAN Lightning Payout Authorization …" string and signs it). The real OCEAN flow is
offer-first: the miner registers their payout address + BOLT12 offer on ocean.xyz, OCEAN
issues a verification **message** (embedding the offer), and the miner signs *that*. To
obtain OCEAN's message the user needs their **payout address**, but the Sign step only
embeds the address inside the (fabricated) message preview — it isn't shown as a copyable
field where it's needed.

## Clarified Problem Statement

**Goal:** Rework the **Sign for OCEAN** step into the real offer-first flow — surface the
payout address + offer as copy fields (to register on ocean.xyz), then sign OCEAN's actual
verification message pasted by the user, instead of a message fabricated in-app.

**Constraints:**
- The backend `/payout` offline path already requires the offer to be embedded in the
  message and to start with `lno1` — mirror that as client-side validation so signing can't
  400.
- The three produced artifacts stay real (address from `/init`/`/generate`, offer from
  `/offer`, signature from `/payout`).
- The payout address must be **copyable on the Sign step** specifically (decided: not a
  persistent header).

**Non-goals:**
- No OCEAN *submission* API — only the read API exists. The user registers on ocean.xyz
  manually and pastes the returned message back; not automating that round-trip.
- No changes to the dashboard or other wizard steps; no persistent address header.

**Success criteria:**
- Sign step shows the payout address (`bc1q…`) and offer (`lno1…`) as copy fields under a
  "register these with OCEAN" framing.
- A textarea accepts OCEAN's verification message; the **Sign** button stays disabled until
  the message contains the offer (inline hint when it doesn't).
- Signing calls `/payout` with the *pasted* message → real signature; the fabricated-message
  code path is removed.

## Chosen approach: Offer-first Sign step

- **Sketch:** Sign step becomes (1) copy fields for payout address + offer
  ("paste these into OCEAN's payout settings"), (2) a `<textarea>` for OCEAN's message bound
  to a new `app.oceanMessage`, (3) a **Sign** button gated on
  `app.oceanMessage.includes(app.offer)`. `signForOcean()` drops the fabricated message and
  signs `app.oceanMessage` via `/payout`.
- **Affected files:**
  - `src/lib/wizard/Sign.svelte` — rework UI: address + offer `CopyField`s, message
    textarea, validation/hint, gated Sign button; show signature after.
  - `src/lib/store.svelte.ts` — add `oceanMessage: ""` to the store; rewrite
    `signForOcean()` to use it (remove the fabricated message/nonce/timestamp); add a
    `canSign()` helper (`offer` present + non-empty message); reset `oceanMessage` in
    `restart()`.
  - Tests — `store.test.ts`: `signForOcean` posts the pasted message; gating rejects a
    message without the offer. A Sign render test: address + offer + textarea present, Sign
    disabled until the message embeds the offer.
- **Tradeoffs:** production-correct and matches the CLI/README offer-first model + the
  backend's offer-in-message guard. Costs the wizard's "fully self-contained demo" property
  — it now needs a real OCEAN round-trip to get the message. Effort: **M**.
- **Alternatives rejected (per the Q&A):** keep the auto-generated message and merely show
  the address (S, but not the real flow); both-with-a-toggle (M, more surface than needed).

## Open questions (non-blocking)

- Require the **address** to also appear in the pasted message, or just the offer? The
  backend only checks the offer, so validate the offer and treat the address as advisory.
- Keep a collapsed "what OCEAN's message looks like" example for first-timers, or omit?

Next: `/ship --from-brainstorm docs/brainstorms/2026-06-04-sign-step-paste-ocean-message.md`
