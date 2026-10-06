1/ Dear Bitcoiners, it is time to think about UX.

Today I am releasing Landfall: a showcase Lightning wallet, powered by @lexeapp, that turns OCEAN's Lightning payout setup into one recovery phrase and a short wizard.

Here is why it exists 🧵

---

2/ The problem that lands on my desk more than any other: miners trying to set up Lightning payouts on OCEAN.

OCEAN pays over BOLT12, and to register an offer you must sign a message with the key behind your payout address, using BIP-322. Most on-chain wallets cannot do that.

---

3/ Until 2025, that meant pairing a Lightning wallet with a hardware wallet just to produce the signature. And once that was done, you still had to manage your own liquidity.

The details are in OCEAN's docs: https://ocean.xyz/docs/lightning

---

4/ I looked at the Breez SDK and its Liquid implementation. Its offers carried a minimum amount, and I found a way around it: an offer is not signed, so you can strip that field and send the invoice request anyway.

That implementation went away before I could test it.

---

5/ By then the UX problem was clear. We needed an SDK that abstracts away liquidity management and still gives you the secret you need to sign the BIP-322 message that finishes the OCEAN setup.

---

6/ In May 2025, as tech lead of the Lightning team at OCEAN, I introduced the OCEAN team to @lexeapp. Technically solid, built on LDK, and these days they are proving that intuition right.

I started pointing miners who struggled with liquidity to Lexe's closed beta.

---

7/ But Lexe alone did not solve the signing step: a miner still had to BIP-322 sign a message for their specific offer.

I built a CLI that took an on-chain wallet and signed an offer-compatible message. I got close, but there were too many wallet combinations to support.

---

8/ In Viareggio I met the Lexe team again and pitched a wallet with a setup wizard built for OCEAN. They pointed me to an unstable feature of their Rust SDK: provision a Lexe wallet from your own root seed, with the node running inside their TEE.

Hell yeah. That was it.

---

9/ One seed could now hold both the Lightning wallet and the key that signs for OCEAN.

So I built the wizard, out of frustration with BOLT12 payout problems that no longer need to exist, and to show wallet providers that a better UX is possible.

---

10/ Why "Landfall"? It is the moment a voyage reaches land. For a miner, it is the moment the rewards cross from the ocean into a wallet they own.

The name is neutral on purpose: it borrows no pool's marks, so any pool or wallet can take the ideas and reuse them.

---

11/ What Landfall does:

1. Derives your payout address from the phrase, so the key that signs provably owns it.
2. Uses the same phrase as the root seed of your Lexe node, which makes a BOLT12 offer.
3. Signs OCEAN's message with BIP-322 and shows exactly what you signed.

---

12/ One backup in, three values out. Point any miner at the address, earn sats you can spend, and let an agent keep an eye on them.

Every screen of the flow is in the README: https://github.com/vincenzopalazzo/landfall#how-the-onboarding-works

---

13/ ⚠️ Landfall is a showcase, not a production wallet.

Use it to receive your OCEAN payouts and spend them. Do not park a large balance in it, and do not restore the recovery phrase of a wallet you care about into it. No audit, no release schedule, no support.

---

14/ Would you like it to become a real product? Say so: open an issue, send a tip, tell OCEAN. With enough interest, OCEAN may consider picking it up and maintaining it properly.

Tips welcome at vincenzopalazzo@sonarprivacy.xyz :P

---

15/ In short:
• One 24-word phrase runs your Lightning wallet and your payout address
• Landfall signs OCEAN's message with BIP-322 and shows what you signed
• Lexe runs the node and the liquidity; you keep the keys
• Web, desktop and CLI, AGPL

Code: https://github.com/vincenzopalazzo/landfall
