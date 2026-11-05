package org.opebstreamstv.android;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

/**
 * Runs Wireless Debugging pairing in a disposable private process.
 *
 * libadb's pairing socket can block inside TLS/protocol I/O. Keeping that work in a
 * separate process lets the activity enforce a hard timeout or lifecycle cancellation
 * without changing or reimplementing the ADB pairing protocol.
 */
public final class AdbPairingService extends Service {
    private final IAdbPairingService.Stub binder = new IAdbPairingService.Stub() {
        @Override public boolean pair(String host, int port, String pairingCode) {
            AppAdbManager adb = null;
            try {
                adb = new AppAdbManager(getApplicationContext());
                return adb.pair(host, port, pairingCode);
            } catch (Exception ignored) {
                return false;
            } finally {
                if (adb != null) try { adb.close(); } catch (Exception ignored) { }
            }
        }

        @Override public void cancel() {
            android.os.Process.killProcess(android.os.Process.myPid());
        }
    };

    @Override public IBinder onBind(Intent intent) { return binder; }
    @Override public boolean onUnbind(Intent intent) { stopSelf(); return false; }
}
