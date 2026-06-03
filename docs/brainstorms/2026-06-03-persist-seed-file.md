## Clarified Problem Statement

**Goal:** Let `oceanln` persist the BIP39 seed once (during `init`) so `offer`/`payout` and re-runs derive keys without re-prompting the mnemonic every time.

**Constraints:**
- Seed lives in a single oceanln-managed file (default `~/.config/oceanln/seed`, honoring `$XDG_CONFIG_HOME`), written `0600`, plaintext — same at-rest posture as Lexe's `LEXE_ROOT_SEED_PATH` file.
- Must not break existing flows: explicit stdin pipe and `--generate` keep working; the file is a fallback, not a hard dependency.
- Keep the `MnemonicSecret` zeroize-on-drop discipline — the file read path must wrap the secret the same way (`src/sign.rs:21-36`).
- Mainnet seed handling — the file must never be world-readable and should be `.gitignore`-safe by living outside any repo.

**Non-goals:**
- No encryption / passphrase / OS keychain (explicitly deferred — plaintext+0600 chosen).
- No multi-wallet / named-profile management.
- Not changing the Lexe sidecar's own env-var contract.

**Success criteria:**
- `oceanln init` (generate or provided) writes the seed file; a subsequent `oceanln payout`/`oceanln offer` runs with no mnemonic prompt and no stdin pipe.
- If the file is absent, behavior is unchanged (prompts / reads stdin as today).
- File is created with `0600`; a permissive-perms file is rejected or repaired loudly.
- Existing tests (`tests/integration.rs` stdin path) still pass.

## Decisions (from brainstorm Q&A)

- **Source model:** Managed seed file owned by oceanln (`init` writes; `offer`/`payout` read automatically).
- **Security posture:** Plaintext + `0600` — matches `lexe-sidecar`'s `LEXE_ROOT_SEED_PATH` file.

## Approaches Considered

### Approach A: Central seed resolver, file is one source
- Sketch: Add `sign::resolve_seed()` with a fixed precedence: explicit `--seed-file <path>` flag → stdin (non-TTY) → managed file (`~/.config/oceanln/seed`) → interactive prompt. `init` gains a `store_seed()` that writes the file `0600`. `offer`/`payout`/`init` all call the resolver instead of `prompt_mnemonic()` directly.
- Affected files: `src/sign.rs` (add `resolve_seed`, `store_seed`, `seed_file_path`, perms check), `src/main.rs:54-58/118-119/233-234` (swap the three `prompt_mnemonic` call sites), `src/cli.rs` (add `--seed-file` and maybe `--no-store`), `README.md`.
- Tradeoffs: Cleanest long-term — one place defines precedence, easy to add env var or keychain later. Costs a small refactor of three call sites and new tests for the precedence ladder.
- Effort: M

### Approach B: `init` writes, callers opportunistically read (minimal)
- Sketch: `init` writes the file after deriving. `offer`/`payout` try reading the managed file first; if missing, fall back to `prompt_mnemonic()` unchanged. No new flags, no formal precedence type.
- Affected files: `src/sign.rs` (two small fns), `src/main.rs` (guard before each prompt call), `README.md`.
- Tradeoffs: Smallest diff, ships fastest. But precedence logic gets duplicated/inlined at each call site, and adding `--seed-file` or env support later means revisiting every site — the thing Approach A front-loads.
- Effort: S

### Approach C: Lexe-compatible — write the file, point Lexe at it too
- Sketch: Approach A, plus `init` writes the seed to a path that doubles as `LEXE_ROOT_SEED_PATH`, and prints/exports it so the user's `lexe-sidecar` and `oceanln` share one file. One seed artifact for the whole stack.
- Affected files: A's set + `src/main.rs:171-172` (the existing "use the same mnemonic for Lexe" hint becomes a real shared path), `README.md` §Lexe.
- Tradeoffs: Best UX cohesion with the sidecar (the project's stated north star). Slightly more surface area and a doc/security note that one plaintext file now feeds two tools.
- Effort: M

## Recommendation

**Approach A**, with the Approach C path-sharing as a fast follow. A is the right shape: a single `resolve_seed()` precedence ladder means the env-var and keychain options considered earlier become a one-line addition later instead of a re-refactor, and it keeps the three call sites in `main.rs` honest. Hold C's Lexe-path-sharing until A lands, since it adds a security/doc consideration (one plaintext file feeding two processes) worth its own review.

## Open questions

- File location: `~/.config/oceanln/seed` (XDG) vs `~/.oceanln/seed` — XDG is more conventional; confirm.
- Should `init` overwrite an existing seed file, or refuse unless `--force`? (Refusing protects against clobbering a funded seed — default to refuse.)
- On a file with looser-than-0600 perms: hard error vs auto-`chmod` + warn?
