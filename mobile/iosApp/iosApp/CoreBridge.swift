import Foundation
import ComposeApp

// The iOS half of the wallet-core binding.
//
// Kotlin/Native cannot call the UniFFI-generated *Swift* API directly, so the
// dependency is inverted: `WalletCoreBridge` is declared in Kotlin and exported
// into ComposeApp as a protocol, and this class implements it here in Swift
// where `OceanlnCore` is available. `MainViewController(bridge:)` injects it.
//
// Mapping notes:
//   • UniFFI's async methods are `async throws`; the Kotlin side wants
//     completion handlers, so each one runs in a detached Task and reports
//     `(value, error)` — exactly one non-nil.
//   • Unsigned Rust types surface as UInt64/UInt16 in Swift but Kotlin/Native
//     does not export unsigned types, so the bridge speaks Int64 and converts
//     at this boundary.
//   • `-1` is the agreed "absent" sentinel for amounts and block heights, since
//     the Objective-C export has no optional Int64 in these positions.
enum CoreBridgeSetupError: Error, LocalizedError {
    case noDocumentsDirectory

    var errorDescription: String? {
        switch self {
        case .noDocumentsDirectory:
            return "The app's private storage directory is unavailable, so the "
                + "wallet cannot be opened safely."
        }
    }
}

final class CoreBridge: WalletCoreBridge {
    private let core: OceanlnCore

    /// Roots the core at the app's private Documents dir; the seed file lives
    /// at `<dir>/seed`, created 0600 by oceanln-common on first write.
    ///
    /// Two deliberate choices here, both about not losing the user's wallet:
    ///
    /// - **No temp-directory fallback.** `NSTemporaryDirectory()` is purgeable
    ///   by the OS, so a seed written there can vanish between launches. If the
    ///   Documents directory cannot be resolved something is deeply wrong with
    ///   the container, and failing loudly is the only safe answer.
    /// - **Excluded from iCloud/iTunes backup.** Documents is backed up by
    ///   default; a recovery phrase silently syncing off-device is not what a
    ///   self-custody wallet should do. Recovery is the 24 words, not a backup.
    init() throws {
        guard let base = FileManager.default
            .urls(for: .documentDirectory, in: .userDomainMask)
            .first?
            .appendingPathComponent("ocean")
        else {
            throw CoreBridgeSetupError.noDocumentsDirectory
        }

        try FileManager.default.createDirectory(
            at: base, withIntermediateDirectories: true
        )
        var dir = base
        var resourceValues = URLResourceValues()
        resourceValues.isExcludedFromBackup = true
        // Best-effort: on a volume that does not support the attribute this is
        // not a reason to refuse to open the wallet.
        try? dir.setResourceValues(resourceValues)

        self.core = OceanlnCore(appDataDir: base.path)
    }

    // ── synchronous: local seed-file reads ──

    // Note `core.status()` returns `configured: false` for a missing seed
    // rather than throwing, so a thrown error here is a genuine read failure.
    // It is reported, never flattened into a plausible-looking value: the
    // Kotlin cache stores results, and a fabricated "no address, no offer"
    // would then be served as truth until the entry expired.
    func status() -> BridgeStatus {
        do {
            let s = try core.status()
            return BridgeStatus(
                configured: s.configured,
                miningAddress: s.miningAddress,
                offer: s.offer,
                error: nil
            )
        } catch {
            return BridgeStatus(
                configured: false,
                miningAddress: nil,
                offer: nil,
                error: Self.describe(error)
            )
        }
    }

    func backupConfirmed() -> Bool {
        core.backupConfirmed()
    }

    /// Returns an error message, or nil on success. A silently-dropped failure
    /// here would strand the user: the app would continue as if the backup were
    /// recorded, then demand the phrase again on next launch.
    func confirmBackup() -> String? {
        do {
            try core.confirmBackup()
            return nil
        } catch {
            return Self.describe(error)
        }
    }

