package org.opebstreamstv.android;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.provider.Settings;
import android.util.Log;

public final class BootReceiver extends BroadcastReceiver {
    private static final String TAG = "JioTVGoBoot";

    @Override public void onReceive(Context context, Intent intent) {
        if (intent == null || !Intent.ACTION_BOOT_COMPLETED.equals(intent.getAction())) return;
        if (!ServerService.isBootStartEnabled(context)) return;
        Intent service = new Intent(context, ServerService.class).setAction(ServerService.ACTION_START);
        try {
            if (Build.VERSION.SDK_INT >= 26) context.startForegroundService(service);
            else context.startService(service);
        } catch (IllegalStateException | SecurityException e) {
            Log.w(TAG, "Boot server start was blocked by the system");
        }

        boolean mayLaunchActivity = Build.VERSION.SDK_INT < 29 || Settings.canDrawOverlays(context);
        if (!mayLaunchActivity) return;
        try {
            Intent launch = context.getPackageManager().getLaunchIntentForPackage(context.getPackageName());
            if (launch != null) {
                launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                context.startActivity(launch);
            }
        } catch (IllegalStateException | SecurityException e) {
            Log.w(TAG, "Boot activity launch was blocked by the system");
        }
    }
}
