package xyz.ocean.mobile

import androidx.compose.ui.window.ComposeUIViewController

// Entry point the iOS host (iosApp/ContentView.swift) embeds. Exposed to Swift
// as `MainViewControllerKt.MainViewController()` in the ComposeApp framework.
fun MainViewController() = ComposeUIViewController { App() }
