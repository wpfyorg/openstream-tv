package org.opebstreamstv.android;

import android.app.*;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.File;
import java.io.InputStreamReader;
import java.util.Map;

public final class ServerService extends Service {
    private static final String TAG = "OpenstreamServer";
    static final String ACTION_START = "org.opebstreamstv.android.START";
    static final String ACTION_STOP = "org.opebstreamstv.android.STOP";
    static final String ACTION_STOPPED = "org.opebstreamstv.android.STOPPED";
    static final String ACTION_EXITED = "org.opebstreamstv.android.EXITED";
    private static final String PREFS = "server_autostart";
    private static final String PREF_ENABLED = "enabled";
    private Process process;

    @Override public void onCreate() { super.onCreate(); createChannel(); }

    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && ACTION_STOP.equals(intent.getAction())) {
            stopServer(); stopForeground(true); sendBroadcast(new Intent(ACTION_STOPPED).setPackage(getPackageName())); stopSelf(); return START_NOT_STICKY;
        }
        startForeground(41, notification("Starting local server…"));
        if (process == null || !process.isAlive()) startServer();
        return START_STICKY;
    }

    private void startServer() {
        try {
            File dir = getFilesDir();
            File config = new File(dir, "config.toml");
            if (!config.exists() && !config.createNewFile()) throw new IOException("Cannot initialize config");
            File nativeServer = new File(getApplicationInfo().nativeLibraryDir, "libopenstream_exec.so");
            int port = LocalServerEndpoint.choosePort(this);
            ProcessBuilder pb = new ProcessBuilder(nativeServer.getAbsolutePath(), "--skip-update-check", "--config", config.getAbsolutePath(), "serve", "--host", "127.0.0.1", "--port", Integer.toString(port));
            pb.directory(dir);
            pb.redirectOutput(new File("/dev/null"));
            Map<String, String> env = pb.environment();
            env.put("OPENSTREAM_PATH_PREFIX", new File(getFilesDir(), "data").getAbsolutePath());
            env.put("HOME", getFilesDir().getAbsolutePath());
            env.put("OPENSTREAM_FASTWAY", "true");
            env.put("OPENSTREAM_TVPLUS", "true");
            env.put("OPENSTREAM_PLAYBOX", "true");
            env.put("OPENSTREAM_VZY", "true");
            env.put("OPENSTREAM_TATAPLAY", "true");
            // Only the companion app forces EPG on; Termux uses the same Android binary target.
            env.put("OPENSTREAM_COMPANION_APP", "1");
            env.put("OPENSTREAM_EPG", "true");
            process = pb.start();
            Process started = process;
            new Thread(() -> {
                try (BufferedReader stderr = new BufferedReader(new InputStreamReader(started.getErrorStream()))) {
                    String line;
                    while ((line = stderr.readLine()) != null) {
                        String lower = line.toLowerCase(java.util.Locale.ROOT);
                        String category = lower.contains("parsing config") ? "config parse"
                                : lower.contains("permission denied") ? "filesystem permission"
                                : lower.contains("address already in use") ? "port already occupied"
                                : lower.contains("no such file") ? "missing file"
                                : lower.contains("not supported") ? "unsupported operation"
                                : lower.contains("failed to") ? "operation failure"
                                : lower.contains("error") ? "startup error" : null;
                        if (category != null) Log.e(TAG, "Native stderr category: " + category);
                    }
                } catch (IOException ignored) {
                    Log.e(TAG, "Could not read native stderr");
                }
            }, "server-stderr-monitor").start();
            new Thread(() -> {
                try {
                    int exitCode = started.waitFor();
                    new android.os.Handler(getMainLooper()).post(() -> {
                        if (process != started) return;
                        Log.e(TAG, "Native server exited with code " + exitCode);
                        process = null;
                        stopForeground(true);
                        sendBroadcast(new Intent(ACTION_EXITED).setPackage(getPackageName()));
                        stopSelf();
                    });
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                }
            }, "server-exit-monitor").start();
            updateNotification("Local server: 127.0.0.1:" + port);
        } catch (Exception e) {
            Log.e(TAG, "Native server launch failed: " + e.getClass().getSimpleName());
            stopForeground(true);
            sendBroadcast(new Intent(ACTION_EXITED).setPackage(getPackageName()));
            stopSelf();
        }
    }

    static boolean isBootStartEnabled(Context context) {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(PREF_ENABLED, true);
    }

    static boolean setBootStartEnabled(Context context, boolean enabled) {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putBoolean(PREF_ENABLED, enabled).commit();
    }

    private void stopServer() {
        if (process != null) { process.destroy(); try { if (!process.waitFor(3, java.util.concurrent.TimeUnit.SECONDS)) process.destroyForcibly(); } catch (InterruptedException e) { Thread.currentThread().interrupt(); process.destroyForcibly(); } process = null; }
    }
    private void createChannel() {
        if (Build.VERSION.SDK_INT >= 26) { NotificationChannel c = new NotificationChannel("jiotv_server", "JioTV local server", NotificationManager.IMPORTANCE_LOW); getSystemService(NotificationManager.class).createNotificationChannel(c); }
    }
    private Notification notification(String text) {
        PendingIntent open = PendingIntent.getActivity(this, 0, new Intent(this, MainActivity.class), PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 23 ? PendingIntent.FLAG_IMMUTABLE : 0));
        Notification.Builder builder = Build.VERSION.SDK_INT >= 26 ? new Notification.Builder(this, "jiotv_server") : new Notification.Builder(this);
        return builder
                .setContentTitle("Openstream TV Slim").setContentText(text).setSmallIcon(android.R.drawable.stat_sys_download_done).setContentIntent(open).setOngoing(true).build();
    }
    private void updateNotification(String text) { getSystemService(NotificationManager.class).notify(41, notification(text)); }
    @Override public void onDestroy() { stopServer(); super.onDestroy(); }
    @Override public IBinder onBind(Intent intent) { return null; }
}
