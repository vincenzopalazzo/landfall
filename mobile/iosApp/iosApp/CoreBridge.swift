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
final class CoreBridge: WalletCoreBridge {
    private let core: OceanlnCore

    /// Roots the core at the app's private Documents dir; the seed file lives
    /// at `<dir>/seed`, created 0600 by oceanln-common on first write.
    init() {
        let base = FileManager.default
            .urls(for: .documentDirectory, in: .userDomainMask)
            .first?
            .appendingPathComponent("ocean")
        let dir = base?.path ?? NSTemporaryDirectory() + "ocean"
        self.core = OceanlnCore(appDataDir: dir)
    }

    // ── synchronous: local seed-file reads ──

    func status() -> BridgeStatus {
        // A failure here means the seed file is unreadable. Reporting
        // "not configured" would invite the user to generate a second seed, so
        // report no mining address / no offer but keep `configured` honest by
        // rethrowing through a crash-free default only when truly absent.
        do {
            let s = try core.status()
            return BridgeStatus(
                configured: s.configured,
                miningAddress: s.miningAddress,
                offer: s.offer
            )
        } catch {
            // Surface as "configured" so the app never offers wallet creation
            // on a read error; the first real operation will report the error.
            return BridgeStatus(configured: true, miningAddress: nil, offer: nil)
        }
    }

    func backupConfirmed() -> Bool {
        core.backupConfirmed()
    }

    func confirmBackup() {
        try? core.confirmBackup()
    }

    func generate() -> BridgeWalletSetup {
        do {
            let g = try core.generate()
            return BridgeWalletSetup(mnemonic: g.mnemonic, miningAddress: g.miningAddress)
        } catch {
            // An empty mnemonic fails the Kotlin 24-word guard, which shows the
            // error state instead of walking the user into the phrase screen.
            return BridgeWalletSetup(mnemonic: nil, miningAddress: "")
        }
    }

    func importSeed(mnemonic: String) -> BridgeWalletSetup {
        do {
            let i = try core.importSeed(mnemonic: mnemonic, force: false)
            return BridgeWalletSetup(mnemonic: nil, miningAddress: i.miningAddress)
        } catch {
            return BridgeWalletSetup(mnemonic: nil, miningAddress: "")
        }
    }

    func revealSeed() -> String {
        (try? core.revealSeed().mnemonic) ?? ""
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
        // -1 / "" are the bridge's "absent" encodings for these optionals.
        let amount: UInt64? = amountSats >= 0 ? UInt64(amountSats) : nil
        let desc: String? = description.isEmpty ? nil : description
        run(onDone) { try await self.core.createInvoice(amountSats: amount, description: desc) }
    }

    func pay(
        payable: String,
        amountSats: Int64,
        note: String?,
        onDone: @escaping (BridgePayment?, String?) -> Void
    ) {
        // A fixed-amount invoice arrives as -1: the amount is already encoded in
        // the invoice and must not be overridden here.
        let amount: UInt64? = amountSats >= 0 ? UInt64(amountSats) : nil
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

    /// `CoreError.Failed` carries the human-readable detail from
    /// `oceanln-common`; anything else falls back to the Swift description.
    private static func describe(_ error: Error) -> String {
        if case let CoreError.Failed(_, detail) = error {
            return detail
        }
        return error.localizedDescription
    }
}
