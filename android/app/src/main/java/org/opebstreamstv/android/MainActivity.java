package org.opebstreamstv.android;

import android.Manifest;
import android.app.*;
import android.content.*;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.os.DeadObjectException;
import android.os.Handler;
import android.os.Looper;
import android.os.RemoteException;
import android.content.res.Configuration;
import android.content.BroadcastReceiver;
import android.content.IntentFilter;
import android.system.Os;
import android.view.*;
import android.view.inputmethod.InputMethodManager;
import android.text.Editable;
import android.text.InputFilter;
import android.text.InputType;
import android.text.TextWatcher;
import android.util.TypedValue;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.StateListDrawable;
import android.graphics.Typeface;
import android.graphics.Color;
import android.widget.*;
import org.json.JSONObject;
import java.io.*;
import java.net.*;
import java.net.ConnectException;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;
import rikka.shizuku.Shizuku;
import android.content.ServiceConnection;
import android.os.IBinder;
import android.os.Parcel;
import android.content.ComponentName;
import android.content.pm.PackageManager;
import io.github.muntashirakon.adb.AdbPairingRequiredException;
import java.net.SocketTimeoutException;

public final class MainActivity extends Activity {
    private static final int PICK_CONFIG = 21;
    private static final int PICK_STORE = 22;
    private static final int CREATE_CONFIG = 23;
    private static final int STORAGE_PERMISSION = 24;
    private static final int CREATE_PLAYLIST = 25;
    private final ExecutorService io = Executors.newSingleThreadExecutor();
    private EditText number, otp;
    private TextView status, playlist;
    private Button send, verify, service, bootPermission, adbSetupButton, shizukuButton;
    private Future<?> setupTask;
    private AdbSetupSession adbSetupSession;
    private static final long ADB_PAIRING_TIMEOUT_MS = 45_000L;
    private static final int ADB_PERMISSION_VERIFY_ATTEMPTS = 10;
    private static final long ADB_PERMISSION_VERIFY_INTERVAL_MS = 250L;
    private static final String LOCAL_ADB_HOST = "127.0.0.1";
    private static final int LOCAL_ADB_PORT = 5555;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private final Handler serverPollHandler = new Handler(Looper.getMainLooper());
    private final Object serverPollLock = new Object();
    private volatile boolean activityDestroyed;
    private volatile IAdbPairingService pairingService;
    private volatile ServiceConnection pairingConnection;
    private volatile boolean pairingBound;
    private static final int SHIZUKU_PERMISSION = 7812;
    private ServiceConnection shizukuConnection;
    private volatile AtomicBoolean shizukuTeardownExpected;
    private final Shizuku.OnBinderDeadListener shizukuBinderDeadListener = () -> {
        cancelAdbSetup();
        say("Shizuku stopped. Setup was interrupted; restart Shizuku and retry, or use ADB / Settings.");
    };
    private Shizuku.UserServiceArgs shizukuArgs;

    private static final class AdbSetupSession {
        private AppAdbManager manager;
        private boolean cancelled;

        synchronized boolean install(AppAdbManager activeManager) {
            if (cancelled) return false;
            manager = activeManager;
            return true;
        }

        synchronized boolean isCancelled() { return cancelled; }

        void cancel() {
            AppAdbManager activeManager;
            synchronized (this) {
                cancelled = true;
                activeManager = manager;
                manager = null;
            }
            close(activeManager);
        }

        void close() {
            AppAdbManager activeManager;
            synchronized (this) {
                activeManager = manager;
                manager = null;
            }
            close(activeManager);
        }

        private static void close(AppAdbManager activeManager) {
            if (activeManager != null) try { activeManager.close(); } catch (Exception ignored) { }
        }
    }

    private void cancelAdbSetup() {
        AdbSetupSession session = adbSetupSession;
        if (session != null) session.cancel();
        Future<?> task = setupTask;
        if (task != null) task.cancel(true);
    }
    private Button copyButton, saveM3uButton, openButton, importConfigButton, exportConfigButton, importStoreButton;
    private Button disableAutostartButton, logoutButton;
    private TextView tvServerBadge, tvLoginHeading, tvLoginHelper, tvOtpLabel, tvAccountName, tvAccountNote;
    private TextView tvAccountRetentionNote;
    private TextView tvProviderHeading, tvProviderHelper, tvMobileLabel, tvAutostartLabel, tvOtpHint;
    private View tvStartupCard, tvAccountCard;
    private FrameLayout tvCanvas;
    private final java.util.List<View> tvLeftControls = new java.util.ArrayList<>();
    private final java.util.List<View> tvRightControls = new java.util.ArrayList<>();
    private final java.util.List<View> tvPlaylistViews = new java.util.ArrayList<>();
    private final java.util.List<View> tvConfigViews = new java.util.ArrayList<>();
    private final java.util.Map<View,Integer> tvBaseY = new java.util.IdentityHashMap<>();
    private final java.util.Map<View,View[]> tvFocusGraph = new java.util.IdentityHashMap<>();
    private float tvScale = 1f;
    private boolean tvOtpRequested, tvAuthenticated, tvLoginBusy;
    private boolean tvAutoStartEnabled, tvServerStarted, tvLogoutDisabled;
    private String authenticatedProvider = "";
    private final Handler healthHandler = new Handler(Looper.getMainLooper());
    private final Runnable healthRefresh = new Runnable() {
        @Override public void run() {
            if (activityDestroyed || !television || io.isShutdown()) return;
            io.execute(() -> refreshServerAndAccount());
            healthHandler.postDelayed(this, 4000);
        }
    };
    private String playlistUrl = "", keyedPrefix = "";
    private boolean television;
    private RadioButton jiotvProvider, tvPlusProvider;
    private ScrollView tvScroll;
    private View pendingTvReturnFocus;
    private EditText activeTvEditField;
    private int pendingLocalPickerRequest;
    private final BroadcastReceiver serverExitedReceiver = new BroadcastReceiver() {
        @Override public void onReceive(Context context, Intent intent) {
            service.setText("Start local server");
            if (television) updateServerBadge(false);
            say("Server stopped unexpectedly. Start it again.");
        }
    };

