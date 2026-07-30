import UIKit
import SwiftUI
import ComposeApp

// Embeds the Compose Multiplatform UI (Kotlin) as a UIViewController and hands
// it the live wallet core: `CoreBridge` implements the Kotlin-declared
// `WalletCoreBridge` protocol on top of the UniFFI-generated `OceanlnCore`
// Swift API. See mobile/README.md → "iOS core binding".
struct ComposeView: UIViewControllerRepresentable {
    // One core per process: it owns the seed path, the ocean.xyz HTTP client's
    // connection pool, and the 60s BTC/USD price cache.
    private let bridge = CoreBridge()

    func makeUIViewController(context: Context) -> UIViewController {
        MainViewControllerKt.MainViewController(bridge: bridge)
    }
    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        ComposeView()
            .ignoresSafeArea(.all)
    }
}
