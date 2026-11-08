package org.jiotvgo.android;

import android.app.Activity;
import android.content.ComponentName;
import android.content.Intent;
import android.content.ServiceConnection;
import android.content.pm.PackageManager;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.provider.Settings;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

import rikka.shizuku.Shizuku;

/** One-shot Shizuku setup for the startup-screen overlay permission. */
final class ShizukuSetup {
    interface Feedback { void show(String message); }

    private static final int SHIZUKU_PERMISSION_REQUEST = 250;
    private final Activity activity;
    private final Feedback feedback;
    private final Handler main = new Handler(Looper.getMainLooper());
    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private Shizuku.UserServiceArgs serviceArgs;
    private ServiceConnection connection;
    private boolean pending;
    private boolean closed;
    private final Shizuku.OnRequestPermissionResultListener permissionListener = (request, result) -> {
        if (request != SHIZUKU_PERMISSION_REQUEST) return;
        main.post(() -> {
            if (closed || !pending) return;
            if (result == PackageManager.PERMISSION_GRANTED) bindService();
            else finish("Shizuku permission denied. Allow Openstream TV Slim in Shizuku and try again.");
        });
    };

    ShizukuSetup(Activity activity, Feedback feedback) {
        this.activity = activity;
        this.feedback = feedback;
        Shizuku.addRequestPermissionResultListener(permissionListener);
    }

    void setup() {
        if (closed || pending) return;
        if (android.os.Build.VERSION.SDK_INT >= 23 && Settings.canDrawOverlays(activity)) {
            feedback.show("Startup screen permission is already enabled.");
            return;
        }
        try {
            if (!Shizuku.pingBinder()) {
                openShizuku();
                return;
            }
            if (Shizuku.isPreV11()) {
                feedback.show("Update Shizuku to version 11 or newer to use setup.");
                return;
            }
            pending = true;
            if (Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED) bindService();
            else if (Shizuku.shouldShowRequestPermissionRationale()) {
                finish("Allow Openstream TV Slim in Shizuku's authorized apps, then try again.");
            } else {
                feedback.show("Approve Openstream TV Slim in the Shizuku permission dialog.");
                Shizuku.requestPermission(SHIZUKU_PERMISSION_REQUEST);
            }
        } catch (RuntimeException e) {
            finish("Shizuku is unavailable. Start its service and try again.");
        }
    }

    private void openShizuku() {
        Intent launch = activity.getPackageManager().getLaunchIntentForPackage("moe.shizuku.privileged.api");
        if (launch == null) {
            feedback.show("Install Shizuku and start its service before setup.");
            return;
        }
        feedback.show("Start the Shizuku service, then return and select Setup using Shizuku.");
        try { activity.startActivity(launch); }
        catch (RuntimeException e) { feedback.show("Open Shizuku and start its service before setup."); }
    }

    private void bindService() {
        if (!pending || closed) return;
        feedback.show("Applying startup screen permission through Shizuku…");
        serviceArgs = new Shizuku.UserServiceArgs(new ComponentName(activity, OverlayPermissionService.class))
                .tag("overlay_permission").processNameSuffix("overlay_permission")
                .version(1).daemon(false);
        final Shizuku.UserServiceArgs args = serviceArgs;
        connection = new ServiceConnection() {
            @Override public void onServiceConnected(ComponentName name, IBinder binder) {
                IOverlayPermissionService remote = IOverlayPermissionService.Stub.asInterface(binder);
                worker.execute(() -> {
                    boolean applied;
                    try { applied = remote.grantOverlayPermission(); }
                    catch (Exception e) { applied = false; }
                    final boolean commandSucceeded = applied;
                    main.post(() -> {
                        if (closed || !pending || connection != this) return;
                        boolean enabled = android.os.Build.VERSION.SDK_INT < 23 || Settings.canDrawOverlays(activity);
                        finish(commandSucceeded && enabled
                                ? "Shizuku setup complete. Startup screen permission enabled."
                                : "Shizuku could not enable the startup screen permission. Check the device policy.");
                    });
                });
            }
            @Override public void onServiceDisconnected(ComponentName name) {
                if (connection == this && pending) finish("Shizuku disconnected. Start its service and try again.");
            }
        };
        try {
            Shizuku.bindUserService(args, connection);
            main.postDelayed(() -> {
                if (pending && !closed && serviceArgs == args) finish("Shizuku connection timed out. Try again.");
            }, 15000);
        } catch (RuntimeException e) {
            finish("Could not connect to Shizuku. Start its service and try again.");
        }
    }

    private void finish(String message) {
        if (closed) return;
        pending = false;
        unbind();
        feedback.show(message);
    }

    private void unbind() {
        if (serviceArgs == null) return;
        try { Shizuku.unbindUserService(serviceArgs, connection, true); }
        catch (RuntimeException ignored) { }
        serviceArgs = null;
        connection = null;
    }

    void close() {
        closed = true;
        Shizuku.removeRequestPermissionResultListener(permissionListener);
        unbind();
        worker.shutdownNow();
    }
}