    @Override public void onCreate(Bundle b) {
        super.onCreate(b);
        render();
        IntentFilter exitFilter = new IntentFilter(ServerService.ACTION_EXITED);
        if (Build.VERSION.SDK_INT >= 33) registerReceiver(serverExitedReceiver, exitFilter, Context.RECEIVER_NOT_EXPORTED);
        else registerReceiver(serverExitedReceiver, exitFilter);
        Shizuku.addRequestPermissionResultListener(shizukuPermissionListener);
        Shizuku.addBinderDeadListener(shizukuBinderDeadListener);
        service.setText(isServiceRunning() ? "Stop local server" : "Start local server");
        if (television) {
            service.post(service::requestFocus);
            healthHandler.post(healthRefresh);
        }
        refreshUrl();
    }
    @Override protected void onResume() {
        super.onResume();
        refreshPermissionButtons();
        restoreTvReturnFocus();
        if (television) io.execute(() -> refreshServerAndAccount());
    }
    @Override public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (!hasFocus || !television || pendingTvReturnFocus != null || activeTvEditField != null) return;
        if (getWindow().getDecorView().findFocus() == null) requestTvFocus(service);
    }
    private void render() {
        television = (getResources().getConfiguration().uiMode & Configuration.UI_MODE_TYPE_MASK) == Configuration.UI_MODE_TYPE_TELEVISION;
        if (television) { renderTv(); return; }
        int inset = dp(television ? 32 : 24);
        ScrollView scroll = new ScrollView(this); scroll.setFillViewport(true); scroll.setDescendantFocusability(ViewGroup.FOCUS_AFTER_DESCENDANTS); scroll.setSmoothScrollingEnabled(true); LinearLayout root = new LinearLayout(this); root.setOrientation(LinearLayout.VERTICAL); root.setPadding(inset, dp(television ? 16 : 20), inset, dp(television ? 16 : 20)); root.setFocusableInTouchMode(!television); scroll.addView(root); scroll.setBackgroundColor(0xff101827); if (television) { root.setFocusable(false); scroll.setFocusable(false); }
        addText(root, "Openstream TV Slim", television ? 28 : 28, 0xff43d9b0); status = addText(root, "Starting…", television ? 15 : 15, 0xffc5d0dd);
        LinearLayout columns = new LinearLayout(this);
        columns.setOrientation(television ? LinearLayout.HORIZONTAL : LinearLayout.VERTICAL);
        root.addView(columns, spacedParams(0, 0));
        LinearLayout left = column(); LinearLayout right = column();
        if (television) {
            LinearLayout.LayoutParams side = new LinearLayout.LayoutParams(0, -2, 1);
            side.rightMargin = dp(12); columns.addView(left, side);
            side = new LinearLayout.LayoutParams(0, -2, 1); side.leftMargin = dp(12); columns.addView(right, side);
        } else { columns.addView(left); columns.addView(right); }

        addText(left, "Server and provider", television ? 19 : 20, 0xffffffff);
        service = button(left, "Start local server"); service.setOnClickListener(v -> toggleService());
        bootPermission = button(left, "Enable startup screen (Android 10+)"); bootPermission.setOnClickListener(v -> requestBootPermission());
        adbSetupButton = button(left, "Setup startup permissions using ADB"); adbSetupButton.setOnClickListener(v -> runLocalAdbSetup());
        shizukuButton = button(left, "Setup using Shizuku"); shizukuButton.setOnClickListener(v -> requestShizukuGrant());
        disableAutostartButton = button(left, "Disable auto-start");
        disableAutostartButton.setOnClickListener(v -> {
            if (ServerService.setBootStartEnabled(this, false)) {
                say("Auto-start disabled. The current server keeps running; Android's overlay permission remains granted.");
                refreshPermissionButtons();
            } else say("Could not save the auto-start setting.");
        });
        RadioGroup providers = new RadioGroup(this); providers.setOrientation(RadioGroup.HORIZONTAL); left.addView(providers, spacedParams(0, 0));
        jiotvProvider = providerChoice("JioTV", true); tvPlusProvider = providerChoice("JioTV+", false);
        providers.addView(jiotvProvider); providers.addView(tvPlusProvider);
        addText(left, "Choose which account receives the OTP. JioTV+ needs an eligible JioFiber/AirFiber account.", television ? 13 : 13, 0xff91a0b3);
        addText(left, "Playlist URL", television ? 18 : 20, 0xffffffff);
        playlist = addText(left, "Start the server to get your keyed link.", television ? 14 : 14, 0xffc5d0dd);
        LinearLayout actions = new LinearLayout(this); actions.setOrientation(LinearLayout.HORIZONTAL); left.addView(actions, spacedParams(0, 0));
        copyButton = button(actions, "Copy"); copyButton.setOnClickListener(v -> copyUrl());
        saveM3uButton = button(actions, "Save M3U"); saveM3uButton.setOnClickListener(v -> createPlaylistDocument());
        openButton = button(left, "Open in IPTV app"); openButton.setOnClickListener(v -> openUrl());

        addText(right, "Provider login", television ? 19 : 20, 0xffffffff);
        number = field(right, "Mobile number"); number.setInputType(3); otp = field(right, "OTP"); otp.setInputType(2);
        LinearLayout loginActions = new LinearLayout(this); loginActions.setOrientation(LinearLayout.HORIZONTAL); right.addView(loginActions, spacedParams(0, 0));
        send = button(loginActions, "Send OTP"); send.setOnClickListener(v -> otpRequest(false));
        verify = button(loginActions, "Verify OTP"); verify.setOnClickListener(v -> otpRequest(true));
        addText(right, "Import", television ? 18 : 20, 0xffffffff);
        importConfigButton = button(right, "Import TOML config"); importConfigButton.setOnClickListener(v -> pickFile("application/toml", PICK_CONFIG));
        exportConfigButton = button(right, "Export TOML config"); exportConfigButton.setOnClickListener(v -> createConfigDocument());
        importStoreButton = button(right, "Import login store backup"); importStoreButton.setOnClickListener(v -> pickFile("application/toml", PICK_STORE));
        setContentView(scroll);
        if (television) configureTvFocus(scroll);
        refreshPermissionButtons();
    }
    // All five Pen frames describe states of this single TV Activity. Positions are
    // specified on a 1920px design canvas and scaled to the current display.
    private static final int TV_BG = 0xff0b1220, TV_PANEL = 0xff152234;
    private static final int TV_ACCENT = 0xff42d7ae, TV_TEXT = 0xffeaf2f5;
    private static final int TV_MUTED = 0xff93a6b7, TV_BORDER = 0xff34485b;

    private int tvPx(int pixels) { return Math.round(pixels * tvScale); }
    private GradientDrawable tvSurface(int fill, int border, int radius) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(fill);
        drawable.setCornerRadius(tvPx(radius));
        if (border != 0) drawable.setStroke(Math.max(1, tvPx(2)), border);
        return drawable;
    }
    private StateListDrawable tvFocusSurface(int fill) {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{android.R.attr.state_focused}, tvSurface(TV_ACCENT, TV_ACCENT, 12));
        states.addState(new int[]{android.R.attr.state_pressed}, tvSurface(TV_ACCENT, TV_ACCENT, 12));
        states.addState(new int[]{}, tvSurface(fill, 0, 12));
        return states;
    }
    private StateListDrawable tvOutlinedFocusSurface(int fill) {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{android.R.attr.state_focused}, tvSurface(0xff203b40, TV_ACCENT, 12));
        states.addState(new int[]{android.R.attr.state_pressed}, tvSurface(0xff203b40, TV_ACCENT, 12));
        states.addState(new int[]{}, tvSurface(fill, 0, 12));
        return states;
    }
    private StateListDrawable tvBorderedActionSurface(int fill) {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{-android.R.attr.state_enabled}, tvSurface(fill, TV_BORDER, 12));
        states.addState(new int[]{android.R.attr.state_focused}, tvSurface(TV_ACCENT, 0xffd2fff1, 12));
        states.addState(new int[]{android.R.attr.state_pressed}, tvSurface(TV_ACCENT, 0xffd2fff1, 12));
        states.addState(new int[]{}, tvSurface(fill, TV_BORDER, 12));
        return states;
    }
    private StateListDrawable tvPrimaryActionSurface() {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{-android.R.attr.state_enabled}, tvSurface(0xff355b58, TV_BORDER, 12));
        states.addState(new int[]{android.R.attr.state_focused}, tvSurface(0xffa7f8de, 0xffd2fff1, 12));
        states.addState(new int[]{android.R.attr.state_pressed}, tvSurface(0xff8cebd3, 0xffd2fff1, 12));
        states.addState(new int[]{}, tvSurface(TV_ACCENT, 0xffd2fff1, 12));
        return states;
    }
    private <T extends View> T tvPlace(T view, int x, int y, int width, int height) {
        FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(tvPx(width), tvPx(height));
        lp.leftMargin = tvPx(x); lp.topMargin = tvPx(y);
        tvCanvas.addView(view, lp);
        tvBaseY.put(view, y);
        return view;
    }
    private TextView tvText(String value, int size, int color, boolean bold) {
        TextView view = new TextView(this);
        view.setText(value); view.setTextColor(color);
        view.setTextSize(TypedValue.COMPLEX_UNIT_PX, tvPx(size));
        view.setGravity(Gravity.CENTER_VERTICAL);
        if (bold) view.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        return view;
    }
    private TextView tvLabel(String value, int x, int y, int width, int height, int size, int color, boolean bold) {
        return tvPlace(tvText(value, size, color, bold), x, y, width, height);
    }
    private View tvCard(int x, int y, int width, int height) {
        View view = new View(this); view.setBackground(tvSurface(TV_PANEL, TV_BORDER, 22));
        return tvPlace(view, x, y, width, height);
    }
    private Button tvAction(String label, int x, int y, int width, int height, boolean right) {
        Button action = new Button(this);
        action.setId(View.generateViewId()); action.setText(label); action.setAllCaps(false);
        action.setTextSize(TypedValue.COMPLEX_UNIT_PX, tvPx(23));
        action.setTextColor(new android.content.res.ColorStateList(
                new int[][]{{android.R.attr.state_focused}, {android.R.attr.state_pressed}, {}},
                new int[]{0xff071e1b, 0xff071e1b, TV_TEXT}));
        action.setBackground(tvFocusSurface(0xff202d3e));
        action.setPadding(tvPx(22), 0, tvPx(18), 0);
        action.setGravity(Gravity.CENTER_VERTICAL | Gravity.START);
        action.setFocusable(true); action.setFocusableInTouchMode(false);
        tvPlace(action, x, y, width, height);
        (right ? tvRightControls : tvLeftControls).add(action);
        configureTvFocusable(action);
        return action;
    }
    private EditText tvInput(String hint, int x, int y, int width) {
        EditText field = new EditText(this);
        field.setId(View.generateViewId()); field.setSingleLine(true);
        field.setHint(hint); field.setTextColor(TV_TEXT); field.setHintTextColor(TV_MUTED);
        field.setTextSize(TypedValue.COMPLEX_UNIT_PX, tvPx(23));
        field.setPadding(tvPx(20), 0, tvPx(20), 0);
        field.setBackground(tvOutlinedFocusSurface(0xff0e1b2b));
        tvPlace(field, x, y, width, 64);
        tvRightControls.add(field);
        return field;
    }
    private void renderTv() {
        tvScale = Math.min(1.3f, getResources().getDisplayMetrics().widthPixels / 1920f);
        tvScroll = new ScrollView(this);
        tvScroll.setFillViewport(true); tvScroll.setSmoothScrollingEnabled(true);
        tvScroll.setBackgroundColor(TV_BG); tvScroll.setFocusable(false);
        tvCanvas = new FrameLayout(this); tvCanvas.setFocusable(false);
        tvScroll.addView(tvCanvas, new ScrollView.LayoutParams(tvPx(1920), tvPx(1080)));
        View brand = new View(this);
        brand.setBackground(tvSurface(TV_ACCENT, 0, 12));
        tvPlace(brand, 70, 54, 52, 52);
        tvLabel("▶", 84, 61, 38, 42, 28, 0xff092820, true);
        tvLabel("Openstream TV Slim", 140, 50, 950, 51, 38, TV_TEXT, true);
        status = tvLabel("Local TV server · Ready", 141, 101, 1300, 32, 20, TV_MUTED, false);
        View line = new View(this); line.setBackgroundColor(TV_BORDER);
        tvPlace(line, 70, 145, 1780, 2);

        tvServerBadge = tvLabel("●  Server stopped", 1515, 61, 325, 50, 23, 0xffffa3a8, true);
        tvServerBadge.setGravity(Gravity.CENTER);
        tvServerBadge.setBackground(tvSurface(0xff3b2029, 0, 14));
        tvLabel("Local server", 70, 180, 800, 38, 32, TV_TEXT, true);
        tvLabel("Start the local server to use playlists, EPG and account login", 70, 222, 840, 31, 20, TV_MUTED, false);
        service = tvAction("Start local server", 70, 270, 840, 68, false);
        service.setOnClickListener(v -> toggleService());

        tvStartupCard = tvCard(70, 352, 840, 104);
        tvStartupCard.setBackground(tvSurface(0xff182b33, 0, 14));
        tvAutostartLabel = tvLabel("Auto-start enabled", 100, 388, 490, 48, 26, 0xff9ae8c9, true);
        bootPermission = tvAction("Enable startup screen (Android 10+)", 70, 428, 840, 62, false);
        bootPermission.setOnClickListener(v -> requestBootPermission());
        adbSetupButton = tvAction("Set up using ADB", 70, 352, 410, 62, false);
        adbSetupButton.setOnClickListener(v -> runLocalAdbSetup());
        shizukuButton = tvAction("Set up using Shizuku", 500, 352, 410, 62, false);
        shizukuButton.setOnClickListener(v -> requestShizukuGrant());
        disableAutostartButton = tvAction("Disable auto-start", 630, 371, 245, 65, false);
        disableAutostartButton.setGravity(Gravity.CENTER);
        disableAutostartButton.setBackground(tvBorderedActionSurface(0xff202d3e));
        disableAutostartButton.setOnClickListener(v -> {
            if (ServerService.setBootStartEnabled(this, false)) {
                say("Auto-start disabled. The current server keeps running; Android's overlay permission remains granted.");
                refreshPermissionButtons();
            } else say("Could not save the auto-start setting.");
        });

        tvPlaylistViews.add(tvLabel("Playlist", 70, 530, 800, 40, 32, TV_TEXT, true));
        playlist = tvLabel("Start the server to get your playlist.", 70, 572, 840, 60, 20, TV_MUTED, false);
        playlist.setPadding(tvPx(20), 0, tvPx(20), 0);
        playlist.setBackground(tvSurface(0xff131f2e, 0, 12));
        playlist.setSingleLine(true); playlist.setEllipsize(android.text.TextUtils.TruncateAt.MIDDLE);
        tvPlaylistViews.add(playlist);
        copyButton = tvAction("Copy", 70, 647, 267, 60, false); copyButton.setOnClickListener(v -> copyUrl());
        saveM3uButton = tvAction("Save M3U", 353, 647, 267, 60, false); saveM3uButton.setOnClickListener(v -> createPlaylistDocument());
        openButton = tvAction("Open in IPTV app", 636, 647, 274, 60, false); openButton.setOnClickListener(v -> openUrl());
        tvPlaylistViews.add(copyButton); tvPlaylistViews.add(saveM3uButton); tvPlaylistViews.add(openButton);

        tvConfigViews.add(tvLabel("Configuration & backup", 70, 757, 810, 39, 32, TV_TEXT, true));
        tvConfigViews.add(tvLabel("Import or export configuration files on this device", 70, 796, 840, 33, 20, TV_MUTED, false));
        importConfigButton = tvAction("Import TOML config", 70, 835, 410, 62, false);
        exportConfigButton = tvAction("Export TOML config", 500, 835, 410, 62, false);
        importStoreButton = tvAction("Import login store backup", 70, 913, 840, 62, false);
        importConfigButton.setOnClickListener(v -> pickFile("application/toml", PICK_CONFIG));
        exportConfigButton.setOnClickListener(v -> createConfigDocument());
        importStoreButton.setOnClickListener(v -> pickFile("application/toml", PICK_STORE));
        tvConfigViews.add(importConfigButton); tvConfigViews.add(exportConfigButton); tvConfigViews.add(importStoreButton);

        View divider = new View(this); divider.setBackgroundColor(TV_BORDER);
        tvPlace(divider, 958, 181, 2, 755);
        tvProviderHeading = tvLabel("Provider", 1010, 180, 680, 38, 32, TV_TEXT, true);
        tvProviderHelper = tvLabel("Select the account you want to use", 1010, 220, 825, 32, 20, TV_MUTED, false);
        RadioGroup providers = new RadioGroup(this); providers.setOrientation(RadioGroup.HORIZONTAL);
        tvPlace(providers, 1010, 258, 840, 62);
        jiotvProvider = providerChoice("JioTV", true); tvPlusProvider = providerChoice("JioTV+", false);
        for (RadioButton choice : new RadioButton[]{jiotvProvider, tvPlusProvider}) {
            choice.setTextSize(TypedValue.COMPLEX_UNIT_PX, tvPx(23));
            choice.setBackground(tvOutlinedFocusSurface(0xff1b2b3b));
            choice.setPadding(tvPx(18), 0, tvPx(10), 0);
            RadioGroup.LayoutParams option = new RadioGroup.LayoutParams(tvPx(410), tvPx(62));
            if (choice == tvPlusProvider) option.leftMargin = tvPx(20);
            providers.addView(choice, option);
            tvRightControls.add(choice);
        }
        boolean plus = getSharedPreferences("tv_provider", MODE_PRIVATE).getBoolean("plus", false);
        (plus ? tvPlusProvider : jiotvProvider).setChecked(true);
        providers.setOnCheckedChangeListener((g, checkedId) -> {
            boolean selectedPlus = checkedId == tvPlusProvider.getId();
            getSharedPreferences("tv_provider", MODE_PRIVATE).edit().putBoolean("plus", selectedPlus).apply();
            tvOtpRequested = false; tvAuthenticated = false;
            authenticatedProvider = selectedPlus ? "JioTV+" : "JioTV";
            otp.setText("");
            updateTvLoginState();
            if (!io.isShutdown()) io.execute(() -> refreshServerAndAccount());
        });

        tvAccountCard = tvCard(1010, 263, 840, 160);
        tvAccountCard.setBackground(tvSurface(0xff182b33, 0, 14));
        tvLoginHeading = tvLabel("Provider login", 1010, 370, 800, 39, 32, TV_TEXT, true);
        tvLoginHelper = tvLabel("Enter your mobile number to receive an OTP", 1010, 411, 840, 33, 20, TV_MUTED, false);
        tvMobileLabel = tvLabel("Mobile number", 1010, 471, 810, 30, 22, TV_TEXT, true);
        number = tvInput("10-digit mobile number", 1010, 503, 840);
        number.setInputType(InputType.TYPE_CLASS_NUMBER);
        number.setFilters(new InputFilter[]{new InputFilter.LengthFilter(10)});
        number.addTextChangedListener(new TextWatcher() {
            @Override public void beforeTextChanged(CharSequence s, int start, int count, int after) { }
            @Override public void onTextChanged(CharSequence s, int start, int before, int count) { }
            @Override public void afterTextChanged(Editable s) {
                if (!tvOtpRequested || tvAuthenticated) return;
                tvOtpRequested = false;
                otp.setText("");
                updateTvLoginState();
            }
        });
        tvOtpLabel = tvLabel("One-time password", 1010, 590, 800, 30, 22, TV_TEXT, true);
        otp = tvInput("6-digit OTP", 1010, 622, 840);
        otp.setInputType(InputType.TYPE_CLASS_NUMBER);
        otp.setFilters(new InputFilter[]{new InputFilter.LengthFilter(6)});
        otp.addTextChangedListener(new TextWatcher() {
            @Override public void beforeTextChanged(CharSequence s, int start, int count, int after) { }
            @Override public void onTextChanged(CharSequence s, int start, int before, int count) { }
            @Override public void afterTextChanged(Editable s) { updateTvLoginActionLabel(); }
        });
        tvOtpHint = tvLabel("", 1010, 785, 810, 28, 17, TV_MUTED, false);
        send = tvAction("Send OTP", 1010, 708, 840, 62, true);
        send.setOnClickListener(v -> otpRequest(tvOtpRequested && !otp.getText().toString().trim().isEmpty()));
        verify = send; // TV uses one changing action; phone retains separate buttons.
        tvAccountName = tvLabel("JioTV", 1040, 289, 470, 39, 30, TV_TEXT, true);
        tvAccountNote = tvLabel("●  Signed in", 1040, 333, 500, 30, 21, 0xff98e5c7, true);
        logoutButton = tvAction("Log out", 1570, 303, 245, 65, true);
        logoutButton.setGravity(Gravity.CENTER);
        logoutButton.setBackground(tvPrimaryActionSurface());
        logoutButton.setTextColor(new android.content.res.ColorStateList(
                new int[][]{{-android.R.attr.state_enabled}, {}},
                new int[]{TV_MUTED, 0xff071e1b}));
        logoutButton.setOnClickListener(v -> confirmLogoutProvider());
        tvAccountRetentionNote = tvLabel("Your login remains saved on this device.", 1010, 471, 840, 32, 20, TV_MUTED, false);
        setContentView(tvScroll);
        configureTvFocus(tvScroll);
        updateTvLoginState();
        refreshPermissionButtons();
        tvCanvas.post(this::updateTvFocusGraph);
    }

    private void updateTvLoginState() {
        if (!television || tvAccountCard == null) return;
        View focused = getCurrentFocus();
        tvProviderHeading.setText(tvAuthenticated ? "Provider account" : "Provider");
        tvProviderHelper.setText(tvAuthenticated ? "Your provider is connected." : "Select the account you want to use");
        tvLoginHeading.setText(tvAuthenticated ? "Connected account" : "Provider login");
        tvLoginHelper.setText(tvAuthenticated ? "You're signed in and ready to watch" :
                tvOtpRequested ? "OTP requested · Enter the six-digit code" : "Enter your mobile number to receive an OTP");
        tvLoginHeading.setVisibility(tvAuthenticated ? View.GONE : View.VISIBLE);
        tvLoginHelper.setVisibility(tvAuthenticated ? View.GONE : View.VISIBLE);
        tvAccountCard.setVisibility(tvAuthenticated ? View.VISIBLE : View.GONE);
        ((View) jiotvProvider.getParent()).setVisibility(tvAuthenticated ? View.GONE : View.VISIBLE);
        for (View view : new View[]{tvMobileLabel, number, send}) view.setVisibility(tvAuthenticated ? View.GONE : View.VISIBLE);
        for (View view : new View[]{tvOtpLabel, otp, tvOtpHint})
            view.setVisibility(!tvAuthenticated && tvOtpRequested ? View.VISIBLE : View.GONE);
        tvOtpHint.setText(tvOtpRequested ? "No code? Clear the OTP field, then choose Resend OTP." : "");
        updateTvLoginActionLabel();
        for (View view : new View[]{tvAccountName, tvAccountNote, logoutButton, tvAccountRetentionNote})
            view.setVisibility(tvAuthenticated ? View.VISIBLE : View.GONE);
        tvAccountName.setText(authenticatedProvider);
        logoutButton.setEnabled(!tvLogoutDisabled);
        logoutButton.setFocusable(!tvLogoutDisabled);
        // Move the action directly below the mobile field until an OTP is requested.
        tvMove(send, tvOtpRequested ? 708 : 590);
        tvCanvas.post(this::updateTvFocusGraph);
        if (focused != null && !isTvFocusable(focused))
            requestTvFocus(firstTvFocusable(tvAuthenticated ? logoutButton : send, number, jiotvProvider, service));
    }
    private void updateTvLoginActionLabel() {
        if (!television || send == null) return;
        boolean hasOtp = otp != null && !otp.getText().toString().trim().isEmpty();
        send.setText(tvLoginBusy ? (tvOtpRequested && hasOtp ? "Logging in…" : "Sending OTP…") :
                tvOtpRequested ? (hasOtp ? "Login" : "Resend OTP") : "Send OTP");
    }
    private void tvMove(View view, int y) {
        if (view == null) return;
        FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) view.getLayoutParams();
        lp.topMargin = tvPx(y); view.setLayoutParams(lp);
    }
    private void updateServerBadge(boolean running) {
        tvServerStarted = running;
        if (!television || tvServerBadge == null) return;
        tvServerBadge.setText(running ? "●  Server started" : "●  Server stopped");
        tvServerBadge.setTextColor(running ? 0xff8be7c5 : 0xffffa3a8);
        tvServerBadge.setBackground(tvSurface(running ? 0xff183b30 : 0xff3b2029, 0, 14));
        service.setText(isServiceRunning() ? "Stop local server" : "Start local server");
    }
    private void refreshServerAndAccount() {
        JSONObject response = null;
        try {
            if (keyedPrefix.isEmpty()) runKeyShow();
            HttpURLConnection conn = (HttpURLConnection) new URL(LocalServerEndpoint.baseUrl(this) + keyedPrefix + "api/status").openConnection();
            conn.setConnectTimeout(1200); conn.setReadTimeout(1800);
            try {
                if (conn.getResponseCode() == 200) {
                    ByteArrayOutputStream out = new ByteArrayOutputStream();
                    try (InputStream in = conn.getInputStream()) {
                        byte[] buffer = new byte[1024]; int count;
                        while ((count = in.read(buffer)) > 0) {
                            if (out.size() + count > 64 * 1024) throw new IOException("Status too large");
                            out.write(buffer, 0, count);
                        }
                    }
                    response = new JSONObject(out.toString("UTF-8"));
                }
            } finally { conn.disconnect(); }
        } catch (Exception ignored) { }
        JSONObject current = response;
        runOnUiThread(() -> {
            if (activityDestroyed || !television) return;
            boolean healthy = current != null && current.has("jiotv");
            updateServerBadge(healthy);
            if (healthy) {
                boolean plus = tvPlusProvider.isChecked();
                JSONObject provider = current.optJSONObject(plus ? "extras" : "jiotv");
                tvAuthenticated = provider != null && provider.optBoolean(plus ? "connected" : "loggedIn", false);
                authenticatedProvider = plus ? "JioTV+" : "JioTV";
                tvLogoutDisabled = current.optBoolean("logoutDisabled", false);
                if (tvAuthenticated) tvOtpRequested = false;
            } else {
                tvAuthenticated = false;
            }
            updateTvLoginState();
        });
    }
    private void requestBootPermission() {
        if (Build.VERSION.SDK_INT < 29 || android.provider.Settings.canDrawOverlays(this)) {
            if (ServerService.setBootStartEnabled(this, true)) {
                say("Auto-start enabled. The current server state is unchanged.");
                refreshPermissionButtons();
            } else say("Could not save the auto-start setting.");
            return;
        }
        try {
            rememberTvReturnFocus(bootPermission);
            startActivity(new Intent(android.provider.Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:" + getPackageName())));
        } catch (ActivityNotFoundException e) {
            pendingTvReturnFocus = null;
            say("Use ADB: adb shell appops set " + getPackageName() + " SYSTEM_ALERT_WINDOW allow");
        }
    }
    private void refreshPermissionButtons() {
        boolean allowed = Build.VERSION.SDK_INT < 29 || android.provider.Settings.canDrawOverlays(this);
        boolean permissionControlHadFocus = television && ((bootPermission != null && bootPermission.hasFocus()) || (adbSetupButton != null && adbSetupButton.hasFocus()) || (shizukuButton != null && shizukuButton.hasFocus()));
        boolean bootStartEnabled = ServerService.isBootStartEnabled(this);
        tvAutoStartEnabled = allowed && bootStartEnabled;
        bootPermission.setText(allowed ? "Enable auto-start" : "Enable startup screen (Android 10+)");
        bootPermission.setEnabled(true);
        if (adbSetupButton != null) {
            adbSetupButton.setEnabled(!allowed);
            if (television) adbSetupButton.setFocusable(!allowed);
        }
        if (shizukuButton != null) {
            shizukuButton.setEnabled(!allowed);
            if (television) shizukuButton.setFocusable(!allowed);
        }
        if (!television && disableAutostartButton != null) {
            disableAutostartButton.setVisibility(bootStartEnabled ? View.VISIBLE : View.GONE);
            bootPermission.setVisibility(allowed && bootStartEnabled ? View.GONE : View.VISIBLE);
        }
        if (television) {
            bootPermission.setVisibility(tvAutoStartEnabled ? View.GONE : View.VISIBLE);
            adbSetupButton.setVisibility(allowed ? View.GONE : View.VISIBLE);
            shizukuButton.setVisibility(allowed ? View.GONE : View.VISIBLE);
            disableAutostartButton.setVisibility(bootStartEnabled ? View.VISIBLE : View.GONE);
            tvAutostartLabel.setVisibility(tvAutoStartEnabled ? View.VISIBLE : View.GONE);
            tvStartupCard.setVisibility(tvAutoStartEnabled ? View.VISIBLE : View.GONE);
            tvAutostartLabel.setText("Auto-start enabled");
            tvMove(bootPermission, allowed ? 352 : 428);
            boolean standaloneDisable = bootStartEnabled && !allowed;
            tvMove(disableAutostartButton, standaloneDisable ? 500 : tvBaseY.get(disableAutostartButton));
            int playlistShift = tvAutoStartEnabled ? -17 : allowed ? -55 : standaloneDisable ? 55 : 0;
            int configShift = tvAutoStartEnabled ? -21 : allowed ? -55 : standaloneDisable ? 55 : 0;
            for (View view : tvPlaylistViews) tvMove(view, tvBaseY.get(view) + playlistShift);
            for (View view : tvConfigViews) tvMove(view, tvBaseY.get(view) + configShift);
            updateTvFocusGraph();
            if (allowed && permissionControlHadFocus) requestTvFocus(firstTvFocusable(disableAutostartButton, bootPermission, service));
        }
    }
    private void showAdbSetup() {
        final boolean wireless = Build.VERSION.SDK_INT >= 30;
        rememberTvReturnFocus(adbSetupButton);
        LinearLayout form = new LinearLayout(this); form.setPadding(dp(20), dp(8), dp(20), dp(8)); form.setOrientation(LinearLayout.VERTICAL);
        EditText host = setupField("Device IP address", LOCAL_ADB_HOST); form.addView(host);
        EditText pairPort = setupField("Wireless pairing port (from Wireless debugging)", "");
        EditText pairCode = setupField("Pairing code", ""); pairCode.setInputType(android.text.InputType.TYPE_CLASS_NUMBER | android.text.InputType.TYPE_NUMBER_VARIATION_PASSWORD);
        EditText connectPort = setupField(wireless ? "Wireless debugging port (separate from pairing port)" : "TCP ADB port shown by device / configured by you", "");
        EditText tcpPort = setupField("Classic TCP ADB port (Android 10 / legacy)", Integer.toString(LOCAL_ADB_PORT));
        if (wireless) { form.addView(pairPort); form.addView(pairCode); form.addView(connectPort); }
        form.addView(tcpPort);
        TextView note = new TextView(this); note.setText(wireless ? "Enable Developer options and Wireless debugging first. Pairing uses its own temporary port, then connect using the separate debugging port. Approve any Android authorization prompt on this device." : "Enable Developer options and TCP debugging first. Enter the endpoint exposed by the device; port 5555 is not assumed. Approve Android’s USB debugging authorization prompt on this device."); note.setTextColor(0xffc5d0dd); form.addView(note);
        AlertDialog.Builder dialog = new AlertDialog.Builder(this).setTitle("Manual ADB startup permission setup").setMessage("Local ADB at 127.0.0.1:5555 was unavailable or pairing is required. ADB will grant only Openstream TV Slim’s startup-screen permission and disconnect after setup.").setView(form)
            .setNegativeButton("Cancel", (d,w) -> say("ADB setup cancelled."));
        if (wireless) dialog.setNeutralButton("Pair and grant", null);
        dialog.setPositiveButton(wireless ? "TCP ADB grant" : "Connect and grant", null);
        AlertDialog alert = dialog.create();
        alert.setOnShowListener(d -> {
            Button neutral = alert.getButton(AlertDialog.BUTTON_NEUTRAL);
            Button positive = alert.getButton(AlertDialog.BUTTON_POSITIVE);
            if (wireless && neutral != null) neutral.setOnClickListener(v -> {
                if (runAdbSetup(host, pairPort, pairCode, connectPort, true)) alert.dismiss();
            });
            if (positive != null) positive.setOnClickListener(v -> {
                if (runAdbSetup(host, tcpPort, tcpPort, null, false)) alert.dismiss();
            });
            if (!television) return;
            if (alert.getWindow() != null) alert.getWindow().setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_HIDDEN);
            View[] fields = wireless ? new View[]{host, pairPort, pairCode, connectPort, tcpPort} : new View[]{host, tcpPort};
            configureTvDialogFocus(fields, alert.getButton(AlertDialog.BUTTON_NEGATIVE), neutral, positive);
            host.requestFocus();
        });
        alert.setOnDismissListener(d -> restoreTvReturnFocus());
        alert.show();
    }
    private EditText setupField(String hint, String value) { EditText e = new EditText(this); e.setId(View.generateViewId()); e.setSingleLine(true); e.setHint(hint); e.setText(value); e.setTextColor(0xffffffff); e.setHintTextColor(0xff91a0b3); e.setInputType(hint.toLowerCase().contains("port") ? 2 : android.text.InputType.TYPE_CLASS_TEXT); if (television) { e.setTextSize(18); e.setPadding(dp(14), dp(10), dp(14), dp(10)); e.setBackgroundResource(org.opebstreamstv.android.R.drawable.tv_field_background); } return e; }
    private void runLocalAdbSetup() {
        runAdbSetup(LOCAL_ADB_HOST, LOCAL_ADB_PORT, "", 0, false, true);
    }
    private boolean runAdbSetup(EditText host, EditText port, EditText code, EditText connectionPort, boolean pairFirst) {
        String address=host.getText().toString().trim(), p=port.getText().toString().trim(), pairing=code==null?"":code.getText().toString().trim(), cp=connectionPort==null?"":connectionPort.getText().toString().trim();
        if(address.isEmpty() || p.isEmpty() || (pairFirst && (pairing.isEmpty() || cp.isEmpty()))) { say("Enter the device address and required ADB ports/code."); return false; }
        int portNumber, debugPort=0; try { portNumber=AdbEndpoint.parsePort(p); if(pairFirst) debugPort=AdbEndpoint.parsePort(cp); } catch(IllegalArgumentException e) { say("Enter valid ports from 1 to 65535."); return false; }
        runAdbSetup(address, portNumber, pairing, debugPort, pairFirst, false);
        return true;
    }
    private void runAdbSetup(String address, int pairingPort, String pairing, int finalDebugPort, boolean pairFirst, boolean manualFallback) {
        AdbSetupSession session = new AdbSetupSession();
        adbSetupSession = session;
        boolean adbButtonHadFocus = television && adbSetupButton.hasFocus();
        adbSetupButton.setEnabled(false);
        if (television) {
            updateTvFocusGraph();
            if (adbButtonHadFocus) requestTvFocus(firstTvFocusable(shizukuButton, bootPermission, jiotvProvider, service));
        }
        say(manualFallback ? "Checking local ADB at 127.0.0.1:5555; approve the Android debugging prompt if shown…" : pairFirst ? "Pairing with Wireless debugging…" : "Connecting to TCP ADB; approve the device authorization prompt if shown…");
        setupTask=io.submit(() -> {
            AppAdbManager adb=null; boolean grantCompleted=false; String phase=pairFirst?"pairing":"connecting";
            try {
                adb=new AppAdbManager(getApplicationContext());
                if (!session.install(adb)) {
                    try { adb.close(); } catch (Exception ignored) { }
                    adb = null;
                    return;
                }
                if (session.isCancelled() || Thread.currentThread().isInterrupted()) return;
                if(pairFirst) {
                    if(!pairInPrivateProcess(address,pairingPort,pairing)) throw new IOException("Pairing failed");
                    if (session.isCancelled() || Thread.currentThread().isInterrupted()) return;
                    say("Paired. Connecting on the separate debugging port…"); phase="connecting";
                    if(!adb.connect(address,finalDebugPort)) throw new java.util.concurrent.TimeoutException("ADB connection timed out");
                } else {
                    say("Authorization pending. Approve the ADB prompt on the device; waiting up to 45 seconds…");
                    if(!adb.connect(address,pairingPort)) throw new java.util.concurrent.TimeoutException("ADB connection timed out");
                }
                if (session.isCancelled() || Thread.currentThread().isInterrupted()) return;
                say("Connected. Granting startup-screen permission…"); phase="granting"; adb.runStartupGrant(); grantCompleted=true;
            } catch(AdbPairingRequiredException e) {
                if(manualFallback) {
                    say("Local ADB requires Wireless debugging pairing. Opening manual pairing setup…");
                    runOnUiThread(this::showAdbSetup);
                } else say("Wireless debugging pairing is required. Pair first, then connect using its separate debugging port.");
            }
            catch(java.util.concurrent.TimeoutException | SocketTimeoutException e) { say(phase.equals("pairing") ? "Pairing timed out. Check the pairing code and current pairing port; ADB disconnected. Retry." : "ADB authorization timed out or was denied. Approve the prompt; ADB disconnected. Retry."); }
            catch(java.net.ConnectException e) {
                if(manualFallback) {
                    say("Local ADB is not listening on 127.0.0.1:5555. Opening manual ADB setup…");
                    runOnUiThread(this::showAdbSetup);
                } else say(phase.equals("pairing") ? "Pairing endpoint unavailable. Check debugging, address, and pairing port; ADB disconnected." : "ADB endpoint unavailable. Check that debugging is enabled and the endpoint is current; ADB disconnected.");
            }
            catch(Exception e) { say(phase.equals("pairing") ? "Pairing failed or expired. Check the code and pairing port; ADB disconnected. Retry." : phase.equals("granting") ? "Connected, but the startup permission grant failed; ADB disconnected. Retry or use Settings." : "ADB connection failed. Check debugging, endpoint, authorization; ADB disconnected. Retry."); }
            finally {
                session.close();
                final boolean completed=grantCompleted;
                boolean verified = false;
                if (completed) {
                    for (int attempt = 0; attempt < ADB_PERMISSION_VERIFY_ATTEMPTS; attempt++) {
                        if (Thread.currentThread().isInterrupted()) return;
                        if (android.provider.Settings.canDrawOverlays(MainActivity.this)) {
                            verified = true;
                            break;
                        }
                        if (attempt + 1 < ADB_PERMISSION_VERIFY_ATTEMPTS) {
                            try {
                                Thread.sleep(ADB_PERMISSION_VERIFY_INTERVAL_MS);
                            } catch (InterruptedException e) {
                                Thread.currentThread().interrupt();
                                return;
                            }
                        }
                    }
                }
                final boolean permissionVerified = verified;
                runOnUiThread(() -> {
                    if (isFinishing() || isDestroyed()) return;
                    if(permissionVerified) say("Startup-screen permission verified. ADB disconnected.");
                    else if(completed) say("Grant command completed, but Android still reports permission denied. ADB disconnected.");
                    refreshPermissionButtons();
                });
            }
        });
    }
    private boolean pairInPrivateProcess(String host, int port, String pairingCode) throws Exception {
        CountDownLatch connected = new CountDownLatch(1);
        AtomicReference<IAdbPairingService> serviceRef = new AtomicReference<>();
        ServiceConnection connection = new ServiceConnection() {
            @Override public void onServiceConnected(ComponentName name, IBinder binder) {
                IAdbPairingService service = IAdbPairingService.Stub.asInterface(binder);
                serviceRef.set(service);
                pairingService = service;
                connected.countDown();
            }
            @Override public void onServiceDisconnected(ComponentName name) {
                pairingService = null;
                connected.countDown();
            }
        };
        pairingConnection = connection;
        pairingBound = bindService(new Intent(this, AdbPairingService.class), connection, Context.BIND_AUTO_CREATE);
        if (!pairingBound) throw new IOException("Could not start private ADB pairing process");
        if (!connected.await(5, TimeUnit.SECONDS)) {
            cancelPairingProcess();
            throw new TimeoutException("ADB pairing process did not start");
        }
        IAdbPairingService service = serviceRef.get();
        if (service == null) {
            unbindPairingProcess();
            throw new IOException("ADB pairing process disconnected");
        }
        AtomicBoolean timedOut = new AtomicBoolean(false);
        Runnable timeout = () -> {
            timedOut.set(true);
            cancelPairingProcess();
        };
        mainHandler.postDelayed(timeout, ADB_PAIRING_TIMEOUT_MS);
        try {
            boolean paired = service.pair(host, port, pairingCode);
            if (timedOut.get()) throw new TimeoutException("ADB pairing timed out");
            return paired;
        } catch (DeadObjectException e) {
            if (timedOut.get()) throw new TimeoutException("ADB pairing timed out");
            throw e;
        } finally {
            mainHandler.removeCallbacks(timeout);
            unbindPairingProcess();
        }
    }
    private void cancelPairingProcess() {
        IAdbPairingService service = pairingService;
        if (service != null) try { service.cancel(); } catch (RemoteException ignored) { }
        unbindPairingProcess();
    }
    private void unbindPairingProcess() {
        ServiceConnection connection = pairingConnection;
        if (pairingBound && connection != null) {
            try { unbindService(connection); } catch (IllegalArgumentException ignored) { }
        }
        pairingBound = false;
        pairingConnection = null;
        pairingService = null;
    }
    private void requestShizukuGrant() {
        if (!Shizuku.pingBinder()) { say("Shizuku is not running. Start it, then retry; server startup does not need Shizuku."); return; }
        if (Shizuku.checkSelfPermission()!=PackageManager.PERMISSION_GRANTED) { rememberTvReturnFocus(shizukuButton); Shizuku.requestPermission(SHIZUKU_PERMISSION); say("Waiting for Shizuku authorization…"); return; }
        bindShizukuService();
    }
    private void bindShizukuService() {
        if (!Shizuku.pingBinder()) { say("Shizuku stopped. Restart it and retry."); return; }
        shizukuArgs = new Shizuku.UserServiceArgs(new ComponentName(getPackageName(), StartupPermissionUserService.class.getName())).daemon(false).tag("startup-permission").version(1).processNameSuffix("startup-permission");
        final AtomicBoolean teardownExpected = new AtomicBoolean();
        shizukuTeardownExpected = teardownExpected;
        shizukuConnection = new ServiceConnection() {
            @Override public void onServiceConnected(ComponentName n, IBinder binder) {
                io.execute(() -> { try { Parcel data=Parcel.obtain(), reply=Parcel.obtain(); data.writeInterfaceToken("org.opebstreamstv.android.IStartupPermissionService"); binder.transact(IBinder.FIRST_CALL_TRANSACTION,data,reply,0); reply.readException(); boolean ok=reply.readInt()!=0; data.recycle(); reply.recycle(); runOnUiThread(() -> { say(ok && android.provider.Settings.canDrawOverlays(MainActivity.this) ? "Startup-screen permission verified through Shizuku." : "Shizuku grant failed or Android still reports permission denied."); refreshPermissionButtons(); }); }
                    catch(Exception e) { say("Shizuku stopped or grant failed. Restart it and retry."); } finally { teardownExpected.set(true); Shizuku.unbindUserService(shizukuArgs, shizukuConnection, true); } });
            }
            @Override public void onServiceDisconnected(ComponentName n) { if (!teardownExpected.get()) say("Shizuku disconnected. Retry setup."); }
        };
        try { Shizuku.bindUserService(shizukuArgs,shizukuConnection); say("Requesting the fixed startup-screen permission through Shizuku…"); }
        catch(Exception e) { say("Shizuku service unavailable. Restart it and retry."); }
    }
    private void configureTvFocus(ScrollView scroll) {
        tvScroll = scroll;
        for (View control : allTvControls()) {
            if (control instanceof EditText) configureTvEditField((EditText) control);
            else configureTvFocusable(control);
        }
        updateTvFocusGraph();
    }
    private java.util.List<View> allTvControls() {
        java.util.LinkedHashSet<View> controls = new java.util.LinkedHashSet<>();
        controls.addAll(tvLeftControls);
        controls.addAll(tvRightControls);
        return new java.util.ArrayList<>(controls);
    }
    private void updateTvFocusGraph() {
        if (!television || tvCanvas == null) return;
        java.util.List<View> controls = allTvControls();
        tvFocusGraph.clear();
        int[] directions = {View.FOCUS_UP, View.FOCUS_DOWN, View.FOCUS_LEFT, View.FOCUS_RIGHT};
        for (View from : controls) {
            clearTvFocusTargets(from);
            if (!isTvFocusable(from)) continue;
            View[] edges = new View[directions.length];
            for (int index = 0; index < directions.length; index++) {
                edges[index] = nearestTvControl(from, controls, directions[index]);
                if (edges[index] != null) setTvFocusTarget(from, directions[index], edges[index]);
            }
            tvFocusGraph.put(from, edges);
        }
        if (tvAuthenticated && isTvFocusable(logoutButton)) {
            for (View left : new View[]{service, disableAutostartButton, bootPermission, shizukuButton}) {
                if (!isTvFocusable(left)) continue;
                View[] edges = tvFocusGraph.get(left);
                if (edges != null) edges[3] = logoutButton;
                setTvFocusTarget(left, View.FOCUS_RIGHT, logoutButton);
            }
            View returnTarget = firstTvFocusable(disableAutostartButton, bootPermission, shizukuButton, service);
            View[] logoutEdges = tvFocusGraph.get(logoutButton);
            if (logoutEdges != null && returnTarget != null) logoutEdges[2] = returnTarget;
            setTvFocusTarget(logoutButton, View.FOCUS_LEFT, returnTarget);
        }
    }
    private View nearestTvControl(View from, java.util.List<View> controls, int direction) {
        android.graphics.Rect origin = tvControlBounds(from);
        if (origin == null) return null;
        boolean vertical = direction == View.FOCUS_UP || direction == View.FOCUS_DOWN;
        boolean fromLeft = tvLeftControls.contains(from);
        View best = null;
        int bestAlong = Integer.MAX_VALUE;
        int bestCross = Integer.MAX_VALUE;
        // Vertical D-pad movement advances one row within the current column.
        // A large horizontal gap must never skip the adjacent ADB/Shizuku row.
        // Horizontal movement prefers a control on the same row, then crosses
        // into the other column when the current row has no more controls.
        for (int pass = 0; pass < 2; pass++) {
            best = null;
            bestAlong = Integer.MAX_VALUE;
            bestCross = Integer.MAX_VALUE;
            for (View target : controls) {
                if (target == from || !isTvFocusable(target)) continue;
                boolean sameColumn = fromLeft == tvLeftControls.contains(target);
                if (pass == 0 && !sameColumn) continue;
                if (!vertical && pass == 1 && sameColumn) continue;
                android.graphics.Rect bound = tvControlBounds(target);
                if (bound == null) continue;
                int dx = bound.centerX() - origin.centerX();
                int dy = bound.centerY() - origin.centerY();
                int along = vertical ? (direction == View.FOCUS_DOWN ? dy : -dy)
                        : (direction == View.FOCUS_RIGHT ? dx : -dx);
                if (along <= tvPx(8)) continue;
                int cross = Math.abs(vertical ? dx : dy);
                if (!vertical && pass == 0 && cross > Math.max(origin.height(), bound.height()) / 2) continue;
                if (vertical) {
                    // Slightly offset controls in the same visual row are peers.
                    if (along < bestAlong - tvPx(12) ||
                            (Math.abs(along - bestAlong) <= tvPx(12) && cross < bestCross)) {
                        best = target; bestAlong = along; bestCross = cross;
                    }
                } else {
                    // Across columns, prefer the closest corresponding row.
                    int cost = along + cross * (pass == 0 ? 0 : 3);
                    if (cost < bestAlong) { best = target; bestAlong = cost; bestCross = cross; }
                }
            }
            if (best != null) return best;
        }
        return null;
    }
    private android.graphics.Rect tvControlBounds(View view) {
        if (view.getWidth() == 0 || view.getHeight() == 0) return null;
        android.graphics.Rect rect = new android.graphics.Rect();
        view.getDrawingRect(rect);
        tvCanvas.offsetDescendantRectToMyCoords(view, rect);
        return rect;
    }
    private void configureTvFocusable(View view) {
        if (view == null) return;
        view.setOnFocusChangeListener((v, focused) -> { if (focused) scrollTvFocusIntoView(v); });
    }
    private void configureTvEditField(EditText field) {
        final int inputType = field.getInputType();
        field.setShowSoftInputOnFocus(false);
        field.setCursorVisible(false);
        field.setInputType(android.text.InputType.TYPE_NULL);
        field.setOnFocusChangeListener((v, focused) -> {
            if (focused) scrollTvFocusIntoView(v);
            else if (activeTvEditField == field) exitTvEditMode(field);
        });
        field.setOnKeyListener((v, key, event) -> {
            if (key == KeyEvent.KEYCODE_DPAD_CENTER || key == KeyEvent.KEYCODE_ENTER) {
                if (event.getAction() == KeyEvent.ACTION_UP && activeTvEditField != field) enterTvEditMode(field, inputType);
                return true;
            }
            if (event.getAction() != KeyEvent.ACTION_DOWN || activeTvEditField == field) return false;
            int direction;
            switch (key) {
                case KeyEvent.KEYCODE_DPAD_UP: direction = View.FOCUS_UP; break;
                case KeyEvent.KEYCODE_DPAD_DOWN: direction = View.FOCUS_DOWN; break;
                case KeyEvent.KEYCODE_DPAD_LEFT: direction = View.FOCUS_LEFT; break;
                case KeyEvent.KEYCODE_DPAD_RIGHT: direction = View.FOCUS_RIGHT; break;
                default: return false;
            }
            View next = v.focusSearch(direction);
            if (!isTvFocusable(next) || next == v) return false;
            return next.requestFocus();
        });
    }
    private void enterTvEditMode(EditText field, int inputType) {
        activeTvEditField = field;
        field.setInputType(inputType);
        field.setCursorVisible(true);
        field.setShowSoftInputOnFocus(true);
        field.setSelection(field.length());
        InputMethodManager keyboard = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
        field.post(() -> keyboard.showSoftInput(field, InputMethodManager.SHOW_IMPLICIT));
    }
    private void exitTvEditMode(EditText field) {
        if (activeTvEditField == field) activeTvEditField = null;
        field.setShowSoftInputOnFocus(false);
        field.setCursorVisible(false);
        field.setInputType(android.text.InputType.TYPE_NULL);
        InputMethodManager keyboard = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
        keyboard.hideSoftInputFromWindow(field.getWindowToken(), 0);
    }
    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        if (television && activeTvEditField != null && event.getKeyCode() == KeyEvent.KEYCODE_BACK) {
            if (event.getAction() == KeyEvent.ACTION_UP) {
                EditText field = activeTvEditField;
                exitTvEditMode(field);
                field.requestFocus();
            }
            return true;
        }
        if (television && activeTvEditField == null) {
            int index;
            switch (event.getKeyCode()) {
                case KeyEvent.KEYCODE_DPAD_UP: index = 0; break;
                case KeyEvent.KEYCODE_DPAD_DOWN: index = 1; break;
                case KeyEvent.KEYCODE_DPAD_LEFT: index = 2; break;
                case KeyEvent.KEYCODE_DPAD_RIGHT: index = 3; break;
                default: index = -1;
            }
            if (index >= 0 && tvFocusGraph.containsKey(getCurrentFocus())) {
                if (event.getAction() == KeyEvent.ACTION_DOWN) {
                    updateTvFocusGraph();
                    View[] edges = tvFocusGraph.get(getCurrentFocus());
                    View next = edges == null ? null : edges[index];
                    if (isTvFocusable(next)) next.requestFocus();
                }
                return true;
            }
        }
        return super.dispatchKeyEvent(event);
    }
    private void configureTvDialogFocus(View[] fields, Button negative, Button neutral, Button positive) {
        for (int index = 0; index < fields.length; index++) {
            View field = fields[index];
            if (field instanceof EditText) configureTvEditField((EditText) field);
            setTvFocusTarget(field, View.FOCUS_UP, index > 0 ? fields[index - 1] : null);
            if (index + 1 < fields.length) setTvFocusTarget(field, View.FOCUS_DOWN, fields[index + 1]);
            else setTvFocusTarget(field, View.FOCUS_DOWN, positive, neutral, negative);
        }
        View lastField = fields.length == 0 ? null : fields[fields.length - 1];
        Button[] buttons = {negative, neutral, positive};
        for (Button button : buttons) {
            if (button == null) continue;
            if (button.getId() == View.NO_ID) button.setId(View.generateViewId());
            configureTvFocusable(button);
            setTvFocusTarget(button, View.FOCUS_UP, lastField);
        }
        setTvFocusTarget(negative, View.FOCUS_RIGHT, neutral, positive);
        setTvFocusTarget(neutral, View.FOCUS_LEFT, negative);
        setTvFocusTarget(neutral, View.FOCUS_RIGHT, positive);
        setTvFocusTarget(positive, View.FOCUS_LEFT, neutral, negative);
    }
    private void clearTvFocusTargets(View view) {
        if (view == null) return;
        view.setNextFocusUpId(View.NO_ID); view.setNextFocusDownId(View.NO_ID); view.setNextFocusLeftId(View.NO_ID); view.setNextFocusRightId(View.NO_ID); view.setNextFocusForwardId(View.NO_ID);
    }
    private void setTvFocusTarget(View from, int direction, View... candidates) {
        if (from == null) return;
        View target = firstTvFocusable(candidates);
        int targetId = target == null || target == from ? View.NO_ID : target.getId();
        if (direction == View.FOCUS_UP) from.setNextFocusUpId(targetId);
        else if (direction == View.FOCUS_DOWN) { from.setNextFocusDownId(targetId); from.setNextFocusForwardId(targetId); }
        else if (direction == View.FOCUS_LEFT) from.setNextFocusLeftId(targetId);
        else if (direction == View.FOCUS_RIGHT) from.setNextFocusRightId(targetId);
    }
    private View firstTvFocusable(View... candidates) {
        if (candidates == null) return null;
        for (View candidate : candidates) if (isTvFocusable(candidate)) return candidate;
        return null;
    }
    private boolean isTvFocusable(View view) {
        return view != null && view.isShown() && view.isEnabled() && view.isFocusable();
    }
    private void scrollTvFocusIntoView(View view) {
        view.post(() -> view.requestRectangleOnScreen(new android.graphics.Rect(0, 0, view.getWidth(), view.getHeight()), false));
    }
    private void requestTvFocus(View target) {
        if (!isTvFocusable(target)) return;
        target.post(() -> { if (isTvFocusable(target)) target.requestFocus(); });
    }
    private void rememberTvReturnFocus(View preferred) {
        if (television) pendingTvReturnFocus = preferred;
    }
    private void restoreTvReturnFocus() {
        if (!television || pendingTvReturnFocus == null) return;
        View preferred = pendingTvReturnFocus;
        pendingTvReturnFocus = null;
        View target = firstTvFocusable(preferred);
        if (target == null && (preferred == bootPermission || preferred == adbSetupButton || preferred == shizukuButton)) target = firstTvFocusable(jiotvProvider, service, number);
        if (target == null && (preferred == send || preferred == verify)) target = firstTvFocusable(otp, number, importConfigButton);
        if (target == null) target = firstTvFocusable(service, jiotvProvider, number, importConfigButton);
        requestTvFocus(target);
    }
    private LinearLayout column() { LinearLayout c = new LinearLayout(this); c.setOrientation(LinearLayout.VERTICAL); c.setPadding(television ? dp(14) : 0, television ? dp(10) : 0, television ? dp(14) : 0, television ? dp(10) : 0); return c; }
    private RadioButton providerChoice(String label, boolean checked) { RadioButton b = new RadioButton(this); b.setId(View.generateViewId()); b.setText(label); b.setTextColor(0xfff4f8fc); b.setTextSize(television ? 15 : 14); b.setChecked(checked); b.setFocusable(true); b.setFocusableInTouchMode(false); if (television) { b.setMinHeight(dp(54)); b.setPadding(dp(12), dp(8), dp(12), dp(8)); b.setBackgroundResource(org.opebstreamstv.android.R.drawable.tv_field_background); } return b; }
    private void pickFile(String type, int request) {
        rememberTvReturnFocus(request == PICK_CONFIG ? importConfigButton : importStoreButton);
        if (television && Build.VERSION.SDK_INT <= 32) {
            openLocalTomlPicker(request);
            return;
        }
        Intent document = new Intent(Intent.ACTION_OPEN_DOCUMENT); document.setType("*/*"); document.addCategory(Intent.CATEGORY_OPENABLE);
        try { startActivityForResult(document, request); }
        catch (ActivityNotFoundException missingPicker) {
            if (Build.VERSION.SDK_INT >= 33) {
                say("No system file picker is available. Enable or install a document picker to import files on Android 13+.");
                restoreTvReturnFocus();
                return;
            }
            Intent fallback = new Intent(Intent.ACTION_GET_CONTENT); fallback.setType("*/*"); fallback.addCategory(Intent.CATEGORY_OPENABLE);
            try { startActivityForResult(fallback, request); }
            catch (ActivityNotFoundException noFallback) { openLocalTomlPicker(request); }
        }
    }
    private void createConfigDocument() {
        rememberTvReturnFocus(exportConfigButton);
        if (television && Build.VERSION.SDK_INT <= 32) {
            openLocalTomlPicker(CREATE_CONFIG);
            return;
        }
        Intent i = new Intent(Intent.ACTION_CREATE_DOCUMENT); i.addCategory(Intent.CATEGORY_OPENABLE); i.setType("application/octet-stream"); i.putExtra(Intent.EXTRA_TITLE, "config.toml");
        try { startActivityForResult(i, CREATE_CONFIG); }
        catch (ActivityNotFoundException e) {
            if (Build.VERSION.SDK_INT >= 33) {
                say("No system file picker is available. Enable or install a document picker to export files on Android 13+.");
                restoreTvReturnFocus();
            } else openLocalTomlPicker(CREATE_CONFIG);
        }
    }
    private void createPlaylistDocument() {
        rememberTvReturnFocus(saveM3uButton);
        if (playlistUrl.isEmpty()) {
            say("Playlist is not ready. Start the server and try again.");
            restoreTvReturnFocus();
            return;
        }
        if (television && Build.VERSION.SDK_INT <= 32) {
            openLocalTomlPicker(CREATE_PLAYLIST);
            return;
        }
        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("audio/x-mpegurl");
        intent.putExtra(Intent.EXTRA_TITLE, "openstream-playlist.m3u");
        try { startActivityForResult(intent, CREATE_PLAYLIST); }
        catch (ActivityNotFoundException e) {
            if (Build.VERSION.SDK_INT >= 33) {
                say("No system file picker is available to save the playlist on Android 13+.");
                restoreTvReturnFocus();
            } else openLocalTomlPicker(CREATE_PLAYLIST);
        }
    }
    private void openLocalTomlPicker(int request) {
        pendingLocalPickerRequest = request;
        if (!hasLegacyStoragePermission()) {
            requestPermissions(new String[]{Manifest.permission.READ_EXTERNAL_STORAGE, Manifest.permission.WRITE_EXTERNAL_STORAGE}, STORAGE_PERMISSION);
            return;
        }
        showLocalTomlPicker();
    }
    private boolean hasLegacyStoragePermission() {
        if (Build.VERSION.SDK_INT < 23 || Build.VERSION.SDK_INT >= 33) return true;
        return checkSelfPermission(Manifest.permission.READ_EXTERNAL_STORAGE) == PackageManager.PERMISSION_GRANTED
                && checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) == PackageManager.PERMISSION_GRANTED;
    }
    private void showLocalTomlPicker() {
        int request = pendingLocalPickerRequest;
        pendingLocalPickerRequest = 0;
        if (request != PICK_CONFIG && request != PICK_STORE && request != CREATE_CONFIG && request != CREATE_PLAYLIST) return;
        File root = Environment.getExternalStorageDirectory();
        File downloads = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS);
        File start = downloads.isDirectory() ? downloads : root;
        LocalTomlPicker.Callback callback = new LocalTomlPicker.Callback() {
            @Override public void onImport(File file) {
                if (request == PICK_CONFIG) {
                    say("Validating config locally…");
                    io.execute(() -> importConfig(Uri.fromFile(file)));
                } else {
                    say("Validating login store backup…");
                    io.execute(() -> importStore(Uri.fromFile(file)));
                }
                restoreTvReturnFocus();
            }
            @Override public void onExport(File file) {
                if (request == CREATE_PLAYLIST) {
                    say("Saving playlist…");
                    io.execute(() -> savePlaylist(Uri.fromFile(file)));
                } else {
                    say("Exporting config…");
                    io.execute(() -> exportConfig(Uri.fromFile(file)));
                }
                restoreTvReturnFocus();
            }
            @Override public void onCancel() { restoreTvReturnFocus(); }
            @Override public void onError(String message) { say(message); restoreTvReturnFocus(); }
        };
        if (request == CREATE_CONFIG || request == CREATE_PLAYLIST)
            LocalTomlPicker.showExport(this, root, start,
                    request == CREATE_PLAYLIST ? "openstream-playlist.m3u" : "config.toml", callback);
        else LocalTomlPicker.showImport(this, root, start, request == PICK_STORE, callback);
    }
    @Override public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode != STORAGE_PERMISSION) return;
        if (hasLegacyStoragePermission()) showLocalTomlPicker();
        else {
            pendingLocalPickerRequest = 0;
            say("Storage permission is required for the built-in file browser.");
            restoreTvReturnFocus();
        }
    }
    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
    private LinearLayout.LayoutParams spacedParams(int top, int bottom) { LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-1, -2); lp.topMargin = dp(top); lp.bottomMargin = dp(bottom); return lp; }
    private TextView addText(LinearLayout p, String s, int size, int color) { TextView t = new TextView(this); t.setText(s); t.setTextSize(size); t.setTextColor(color); t.setPadding(0, dp(television ? 3 : 10), 0, dp(television ? 3 : 10)); p.addView(t, spacedParams(television ? 1 : 4, television ? 1 : 4)); return t; }
    private EditText field(LinearLayout p, String hint) { EditText e = new EditText(this); e.setId(View.generateViewId()); e.setSingleLine(true); e.setHint(hint); e.setTextColor(0xffffffff); e.setHintTextColor(0xff91a0b3); e.setTextSize(television ? 20 : 16); e.setPadding(dp(16), dp(television ? 14 : 8), dp(16), dp(television ? 14 : 8)); e.setBackgroundResource(org.opebstreamstv.android.R.drawable.tv_field_background); p.addView(e, spacedParams(television ? 8 : 4, television ? 8 : 4)); return e; }
    private Button button(LinearLayout p, String label) { Button b = new Button(this); b.setId(View.generateViewId()); b.setText(label); b.setAllCaps(false); b.setTextSize(television ? 18 : 14); b.setTextColor(getResources().getColorStateList(org.opebstreamstv.android.R.color.tv_button_text)); b.setMinHeight(dp(television ? 64 : 48)); b.setPadding(dp(20), dp(television ? 12 : 8), dp(20), dp(television ? 12 : 8)); b.setBackgroundResource(org.opebstreamstv.android.R.drawable.tv_button_background); b.setFocusable(true); b.setFocusableInTouchMode(false); LinearLayout.LayoutParams lp = p.getOrientation() == LinearLayout.HORIZONTAL ? new LinearLayout.LayoutParams(0, -2, 1) : spacedParams(television ? 8 : 4, television ? 8 : 4); if (p.getOrientation() == LinearLayout.HORIZONTAL) { lp.leftMargin = dp(4); lp.rightMargin = dp(4); } p.addView(b, lp); return b; }
    private void say(String s) { runOnUiThread(() -> { if (status != null) status.setText(s); }); }
    private final Shizuku.OnRequestPermissionResultListener shizukuPermissionListener = (requestCode, result) -> {
        if (requestCode != SHIZUKU_PERMISSION) return;
        restoreTvReturnFocus();
        if (result == PackageManager.PERMISSION_GRANTED) bindShizukuService(); else say("Shizuku permission denied. Retry or use Android settings / ADB.");
    };

    private void toggleService() {
        if (isServiceRunning()) { startService(new Intent(this, ServerService.class).setAction(ServerService.ACTION_STOP)); service.setText("Start local server"); say("Server stopped"); }
        else { Intent i = new Intent(this, ServerService.class).setAction(ServerService.ACTION_START); if (Build.VERSION.SDK_INT >= 26) startForegroundService(i); else startService(i); service.setText("Stop local server"); say("Starting server…"); pollServer(0); }
    }
    private boolean isServiceRunning() { android.app.ActivityManager m = (android.app.ActivityManager)getSystemService(ACTIVITY_SERVICE); for (android.app.ActivityManager.RunningServiceInfo i:m.getRunningServices(Integer.MAX_VALUE)) if (ServerService.class.getName().equals(i.service.getClassName())) return true; return false; }
    private void showPlaylistUrl() { runOnUiThread(() -> playlist.setText(PlaylistDisplay.masked(playlistUrl))); }
    private void refreshUrl() { io.execute(() -> { try { String path = runKeyShow(); playlistUrl=PlaylistLink.loopbackUrl(path, LocalServerEndpoint.port(this)); boolean running=serverHealthy(); showPlaylistUrl(); say(running ? "Server is running on this device. Log in to load channels." : "Server is stopped. Start it to log in and stream."); } catch (Exception e) { android.util.Log.e("JioTVGo", "Cannot initialize local playlist", e); say("Could not read playlist key. Check the bundled server and imported config."); } }); }
    private void pollServer(int attempt) {
        synchronized (serverPollLock) {
            if (activityDestroyed || io.isShutdown()) return;
            io.execute(() -> {
                if (activityDestroyed) return;
                boolean running=false;
                try { String path=runKeyShow(); playlistUrl=PlaylistLink.loopbackUrl(path, LocalServerEndpoint.port(this)); showPlaylistUrl(); running=serverHealthy(); } catch(Exception ignored) {}
                if (activityDestroyed) return;
                if(running) { say("Server is running. Log in to load channels."); return; }
                if(attempt<12) {
                    synchronized (serverPollLock) {
                        if (!activityDestroyed) serverPollHandler.postDelayed(() -> pollServer(attempt+1),750);
                    }
                } else say("Server did not become ready. Check the local port, config, and the APK ABI.");
            });
        }
    }
    private boolean serverHealthy() throws Exception { HttpURLConnection c=(HttpURLConnection)new URL(LocalServerEndpoint.baseUrl(this)+keyedPrefix+"api/status").openConnection(); c.setConnectTimeout(1200); c.setReadTimeout(1800); c.setRequestMethod("GET"); try { int code=c.getResponseCode(); if(code!=200) return false; ByteArrayOutputStream out=new ByteArrayOutputStream(); try(InputStream in=c.getInputStream()){ byte[] buf=new byte[1024]; int n; while((n=in.read(buf))!=-1) out.write(buf,0,n); } return new JSONObject(out.toString("UTF-8")).has("jiotv"); } catch (ConnectException e) { return false; } finally { c.disconnect(); } }
    private String runKeyShow() throws Exception { File binary=new File(getApplicationInfo().nativeLibraryDir,"libopenstream_exec.so"); java.util.List<String> command=new java.util.ArrayList<>(); command.add(binary.getAbsolutePath()); command.add("--skip-update-check"); File config=new File(getFilesDir(),"config.toml"); if(config.isFile()) { command.add("--config"); command.add(config.getAbsolutePath()); } command.add("key"); command.add("show"); ProcessBuilder b=new ProcessBuilder(command); b.directory(getFilesDir()); b.redirectError(new File("/dev/null")); b.environment().put("JIOTV_PATH_PREFIX",new File(getFilesDir(),"data").getAbsolutePath()); b.environment().put("HOME",getFilesDir().getAbsolutePath()); Process p=b.start(); String result; try (InputStream in=p.getInputStream()) { result=PlaylistLink.readValidatedPath(in); } catch (Exception e) { p.destroy(); throw e; } if(p.waitFor()!=0) throw new IOException("key command failed"); keyedPrefix=PlaylistLink.keyedPrefix(result); return result; }

    private void otpRequest(boolean verifyRequest) {
        if (television && tvLoginBusy) return;
        String n = number.getText().toString().trim(), code = otp.getText().toString().trim();
        if (!n.matches("[0-9]{10}")) { say("Enter exactly 10 digits for the mobile number."); return; }
        if (verifyRequest && !code.matches("[0-9]{6}")) { say("Enter exactly 6 digits for the OTP."); return; }
        final boolean tvPlus = tvPlusProvider != null && tvPlusProvider.isChecked();
        if (television) {
            tvLoginBusy = true;
            updateTvLoginState();
        } else {
            send.setEnabled(false); verify.setEnabled(false);
        }
        say((verifyRequest ? "Verifying " : "Requesting ") + (tvPlus ? "JioTV+" : "JioTV") + " OTP…");
        io.execute(() -> {
            try {
                if(keyedPrefix.isEmpty()) runKeyShow();
                JSONObject body=new JSONObject(); body.put("number",n); if(verifyRequest) body.put("otp",code);
                String path = keyedPrefix + (tvPlus ? "extras/login/" : "login/") + (verifyRequest ? "verifyOTP" : "sendOTP");
                JSONObject response=post(path,body);
                if (tvPlus) {
                    if (verifyRequest) {
                        if (response.optBoolean("status", false)) completeTvLogin(n, true);
                        else say("JioTV+ OTP was rejected or expired. Check the code and retry.");
                    } else {
                        org.json.JSONArray connections = response.optJSONArray("connections");
                        if (connections != null && connections.length() > 0) runOnUiThread(() -> showTvPlusConnectionChooser(n, connections));
                        else if (response.optBoolean("status", false)) completeTvOtpRequest(n, true);
                        else say("JioTV+ OTP could not be sent. Check the number and try again.");
                    }
                } else if (verifyRequest && "success".equals(response.optString("status"))) completeTvLogin(n, false);
                else if (!verifyRequest && response.optBoolean("status", false)) completeTvOtpRequest(n, false);
                else say(verifyRequest ? "JioTV OTP was rejected or expired. Check the code and retry." :
                        "JioTV OTP could not be sent. Check the number and try again.");
            } catch(Exception e) {
                say("Could not complete " + (tvPlus ? "JioTV+" : "JioTV") + " login. Check your connection and try again.");
            } finally {
                runOnUiThread(() -> {
                    if (television) {
                        tvLoginBusy = false;
                        updateTvLoginState();
                    } else {
                        send.setEnabled(true); verify.setEnabled(true);
                    }
                });
            }
        });
    }
    private boolean matchesPendingProvider(String mobile, boolean plus) {
        return mobile.equals(number.getText().toString().trim()) &&
                (tvPlusProvider != null && tvPlusProvider.isChecked()) == plus;
    }
    private void completeTvOtpRequest(String mobile, boolean plus) {
        runOnUiThread(() -> {
            if (activityDestroyed || !matchesPendingProvider(mobile, plus)) return;
            if (television) {
                tvOtpRequested = true;
                updateTvLoginState();
                requestTvFocus(otp);
            }
            say((plus ? "JioTV+" : "JioTV") + " OTP sent. Enter the six-digit code.");
        });
    }
    private void completeTvLogin(String mobile, boolean plus) {
        runOnUiThread(() -> {
            if (activityDestroyed || !matchesPendingProvider(mobile, plus)) return;
            if (television) {
                tvAuthenticated = true;
                tvOtpRequested = false;
                authenticatedProvider = plus ? "JioTV+" : "JioTV";
                otp.setText("");
                updateTvLoginState();
                requestTvFocus(logoutButton);
            }
            say((plus ? "JioTV+" : "JioTV") + " login complete. Playlist is ready.");
        });
    }
    private void showTvPlusConnectionChooser(String number, org.json.JSONArray connections) {
        String[] labels = new String[connections.length()];
        int[] connectionIndexes = new int[connections.length()];
        for (int i = 0; i < connections.length(); i++) {
            JSONObject connection = connections.optJSONObject(i);
            connectionIndexes[i] = connection == null ? i : connection.optInt("index", i);
            if (connection == null) { labels[i] = "Connection " + (i + 1); continue; }
            String name = connection.optString("name", "").trim();
            String product = connection.optString("product", "").trim();
            String tail = connection.optString("lineEndsWith", "").trim();
            StringBuilder label = new StringBuilder(name.isEmpty() ? "Connection " + (i + 1) : name);
            if (!product.isEmpty()) label.append(" · ").append(product);
            if (!tail.isEmpty()) label.append(" · ••••").append(tail);
            labels[i] = label.toString();
        }
        AlertDialog dialog = new AlertDialog.Builder(this)
                .setTitle("Choose JioTV+ connection")
                .setItems(labels, (d, which) -> sendTvPlusOtpForConnection(number, connectionIndexes[which]))
                .setNegativeButton("Cancel", null)
                .create();
        dialog.setOnDismissListener(d -> { if (television && !tvOtpRequested) requestTvFocus(send); });
        dialog.show();
    }
    private void sendTvPlusOtpForConnection(String number, int connectionIndex) {
        if (television) {
            if (tvLoginBusy) return;
            tvLoginBusy = true;
            updateTvLoginState();
        } else {
            send.setEnabled(false); verify.setEnabled(false);
        }
        say("Sending JioTV+ OTP…");
        io.execute(() -> {
            try {
                if(keyedPrefix.isEmpty()) runKeyShow();
                JSONObject body = new JSONObject();
                body.put("number", number);
                body.put("connection", connectionIndex);
                JSONObject response = post(keyedPrefix + "extras/login/sendOTP", body);
                if (response.optBoolean("status", false)) completeTvOtpRequest(number, true);
                else say("JioTV+ OTP could not be sent. Choose a connection and retry.");
            } catch (Exception e) {
                say("Could not send the JioTV+ OTP. Check your connection and try again.");
            } finally {
                runOnUiThread(() -> {
                    if (television) {
                        tvLoginBusy = false;
                        updateTvLoginState();
                    } else {
                        send.setEnabled(true); verify.setEnabled(true);
                    }
                });
            }
        });
    }
    private JSONObject post(String path, JSONObject data) throws Exception { HttpURLConnection c=(HttpURLConnection)new URL(LocalServerEndpoint.baseUrl(this)+path).openConnection(); c.setRequestMethod("POST"); c.setConnectTimeout(8000); c.setReadTimeout(20000); c.setDoOutput(true); c.setRequestProperty("Content-Type","application/json"); try(OutputStream o=c.getOutputStream()){o.write(data.toString().getBytes(StandardCharsets.UTF_8));} InputStream in=c.getResponseCode()<400?c.getInputStream():c.getErrorStream(); ByteArrayOutputStream out=new ByteArrayOutputStream(); byte[] buf=new byte[512]; int n; while((n=in.read(buf))!=-1) out.write(buf,0,n); return new JSONObject(out.toString("UTF-8")); }
    private void copyUrl() { if(playlistUrl.isEmpty()) return; ((android.content.ClipboardManager)getSystemService(CLIPBOARD_SERVICE)).setPrimaryClip(ClipData.newPlainText("JioTV playlist",playlistUrl)); say("Playlist URL copied"); }
    private void confirmLogoutProvider() {
        if (tvLogoutDisabled) { say("Logout is currently unavailable."); return; }
        final boolean plus = tvPlusProvider != null && tvPlusProvider.isChecked();
        rememberTvReturnFocus(logoutButton);
        AlertDialog alert = new AlertDialog.Builder(this)
                .setTitle("Log out of " + (plus ? "JioTV+" : "JioTV") + "?")
                .setMessage("You will need to sign in with OTP again to reconnect this provider.")
                .setNegativeButton("Cancel", null)
                .setPositiveButton("Log out", null)
                .create();
        alert.setOnShowListener(dialog -> {
            Button cancel = alert.getButton(AlertDialog.BUTTON_NEGATIVE);
            Button logout = alert.getButton(AlertDialog.BUTTON_POSITIVE);
            if (logout != null) logout.post(() -> logout.setOnClickListener(v -> {
                alert.dismiss();
                logoutProvider(plus);
            }));
            if (television) {
                configureTvDialogFocus(new View[0], cancel, null, logout);
                if (cancel != null) cancel.requestFocus();
            }
        });
        alert.setOnDismissListener(dialog -> restoreTvReturnFocus());
        alert.show();
    }
    private void logoutProvider(boolean plus) {
        logoutButton.setEnabled(false);
        say("Signing out of " + (plus ? "JioTV+" : "JioTV") + "…");
        io.execute(() -> {
            boolean success = false;
            try {
                if (keyedPrefix.isEmpty()) runKeyShow();
                JSONObject response = post(keyedPrefix + (plus ? "api/extras/logout" : "api/jiotv/logout"), new JSONObject());
                success = response.optBoolean("status", false);
            } catch (Exception ignored) { }
            boolean loggedOut = success;
            runOnUiThread(() -> {
                if (activityDestroyed) return;
                if (loggedOut) {
                    if ((tvPlusProvider != null && tvPlusProvider.isChecked()) == plus) {
                        tvAuthenticated = false;
                        tvOtpRequested = false;
                        otp.setText("");
                    }
                    say((plus ? "JioTV+" : "JioTV") + " signed out.");
                    updateTvLoginState();
                    requestTvFocus(number);
                }
                logoutButton.setEnabled(!tvLogoutDisabled);
                if (!loggedOut) {
                    say("Logout failed. Check the server and try again.");
                    requestTvFocus(logoutButton);
                }
                if (!io.isShutdown()) io.execute(this::refreshServerAndAccount);
            });
        });
    }
    private Intent iptvLaunchIntent(String packageName) {
        PackageManager pm = getPackageManager();
        Intent launch = pm.getLeanbackLaunchIntentForPackage(packageName);
        if (launch == null) launch = pm.getLaunchIntentForPackage(packageName);
        return launch != null && launch.resolveActivity(pm) != null ? launch : null;
    }
    private String appLabel(Intent launch) {
        android.content.pm.ActivityInfo resolved = launch.resolveActivityInfo(getPackageManager(), 0);
        if (resolved == null) return launch.getPackage();
        CharSequence label = resolved.loadLabel(getPackageManager());
        return label == null ? resolved.packageName : label.toString();
    }
    private void launchIptvApp(Intent launch) {
        try {
            startActivity(launch);
            say("Playlist URL copied. Add or import it in the IPTV app.");
        } catch (ActivityNotFoundException e) {
            restoreTvReturnFocus();
            say("Playlist URL copied, but the selected IPTV app could not be opened.");
        }
    }
    private void openUrl() {
        if (playlistUrl.isEmpty()) return;
        rememberTvReturnFocus(openButton);
        ((android.content.ClipboardManager)getSystemService(CLIPBOARD_SERVICE)).setPrimaryClip(ClipData.newPlainText("JioTV playlist", playlistUrl));

        Intent tvPlayer = iptvLaunchIntent("ar.tvplayer.tv");
        Intent tubPlayer = iptvLaunchIntent("com.tubs.tubplayer");
        if (tvPlayer == null && tubPlayer == null) {
            restoreTvReturnFocus();
            say("Playlist URL copied. No supported IPTV app is installed.");
            return;
        }
        if (tvPlayer == null || tubPlayer == null) {
            launchIptvApp(tvPlayer != null ? tvPlayer : tubPlayer);
            return;
        }

        Intent[] launches = {tvPlayer, tubPlayer};
        String[] labels = {appLabel(tvPlayer), appLabel(tubPlayer)};
        android.app.AlertDialog dialog = new android.app.AlertDialog.Builder(this)
                .setTitle("Open IPTV app")
                .setItems(labels, (d, which) -> launchIptvApp(launches[which]))
                .setOnCancelListener(d -> restoreTvReturnFocus())
                .create();
        dialog.show();
    }
    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if ((request != PICK_CONFIG && request != PICK_STORE && request != CREATE_CONFIG && request != CREATE_PLAYLIST)
                || result != RESULT_OK || data == null || data.getData() == null) {
            restoreTvReturnFocus();
            return;
        }
        Uri uri = data.getData();
        if (request == PICK_CONFIG) { say("Validating config locally…"); io.execute(() -> importConfig(uri)); }
        else if (request == PICK_STORE) { say("Validating login store backup…"); io.execute(() -> importStore(uri)); }
        else if (request == CREATE_PLAYLIST) { say("Saving playlist…"); io.execute(() -> savePlaylist(uri)); }
        else { say("Exporting config…"); io.execute(() -> exportConfig(uri)); }
        restoreTvReturnFocus();
    }
    private InputStream openImportStream(Uri uri) throws IOException {
        if ("file".equals(uri.getScheme())) {
            String path = uri.getPath();
            if (path == null) throw new IOException("Missing file path");
            return new FileInputStream(new File(path));
        }
        InputStream in = getContentResolver().openInputStream(uri);
        if (in == null) throw new IOException("Cannot read selected file");
        return in;
    }
    private OutputStream openExportStream(Uri uri) throws IOException {
        if ("file".equals(uri.getScheme())) {
            String path = uri.getPath();
            if (path == null) throw new IOException("Missing file path");
            return new FileOutputStream(new File(path));
        }
        OutputStream out = getContentResolver().openOutputStream(uri);
        if (out == null) throw new IOException("Cannot open export destination");
        return out;
    }
    private void exportConfig(Uri uri) { try (OutputStream out=openExportStream(uri)) { ConfigExport.write(new File(getFilesDir(), "config.toml"), out); say("Config exported. Login data was not included."); } catch(Exception e) { say("Config export failed. Choose a writable destination and try again."); } }
    private void savePlaylist(Uri destination) {
        File staged = new File(getCacheDir(), "playlist-export-" + System.nanoTime() + ".m3u");
        HttpURLConnection connection = null;
        try {
            // The existing validated playlist link is loopback and keyed. Never log it.
            String link = playlistUrl;
            if (!link.startsWith(LocalServerEndpoint.baseUrl(this) + "/")) throw new IOException("Invalid local playlist URL");
            URL playlistLink = new URL(link);
            String contentPath = PlaylistLink.playlistContentPath(playlistLink.getPath());
            connection = (HttpURLConnection) new URL(LocalServerEndpoint.baseUrl(this) + contentPath).openConnection();
            connection.setInstanceFollowRedirects(false);
            connection.setConnectTimeout(4000);
            connection.setReadTimeout(15000);
            if (connection.getResponseCode() != HttpURLConnection.HTTP_OK) throw new IOException("Playlist unavailable");
            try (InputStream in = connection.getInputStream(); OutputStream out = new FileOutputStream(staged)) {
                byte[] buf = new byte[8192];
                int n;
                long total = 0;
                while ((n = in.read(buf)) != -1) {
                    total += n;
                    if (total > 32L * 1024 * 1024) throw new IOException("Playlist exceeds export limit");
                    out.write(buf, 0, n);
                }
            }
            try (InputStream in = new FileInputStream(staged)) {
                byte[] header = new byte[7];
                if (in.read(header) != header.length || !"#EXTM3U".equals(new String(header, StandardCharsets.US_ASCII)))
                    throw new IOException("Unexpected playlist response");
            }
            try (InputStream in = new FileInputStream(staged); OutputStream out = openExportStream(destination)) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = in.read(buf)) != -1) out.write(buf, 0, n);
            }
            say("M3U playlist saved. Keep the local server running for loopback streams.");
        } catch (Exception e) {
            say("Playlist export failed. Start the server, sign in, and choose a writable file.");
        } finally {
            if (connection != null) connection.disconnect();
            staged.delete();
        }
    }
    private void importConfig(Uri uri) {
        File staged = new File(getCacheDir(), "config.import.toml");
        File validationData = new File(getCacheDir(), "validate-" + System.nanoTime());
        File temp = new File(getFilesDir(), "config.toml.tmp");
        try {
            try (InputStream in = openImportStream(uri);
                 OutputStream out = new FileOutputStream(staged)) {
                byte[] buffer = new byte[8192];
                int n, total = 0;
                while ((n = in.read(buffer)) != -1) {
                    total += n;
                    if (total > 1024 * 1024) throw new IOException("Config too large");
                    out.write(buffer, 0, n);
                }
            }
            if (!validationData.mkdir()) throw new IOException("Cannot create validation directory");
            ProcessBuilder pb = new ProcessBuilder(
                    new File(getApplicationInfo().nativeLibraryDir, "libopenstream_exec.so").getAbsolutePath(),
                    "--skip-update-check", "--config", staged.getAbsolutePath(), "key", "show");
            pb.directory(getFilesDir());
            pb.redirectOutput(new File("/dev/null"));
            pb.redirectError(new File("/dev/null"));
            pb.environment().put("JIOTV_PATH_PREFIX", validationData.getAbsolutePath());
            Process p = pb.start();
            if (!p.waitFor(10, java.util.concurrent.TimeUnit.SECONDS)) {
                p.destroyForcibly();
                throw new IOException("Config validation timed out");
            }
            if (p.exitValue() != 0) throw new IOException("Invalid TOML config");
            try (InputStream in = new FileInputStream(staged);
                 OutputStream out = new FileOutputStream(temp)) {
                byte[] buffer = new byte[8192];
                int n;
                while ((n = in.read(buffer)) != -1) out.write(buffer, 0, n);
            }
            if (!temp.renameTo(new File(getFilesDir(), "config.toml"))) {
                throw new IOException("Could not save config");
            }
            say("Config imported and validated. Restart the server to apply it.");
        } catch (Exception e) {
            say("Config import failed. Choose a valid TOML config; existing config is unchanged.");
        } finally {
            staged.delete();
            temp.delete();
            // Validation must never modify the live server's store or access key.
            File[] files = validationData.listFiles();
            if (files != null) for (File file : files) file.delete();
            validationData.delete();
        }
    }

    private void importStore(Uri uri) {
        File staged = new File(getCacheDir(), "store-import-" + System.nanoTime() + ".toml");
        File validationData = new File(getCacheDir(), "store-validation-" + System.nanoTime());
        File liveData = new File(getFilesDir(), "data");
        File temp = new File(liveData, "store_v4.toml.tmp-" + System.nanoTime());
        boolean restart = false;
        try {
            try (InputStream in = openImportStream(uri)) {
                StoreBackup.copyBounded(in, staged, 2 * 1024 * 1024);
            }
            if (!StoreBackup.declaresAccessKey(staged)) throw new IOException("Store backup has no access key");
            if (!validationData.mkdir()) throw new IOException("Cannot create validation directory");
            File candidate = new File(validationData, "store_v4.toml");
            try (InputStream in = new FileInputStream(staged)) { StoreBackup.copyBounded(in, candidate, 2 * 1024 * 1024); }
            ProcessBuilder pb = new ProcessBuilder(new File(getApplicationInfo().nativeLibraryDir, "libopenstream_exec.so").getAbsolutePath(), "--skip-update-check", "key", "show");
            pb.directory(validationData);
            File keyOutput = new File(validationData, "playlist-path");
            pb.redirectOutput(keyOutput);
            pb.redirectError(new File("/dev/null"));
            pb.environment().put("JIOTV_PATH_PREFIX", validationData.getAbsolutePath());
            pb.environment().put("HOME", getFilesDir().getAbsolutePath());
            Process process = pb.start();
            if (!process.waitFor(10, java.util.concurrent.TimeUnit.SECONDS)) { process.destroyForcibly(); throw new IOException("Store validation timed out"); }
            if (process.exitValue() != 0) throw new IOException("Invalid store schema");
            String restoredPath;
            try (InputStream in = new FileInputStream(keyOutput)) { restoredPath = PlaylistLink.readValidatedPath(in); }
            if (!liveData.isDirectory() && !liveData.mkdirs()) throw new IOException("Cannot access app data directory");
            try (InputStream in = new FileInputStream(candidate)) { StoreBackup.copyBounded(in, temp, 2 * 1024 * 1024); }
            restart = isServiceRunning();
            if (restart && !stopServiceAndWait()) throw new IOException("Could not stop server safely");
            Os.rename(temp.getAbsolutePath(), new File(liveData, "store_v4.toml").getAbsolutePath());
            keyedPrefix = PlaylistLink.keyedPrefix(restoredPath);
            playlistUrl = PlaylistLink.loopbackUrl(restoredPath, LocalServerEndpoint.port(this));
            showPlaylistUrl();
            if (restart) startServerService();
            say(restart ? "Login store imported. Server restarted." : "Login store imported. Start the server to use it.");
        } catch (Exception e) {
            say("Login store import failed. Existing login data was left unchanged.");
        } finally {
            staged.delete(); temp.delete();
            File[] children = validationData.listFiles();
            if (children != null) for (File child : children) child.delete();
            validationData.delete();
        }
    }

    private boolean stopServiceAndWait() throws Exception {
        java.util.concurrent.CountDownLatch stopped = new java.util.concurrent.CountDownLatch(1);
        BroadcastReceiver receiver = new BroadcastReceiver() { @Override public void onReceive(Context context, Intent intent) { stopped.countDown(); } };
        IntentFilter filter = new IntentFilter(ServerService.ACTION_STOPPED);
        if (Build.VERSION.SDK_INT >= 33) registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED);
        else registerReceiver(receiver, filter);
        try {
            startService(new Intent(this, ServerService.class).setAction(ServerService.ACTION_STOP));
            return stopped.await(8, java.util.concurrent.TimeUnit.SECONDS);
        } finally { unregisterReceiver(receiver); }
    }

    private void startServerService() {
        Intent intent = new Intent(this, ServerService.class).setAction(ServerService.ACTION_START);
        if (Build.VERSION.SDK_INT >= 26) startForegroundService(intent); else startService(intent);
        runOnUiThread(() -> service.setText("Stop local server"));
    }
    @Override protected void onDestroy() {
        healthHandler.removeCallbacksAndMessages(null);
        unregisterReceiver(serverExitedReceiver);
        synchronized (serverPollLock) {
            activityDestroyed = true;
            serverPollHandler.removeCallbacksAndMessages(null);
        }
        cancelPairingProcess();
        cancelAdbSetup();
        Shizuku.removeRequestPermissionResultListener(shizukuPermissionListener);
        Shizuku.removeBinderDeadListener(shizukuBinderDeadListener);
        if (shizukuArgs != null && shizukuConnection != null) {
            AtomicBoolean teardownExpected = shizukuTeardownExpected;
            if (teardownExpected != null) teardownExpected.set(true);
            try { Shizuku.unbindUserService(shizukuArgs, shizukuConnection, true); } catch (Exception ignored) { }
        }
        io.shutdownNow();
        super.onDestroy();
    }
}