    func generate() -> BridgeWalletSetup {
        do {
            let g = try core.generate()
            return BridgeWalletSetup(
                mnemonic: g.mnemonic, miningAddress: g.miningAddress, error: nil
            )
        } catch {
            // Reports the real reason -- "wallet already exists" reads very
            // differently to the user than "phrase could not be read back".
            return BridgeWalletSetup(
                mnemonic: nil, miningAddress: "", error: Self.describe(error)
            )
        }
    }

    func importSeed(mnemonic: String) -> BridgeWalletSetup {
        do {
            let i = try core.importSeed(mnemonic: mnemonic, force: false)
            return BridgeWalletSetup(mnemonic: nil, miningAddress: i.miningAddress, error: nil)
        } catch {
            // Usually an invalid phrase or checksum; the user needs to be told which.
            return BridgeWalletSetup(
                mnemonic: nil, miningAddress: "", error: Self.describe(error)
            )
        }
    }

    func revealSeed() -> BridgeSeed {
        do {
            return BridgeSeed(mnemonic: try core.revealSeed().mnemonic, error: nil)
        } catch {
            return BridgeSeed(mnemonic: nil, error: Self.describe(error))
        }
    }

    func payableAmountSats(payable: String) -> KotlinLong? {
        guard let sats = try? core.payableAmountSats(payable: payable) else { return nil }
        return KotlinLong(longLong: Int64(clamping: sats))
    }

    // ── asynchronous: node + network ──

    // Kotlin exports `initWallet` as `doInitWallet`: Objective-C reserves the
    // `init*` prefix for constructors.
    func doInitWallet(onDone: @escaping (String?, String?) -> Void) {
        run(onDone) { try await self.core.initWallet(pathOverride: nil).miningAddress }
    }

    func createOffer(description: String, onDone: @escaping (String?, String?) -> Void) {
        run(onDone) { try await self.core.createOffer(description: description, minAmount: nil).offer }
    }

    func nodeStatus(onDone: @escaping (BridgeNodeStatus?, String?) -> Void) {
        run(onDone) {
            let n = try await self.core.nodeStatus()
            return BridgeNodeStatus(
                nodePk: n.nodePk,
                numChannels: Int32(clamping: n.numChannels),
                numUsableChannels: Int32(clamping: n.numUsableChannels),
                lightningTotalSats: Int64(clamping: n.lightningTotalSats),
                lightningSendableSats: Int64(clamping: n.lightningSendableSats),
                onchainTotalSats: Int64(clamping: n.onchainTotalSats),
                onchainTrustedSats: Int64(clamping: n.onchainTrustedSats),
                totalBalanceSats: Int64(clamping: n.totalBalanceSats)
            )
        }
    }

    func listPayments(limit: Int32, onDone: @escaping ([BridgeActivity]?, String?) -> Void) {
        run(onDone) {
            let rows = try await self.core.listPayments(limit: UInt16(clamping: limit))
            return rows.map { a in
                BridgeActivity(
                    id: a.id,
                    direction: a.direction,
                    rail: a.rail,
                    amountSats: Int64(clamping: a.amountSats),
                    feeSats: Int64(clamping: a.feeSats),
                    status: a.status,
                    note: a.note,
                    counterparty: a.counterparty,
                    finalizedAtMs: a.finalizedAtMs,
                    paymentHash: a.paymentHash,
                    txid: a.txid,
                    isOcean: a.isOcean,
                    blockHeight: a.blockHeight.map { Int64(clamping: $0) } ?? -1,
                    preimage: a.preimage,
                    offer: a.offer
                )
            }
        }
    }

