package org.opebstreamstv.android;

import android.os.Binder;
import android.os.Parcel;
import android.os.RemoteException;
import android.util.Log;
import java.util.concurrent.TimeUnit;

/** Shizuku user service. Its only operation is the fixed startup-screen app-op. */
public final class StartupPermissionUserService extends Binder {
    private static final String TAG = "JioTVStartupSetup";
    public boolean grantStartupScreenPermission() throws RemoteException {
        ProcessBuilder builder = new ProcessBuilder("appops", "set", "org.opebstreamstv.android", "SYSTEM_ALERT_WINDOW", "allow");
        builder.redirectOutput(new java.io.File("/dev/null"));
        builder.redirectError(new java.io.File("/dev/null"));
        java.lang.Process process = null;
        try {
            process = builder.start();
            if (!process.waitFor(8, TimeUnit.SECONDS)) { process.destroyForcibly(); return false; }
            return process.exitValue() == 0;
        } catch (Exception e) { Log.w(TAG, "Fixed startup app-op failed"); return false; }
        finally { if (process != null) process.destroy(); }
    }
    @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) throws RemoteException {
        // Shizuku UserService destroy transaction (official API: 16777115).
        if (code == 16777115) {
            System.exit(0);
            return true;
        }
        if (code == FIRST_CALL_TRANSACTION) { data.enforceInterface("org.opebstreamstv.android.IStartupPermissionService"); boolean granted=grantStartupScreenPermission(); reply.writeNoException(); reply.writeInt(granted?1:0); return true; }
        return super.onTransact(code,data,reply,flags);
    }
}
