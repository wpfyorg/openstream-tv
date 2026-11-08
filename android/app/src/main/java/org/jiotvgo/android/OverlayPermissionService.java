package org.jiotvgo.android;

import java.io.File;

/** Runs under Shizuku's shell/root identity, never the Android app process. */
public final class OverlayPermissionService extends IOverlayPermissionService.Stub {
    public OverlayPermissionService() { }

    @Override public boolean grantOverlayPermission() {
        // Limit the elevated operation to this app. Do not accept commands or package names over IPC.
        try {
            Process process = new ProcessBuilder(
                    "/system/bin/appops", "set", "org.jiotvgo.android", "SYSTEM_ALERT_WINDOW", "allow")
                    .redirectOutput(new File("/dev/null"))
                    .redirectError(new File("/dev/null"))
                    .start();
            return process.waitFor() == 0;
        } catch (Exception e) {
            return false;
        }
    }

    @Override public void destroy() { System.exit(0); }
}