    func createInvoice(
        amountSats: Int64,
        description: String,
        onDone: @escaping (String?, String?) -> Void
    ) {
        // NONE (-1) is the only valid "absent" encoding. Any other negative is
        // a caller bug, and quietly reinterpreting it as "amountless" would
        // mint an invoice for the wrong thing.
        guard let amount = Self.optionalAmount(amountSats) else {
            onDone(nil, "invalid invoice amount: \(amountSats)")
            return
        }
        let desc: String? = description.isEmpty ? nil : description
        run(onDone) { try await self.core.createInvoice(amountSats: amount, description: desc) }
    }

    func pay(
        payable: String,
        amountSats: Int64,
        note: String?,
        onDone: @escaping (BridgePayment?, String?) -> Void
    ) {
        // A fixed-amount invoice arrives as NONE (-1): the amount is already
        // encoded in the invoice and must not be overridden here. Any other
        // negative is rejected rather than silently becoming "let the invoice
        // decide" -- on the send path that is real money.
        guard let amount = Self.optionalAmount(amountSats) else {
            onDone(nil, "invalid payment amount: \(amountSats)")
            return
        }
        run(onDone) {
            let p = try await self.core.pay(payable: payable, amountSats: amount, note: note)
            return BridgePayment(id: p.id, amountSats: Int64(clamping: p.amountSats))
        }
    }

    func poolStats(address: String, onDone: @escaping (BridgePoolStats?, String?) -> Void) {
        run(onDone) {
            let p = try await self.core.poolStats(address: address)
            return BridgePoolStats(
                hashrate300s: p.hashrate300s,
                hashrate3600s: p.hashrate3600s,
                hashrate10800s: p.hashrate10800s,
                hashrate86400s: p.hashrate86400s,
                activeWorkers: Int32(clamping: p.activeWorkers),
                lastShareTs: p.lastShareTs,
                unpaidSats: Int64(clamping: p.unpaidSats),
                estPayoutNextBlockSats: Int64(clamping: p.estPayoutNextBlockSats),
                estEarnNextBlockSats: Int64(clamping: p.estEarnNextBlockSats),
                totalPaidSats: Int64(clamping: p.totalPaidSats),
                lifetimeSats: Int64(clamping: p.lifetimeSats),
                tidesShares: p.tidesShares,
                poolTidesShares: p.poolTidesShares,
                sharePct: p.sharePct,
                poolActiveUsers: Int64(clamping: p.poolActiveUsers),
                poolActiveWorkers: Int64(clamping: p.poolActiveWorkers),
                networkDifficulty: p.networkDifficulty,
                addressUnknown: p.addressUnknown
            )
        }
    }

    func btcUsd(onDone: @escaping (KotlinDouble) -> Void) {
        Task.detached {
            let rate = await self.core.btcUsd()
            await MainActor.run { onDone(KotlinDouble(double: rate)) }
        }
    }

    // ── plumbing ──

    /// Run an async core call and report it through a `(value, error)` handler
    /// on the main actor, which is where Compose observes state.
    private func run<T>(
        _ onDone: @escaping (T?, String?) -> Void,
        _ body: @escaping () async throws -> T
    ) {
        Task.detached {
            do {
                let value = try await body()
                await MainActor.run { onDone(value, nil) }
            } catch {
                await MainActor.run { onDone(nil, Self.describe(error)) }
            }
        }
    }

    /// Decode the bridge's amount encoding: `NONE` (-1) means absent, a
    /// non-negative value is the amount, and anything else is invalid.
    /// Returns `.some(nil)` for absent, `.some(.some(v))` for a value, and
    /// `nil` for invalid.
    private static func optionalAmount(_ raw: Int64) -> UInt64?? {
        if raw == -1 { return .some(nil) }
        if raw >= 0 { return .some(UInt64(raw)) }
        return nil
    }

    /// `CoreError.Failed` carries the human-readable detail from
    /// `oceanln-common`; anything else falls back to the Swift description.
    private static func describe(_ error: Error) -> String {
        if case let CoreError.Failed(_, detail) = error {
            return detail
        }
        return error.localizedDescription
    }
}
