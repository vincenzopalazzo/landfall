import UIKit
import SwiftUI
import ComposeApp

// Embeds the Compose Multiplatform UI (Kotlin) and hands it the live wallet
// core: `CoreBridge` implements the Kotlin-declared `WalletCoreBridge` protocol
// on top of the UniFFI-generated `OceanlnCore` Swift API. See
// mobile/README.md -> "iOS core binding".
struct ComposeView: UIViewControllerRepresentable {
    let bridge: CoreBridge

    func makeUIViewController(context: Context) -> UIViewController {
        MainViewControllerKt.MainViewController(bridge: bridge)
    }
    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {}
}

struct ContentView: View {
    // Built once and held by the view, not per `body` evaluation: the core owns
    // the seed path, the ocean.xyz connection pool and the BTC/USD cache, so a
    // fresh one per render would throw all three away.
    @State private var setup = Result { try CoreBridge() }

    var body: some View {
        switch setup {
        case .success(let bridge):
            ComposeView(bridge: bridge)
                .ignoresSafeArea(.all)
        case .failure(let error):
            // Storage is unavailable, so we cannot open the wallet safely.
            // Deliberately no "reinstall" advice: on iOS that deletes the
            // container, and the seed with it.
            Text(
                "OCEAN Lightning could not open its secure storage.\n\n"
                + error.localizedDescription
                + "\n\nYour wallet has not been changed. Restart the app and try again."
            )
            .foregroundColor(.red)
            .padding(28)
        }
    }
}
