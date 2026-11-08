package org.jiotvgo.android;

import android.app.*;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.IBinder;
import java.io.IOException;
import java.io.File;
import java.util.Map;

public final class ServerService extends Service {
    static final String ACTION_START = "org.jiotvgo.android.START";
    static final String ACTION_STOP = "org.jiotvgo.android.STOP";
    static final String ACTION_STOPPED = "org.jiotvgo.android.STOPPED";
    private static final String PREFS = "server_autostart";
    private static final String PREF_ENABLED = "enabled";
    private static final String PREF_TV_PLUS = "tv_plus";
    private Process process;

    @Override public void onCreate() { super.onCreate(); createChannel(); }

    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && ACTION_STOP.equals(intent.getAction())) {
            stopServer(); stopForeground(true); sendBroadcast(new Intent(ACTION_STOPPED).setPackage(getPackageName())); stopSelf(); return START_NOT_STICKY;
        }
        if (intent == null && !isBootStartEnabled(this)) { stopSelf(startId); return START_NOT_STICKY; }
        startForeground(41, notification("Starting local server…"));
        if (process == null || !process.isAlive()) startServer();
        return START_STICKY;
    }

    private void startServer() {
        try {
            File dir = getFilesDir();
            File config = new File(dir, "config.toml");
            if (!config.exists() && !config.createNewFile()) throw new IOException("Cannot initialize config");
            ProcessBuilder pb = new ProcessBuilder(new File(getApplicationInfo().nativeLibraryDir, "libjiotv_exec.so").getAbsolutePath(), "--skip-update-check", "--config", config.getAbsolutePath(), "serve", "--host", "127.0.0.1", "--port", "5001");
            pb.directory(dir);
            pb.redirectOutput(new File("/dev/null"));
            pb.redirectError(new File("/dev/null"));
            Map<String, String> env = pb.environment();
            env.put("JIOTV_PATH_PREFIX", new File(getFilesDir(), "data").getAbsolutePath());
            env.put("HOME", getFilesDir().getAbsolutePath());
            env.put("JIOTV_FASTWAY", "false");
            env.put("JIOTV_TVPLUS", Boolean.toString(isTvPlusEnabled(this)));
            process = pb.start();
            updateNotification("Local server: 127.0.0.1:5001");
        } catch (Exception e) {
            updateNotification("Could not start server. Open Openstream TV Slim for details.");
            stopSelf();
        }
    }

    static boolean isBootStartEnabled(Context context) {
        return context.getSharedPreferences(PREFS, MODE_PRIVATE).getBoolean(PREF_ENABLED, true);
    }

    static boolean isTvPlusEnabled(Context context) {
        return context.getSharedPreferences(PREFS, MODE_PRIVATE).getBoolean(PREF_TV_PLUS, false);
    }

    static void setTvPlusEnabled(Context context, boolean enabled) {
        context.getSharedPreferences(PREFS, MODE_PRIVATE).edit().putBoolean(PREF_TV_PLUS, enabled).apply();
    }

    static boolean setBootStartEnabled(Context context, boolean enabled) {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
                .edit().putBoolean(PREF_ENABLED, enabled).commit();
    }

    private void stopServer() {
        if (process != null) { process.destroy(); try { if (!process.waitFor(3, java.util.concurrent.TimeUnit.SECONDS)) process.destroyForcibly(); } catch (InterruptedException e) { Thread.currentThread().interrupt(); process.destroyForcibly(); } process = null; }
    }
    private void createChannel() {
        if (Build.VERSION.SDK_INT >= 26) { NotificationChannel c = new NotificationChannel("jiotv_server", "Openstream local server", NotificationManager.IMPORTANCE_LOW); getSystemService(NotificationManager.class).createNotificationChannel(c); }
    }
    private Notification notification(String text) {
        PendingIntent open = PendingIntent.getActivity(this, 0, new Intent(this, MainActivity.class), PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 23 ? PendingIntent.FLAG_IMMUTABLE : 0));
        return new Notification.Builder(this, Build.VERSION.SDK_INT >= 26 ? "jiotv_server" : "")
                .setContentTitle("Openstream TV Slim").setContentText(text).setSmallIcon(android.R.drawable.stat_sys_download_done).setContentIntent(open).setOngoing(true).build();
    }
    private void updateNotification(String text) { getSystemService(NotificationManager.class).notify(41, notification(text)); }
    @Override public void onDestroy() { stopServer(); super.onDestroy(); }
    @Override public IBinder onBind(Intent intent) { return null; }
}
