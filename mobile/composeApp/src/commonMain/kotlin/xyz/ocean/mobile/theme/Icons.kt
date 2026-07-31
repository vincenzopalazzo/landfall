package xyz.ocean.mobile.theme

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material.icons.outlined.AccountBalance
import androidx.compose.material.icons.outlined.AccountBalanceWallet
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.AlternateEmail
import androidx.compose.material.icons.outlined.AutoAwesome
import androidx.compose.material.icons.outlined.CallMade
import androidx.compose.material.icons.outlined.CallReceived
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.ChevronLeft
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.CurrencyBitcoin
import androidx.compose.material.icons.outlined.Dns
import androidx.compose.material.icons.outlined.ExpandMore
import androidx.compose.material.icons.outlined.HourglassEmpty
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.LocalOffer
import androidx.compose.material.icons.outlined.Memory
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.OpenInNew
import androidx.compose.material.icons.outlined.QrCode2
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material.icons.outlined.SwapVert
import androidx.compose.material.icons.outlined.Thermostat
import androidx.compose.material.icons.outlined.ViewInAr
import androidx.compose.material.icons.outlined.Waves
import androidx.compose.ui.graphics.vector.ImageVector

// Maps the design's icon names (data.jsx M_ICONS) to Material equivalents. In
// production OCEAN uses Material Symbols, so this stays faithful to intent; the
// custom geometric OCEAN mark ("ocean") is drawn separately from the vendored
// SVG asset (see OceanMark).
fun oceanIcon(name: String): ImageVector = when (name) {
    "pool" -> Icons.Outlined.Waves
    "wallet" -> Icons.Outlined.AccountBalanceWallet
    "activity" -> Icons.Outlined.AutoAwesome
    "node", "gear" -> Icons.Outlined.Settings
    "nodeChip" -> Icons.Outlined.Dns
    "bolt", "send" -> Icons.Filled.Bolt
    "btc" -> Icons.Outlined.CurrencyBitcoin
    "in" -> Icons.Outlined.CallReceived
    "out" -> Icons.Outlined.CallMade
    "chip" -> Icons.Outlined.Memory
    "cube" -> Icons.Outlined.ViewInAr
    "hourglass" -> Icons.Outlined.HourglassEmpty
    "check" -> Icons.Outlined.Check
    "chevR" -> Icons.Outlined.ChevronRight
    "chevL" -> Icons.Outlined.ChevronLeft
    "chevD" -> Icons.Outlined.ExpandMore
    "close" -> Icons.Outlined.Close
    "key" -> Icons.Outlined.Key
    "shield" -> Icons.Outlined.Shield
    "swap" -> Icons.Outlined.SwapVert
    "at" -> Icons.Outlined.AlternateEmail
    "bank" -> Icons.Outlined.AccountBalance
    "qr" -> Icons.Outlined.QrCode2
    "search" -> Icons.Outlined.Search
    "copy" -> Icons.Outlined.ContentCopy
    "refresh" -> Icons.Outlined.Refresh
    "warn" -> Icons.Outlined.Info
    "info" -> Icons.Outlined.Info
    "ext" -> Icons.Outlined.OpenInNew
    "bell" -> Icons.Outlined.Notifications
    "clock" -> Icons.Outlined.Schedule
    "temp" -> Icons.Outlined.Thermostat
    "plus" -> Icons.Outlined.Add
    "offer" -> Icons.Outlined.LocalOffer
    "spark" -> Icons.Outlined.AutoAwesome
    else -> Icons.Outlined.Info
}
