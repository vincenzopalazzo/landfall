# Design source — Ocean Lightning Mobile

The mobile UI is a 1-1 implementation of the **Ocean Lightning Mobile** design in
the Claude Design project, synced locally here.

- Project: `claude.ai/design/p/15e84679-3797-4b54-91c7-c30d2565b8bd`
- File: `Ocean Lightning Mobile.html`
- Source components (React/JSX, read-only reference):
  `mobile/app.jsx`, `mobile/screens.jsx`, `mobile/flows.jsx`, `mobile/data.jsx`,
  `mobile/ios-frame.jsx`, `mobile/mobile.css`

## What's vendored here

- `colors_and_type.css` — the design tokens. The `[data-theme="dashboard"]`
  block (dark surface + Bitcoin-orange `#f7931a`) is the source of truth for the
  Compose theme in `composeApp/src/commonMain/kotlin/xyz/ocean/mobile/theme/`.
- `assets/OCEAN-icon-white.svg` — the OCEAN mark.

## Screen ↔ Compose mapping

| Design (screens.jsx / flows.jsx) | Compose |
|---|---|
| `PoolScreen`   | `screens/PoolScreen.kt` |
| `WalletScreen` (simple/full) | `screens/WalletScreen.kt` |
| `ActivityScreen` | `screens/ActivityScreen.kt` |
| `NodeScreen`   | `screens/NodeScreen.kt` |
| `TxDetail` / `SendSheet` / `ReceiveSheet` | `sheets/Sheets.kt` |
| `data.jsx` mock data | `data/MockData.kt` |
| icon set (`M_ICONS`) | `theme/Icons.kt` (Material Symbols equivalents) |

## Fonts

The design uses Inter (Variable/Display) + Geist Mono. Drop the font files under
`composeApp/src/commonMain/composeResources/font/` and swap the placeholder
families in `theme/Type.kt` (`FontFamily.Default` / `FontFamily.Monospace`).
Re-pull the binaries from the design project's `fonts/` via the DesignSync MCP if
you don't have them locally.
