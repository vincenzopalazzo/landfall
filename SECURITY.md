# Security policy

Landfall is a showcase wallet, not a production one: it runs on Bitcoin
mainnet, has had no security audit, and has no release schedule. Reports are
still very welcome, especially anything that could leak a recovery phrase or
move funds.

## Reporting a vulnerability

Please report privately through GitHub: open the repository's **Security**
tab and choose **Report a vulnerability**. Do not open a public issue for
anything that could put a user's funds or recovery phrase at risk.

Include what you found, how to reproduce it (commit, platform, and which
part: CLI, `landfall-httpd`, `landfall-mcp`, web wizard or desktop app), and
what an attacker would need (local access, a malicious web page, a
compromised dependency, and so on). You will get an answer as soon as I can
manage; this is a personal project, so there is no fixed response time.

Only the `main` branch is supported. There are no tagged releases yet.

## Known limitations

These are design choices of the showcase, documented in the README. They are
not vulnerabilities, but a report that one of them is worse than described
is welcome.

- **The recovery phrase is stored in plaintext** on the machine that runs
  Landfall, in a file only your user can read (`0600`, parent folder
  `0700`). Anyone with access to your user account can read it.
- **Mainnet only.** There is no testnet mode; every wallet Landfall creates
  is a real Lexe wallet.
- **`landfall-httpd` trusts the local machine.** It binds to loopback only
  and requires a bearer token for every route that touches the seed, signs,
  or spends. With `--no-auth`, read-only routes (wallet status, payouts,
  activity) are open to any local process.
- **`landfall-mcp` has no token of its own.** It is read-only, binds to
  loopback and rejects browser origins, but any local process can read what
  it exposes.
- **Tokens passed as command-line flags** (`--token`, `--httpd-token`,
  `--credentials`) are visible to other local users through the process
  list.
