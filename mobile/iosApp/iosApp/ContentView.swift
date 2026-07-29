import UIKit
import SwiftUI
import ComposeApp

// Embeds the Compose Multiplatform UI (Kotlin) as a UIViewController. The
// OCEAN Lightning Rust core is linked as OceanlnMobileCore.xcframework (built by
// ../core/build-ios.sh); wiring its Swift bindings into the Compose data layer
// is tracked in mobile/README.md → "iOS core binding".
struct ComposeView: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> UIViewController {
        MainViewControllerKt.MainViewController()
    }
    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        ComposeView()
            .ignoresSafeArea(.all)
    }
}
