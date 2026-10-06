Dear Bitcoiners,

It is time to think about UX.

Over the last year I spent a lot of time experimenting with libraries and SDK providers, trying to solve a problem I see every day.

One problem lands on my desk more than any other: miners trying to set up Lightning payouts on OCEAN. I have to be honest, it is not a trivial setup. OCEAN pays over BOLT12, and to register an offer you must sign a message with the key behind your payout address, using BIP-322. Most on-chain wallets cannot produce that signature. The details are in OCEAN's docs: https://ocean.xyz/docs/lightning

So I started digging into the ecosystem. Until 2025, doing this meant pairing a Lightning wallet with a hardware wallet just to produce the signature, and once that was done you still had to manage your own liquidity.

At the time I looked at the Breez SDK with its Liquid implementation. Its offers carried a minimum amount, and I found a way around it: an offer is not signed, so you can strip that field and send the invoice request anyway. That implementation went away before I had the chance to test the idea properly.

By then the UX problem was clear. We needed an SDK that abstracts away liquidity management and still gives you the secret you need to sign the BIP-322 message that finishes the OCEAN setup.

In May 2025, as tech lead of the Lightning team at OCEAN, I introduced the OCEAN team to Lexe (https://www.lexe.app). Their solution is technically solid and built on a stable foundation, LDK, and these days are proving that intuition right. Early on I started pointing miners who were struggling with liquidity to Lexe's closed beta.

But Lexe alone did not solve the signing step: a miner still had to BIP-322 sign a message for their specific offer. I started building a CLI that took an on-chain wallet and signed an offer-compatible message. I got close, but there were too many wallet combinations to support.

In Viareggio I met the Lexe team again and told them my idea: a wallet with a setup wizard built for OCEAN. They pointed me to an unstable feature of their Rust SDK that provisions a Lexe wallet from your own root seed, with the node running inside their TEE. Hell yeah, that was exactly what I was looking for. One seed could now hold both the Lightning wallet and the key that signs for OCEAN. I could build the wizard, and show wallet providers that a better UX is possible.

Today I am releasing the side project I started out of frustration, after watching BOLT12 payouts on OCEAN cause problems that no longer need to exist. It is called Landfall, a showcase Lightning wallet powered by @lexeapp.

Why Landfall? Landfall is the moment a voyage reaches land. For a miner, it is the moment the rewards cross from the ocean into a wallet they own. The name also stays neutral on purpose: it borrows no pool's marks, so any pool or wallet can take the ideas and reuse them.

From a single recovery phrase, Landfall gives you an on-chain payout address and a BOLT12 offer that work with OCEAN. Point any miner at the address and start earning sats you can spend, let an agent keep an eye on them, or support me with a tip at vincenzopalazzo@sonarprivacy.xyz :P

What Landfall does, in three steps: it derives your payout address from the phrase, so the key that signs is provably the key that owns the address. The same phrase is the root seed of your Lexe node, which creates a payable BOLT12 offer. Then you paste the verification message OCEAN gives you, Landfall signs it with BIP-322 and shows you exactly what you signed and with which address. One backup in, three values out. Every screen of that flow is in the README, if you want to see it before you run it: https://github.com/vincenzopalazzo/landfall#how-the-onboarding-works

Be clear about one thing: Landfall is a showcase, not a production wallet. Use it to receive your OCEAN payouts and spend them. Do not park a large balance in it, and do not restore the recovery phrase of a wallet you care about into it. A showcase gets no audit, no release schedule and no support. If you would like it to become a real product, say so: open an issue, send a tip, tell OCEAN. With enough interest, OCEAN may consider picking it up and maintaining it properly.

In short:

- One 24-word recovery phrase runs both your Lightning wallet and your payout address. One backup.
- Landfall signs OCEAN's message for you with BIP-322, and shows you exactly what you signed.
- Lexe runs the node and handles the liquidity. You keep the keys.
- A web wizard, a desktop app and a CLI, all open source under AGPL.
- It is a showcase. Take the UX ideas and build them into your own wallet, or tell us you want it to become real.

Code: https://github.com/vincenzopalazzo/landfall
