// Root build file — plugins are declared here `apply false` and applied in the
// module build files. Versions come from gradle/libs.versions.toml.
plugins {
    alias(libs.plugins.androidApplication) apply false
    alias(libs.plugins.kotlinMultiplatform) apply false
    alias(libs.plugins.composeMultiplatform) apply false
    alias(libs.plugins.composeCompiler) apply false
}
