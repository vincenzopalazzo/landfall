package xyz.ocean.mobile.data

import platform.Foundation.NSDate
import platform.Foundation.timeIntervalSince1970

/** Wall-clock millis. Read per call so relative timestamps stay live. */
internal fun currentTimeMillis(): Long =
    (NSDate().timeIntervalSince1970 * 1000.0).toLong()
