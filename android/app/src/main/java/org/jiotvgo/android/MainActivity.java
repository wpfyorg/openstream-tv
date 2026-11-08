package org.jiotvgo.android;

import android.app.*;
import android.Manifest;
import android.content.*;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.content.res.Configuration;
import android.content.BroadcastReceiver;
import android.content.IntentFilter;
import android.text.InputFilter;
import android.text.InputType;
import android.provider.MediaStore;
import android.system.Os;
import android.util.TypedValue;
import android.view.*;
import android.view.inputmethod.InputMethodManager;
import android.widget.*;
import org.json.JSONObject;
import org.json.JSONArray;
import java.io.*;
import java.net.*;
import java.net.ConnectException;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class MainActivity extends Activity {
    private static final int PICK_CONFIG = 21;
    private static final int PICK_STORE = 22;
    private static final int CREATE_CONFIG = 23;
    private static final int STORAGE_PERMISSION = 24;
    private final ExecutorService io = Executors.newSingleThreadExecutor();
    private EditText number, otp;
    private TextView status, playlist, serverStatus, otpLabel;
    private Button send, verify, service, bootPermission, adbButton, shizukuButton, bootAutoStartButton;
    private Button copyButton, openButton, importConfigButton, exportConfigButton, importStoreButton, savePlaylistButton;
    private Button tvJioTvProvider, tvPlusProvider;
    private String playlistUrl = "", keyedPrefix = "";
    private boolean television;
    private RadioButton jiotvProvider, phoneTvPlusProvider;
    private boolean tvPlusSelected;
    private int pendingFileRequest = -1;
    private boolean pendingPlaylistSave;
    private boolean pendingConfigExport;
    private float tvScaleX = 1f, tvScaleY = 1f, tvScale = 1f;
    private ShizukuSetup shizukuSetup;
    private View[][] tvLeftRows, tvRightRows;
    private View lastTvFocus;
    private EditText editingField;
    private boolean consumeBackKeyUp;
    private boolean loginInFlight, otpRequested, otpPendingTvPlus;
    private String otpPendingNumber = "";

    @Override public void onCreate(Bundle b) {
        super.onCreate(b);
        render();
        shizukuSetup = new ShizukuSetup(this, this::say);
        boolean running = isServiceRunning();
        setServiceButtonState(running);
        updateServerStatus(running);
        if (ServerService.isBootStartEnabled(this) && !running) {
            startServerService();
            pollServer(0);
        }
        if (television) bootAutoStartButton.post(bootAutoStartButton::requestFocus);
        refreshUrl();
    }
    private void render() {
        television = (getResources().getConfiguration().uiMode & Configuration.UI_MODE_TYPE_MASK) == Configuration.UI_MODE_TYPE_TELEVISION;
        if (television) renderTv();
        else renderPhone();
    }
    private void renderPhone() {
        int inset = dp(24);
        ScrollView scroll = new ScrollView(this); scroll.setFillViewport(true); scroll.setDescendantFocusability(ViewGroup.FOCUS_AFTER_DESCENDANTS); scroll.setSmoothScrollingEnabled(true); LinearLayout root = new LinearLayout(this); root.setOrientation(LinearLayout.VERTICAL); root.setPadding(inset, dp(20), inset, dp(20)); root.setFocusableInTouchMode(true); scroll.addView(root); scroll.setBackgroundColor(0xff101827);
        addText(root, "Openstream TV Slim", 28, 0xff43d9b0); status = addText(root, "Starting…", 15, 0xffc5d0dd);
        LinearLayout columns = new LinearLayout(this);
        columns.setOrientation(LinearLayout.VERTICAL);
        root.addView(columns, spacedParams(0, 0));
        LinearLayout left = column(); LinearLayout right = column();
        columns.addView(left); columns.addView(right);

        addText(left, "Server and provider", 20, 0xffffffff);
        bootAutoStartButton = button(left, "Auto-start: ON"); bootAutoStartButton.setOnClickListener(v -> toggleAutoStart());
        service = button(left, "Start local server"); service.setOnClickListener(v -> toggleService());
        bootPermission = button(left, "Enable startup screen (Android 10+)"); bootPermission.setOnClickListener(v -> requestBootPermission());
        RadioGroup providers = new RadioGroup(this); providers.setOrientation(RadioGroup.HORIZONTAL); left.addView(providers, spacedParams(0, 0));
        tvPlusSelected = ServerService.isTvPlusEnabled(this);
        jiotvProvider = providerChoice("JioTV", !tvPlusSelected);
        phoneTvPlusProvider = providerChoice("JioTV+  ·  Beta", tvPlusSelected);
        providers.addView(jiotvProvider); providers.addView(phoneTvPlusProvider);
        jiotvProvider.setOnClickListener(v -> selectProvider(false));
        phoneTvPlusProvider.setOnClickListener(v -> selectProvider(true));
        addText(left, "Only one provider is active at a time.", 13, 0xff91a0b3);
        addText(left, "Playlist URL", 20, 0xffffffff);
        playlist = addText(left, "Start the server to get your keyed link.", 14, 0xffc5d0dd);
        LinearLayout actions = new LinearLayout(this); actions.setOrientation(LinearLayout.HORIZONTAL); left.addView(actions, spacedParams(0, 0));
        copyButton = button(actions, "Copy"); copyButton.setOnClickListener(v -> copyUrl());
        savePlaylistButton = button(actions, "Save M3U"); savePlaylistButton.setOnClickListener(v -> savePlaylist());
        openButton = button(left, "Open in IPTV app"); openButton.setOnClickListener(v -> openUrl());

        addText(right, "JioTV login", 20, 0xffffffff);
        number = field(right, "10-digit mobile number"); number.setInputType(InputType.TYPE_CLASS_NUMBER); number.setFilters(new InputFilter[]{new InputFilter.LengthFilter(10)});
        otp = field(right, "6-digit OTP"); otp.setInputType(InputType.TYPE_CLASS_NUMBER); otp.setFilters(new InputFilter[]{new InputFilter.LengthFilter(6)});
        LinearLayout loginActions = new LinearLayout(this); loginActions.setOrientation(LinearLayout.HORIZONTAL); right.addView(loginActions, spacedParams(0, 0));
        send = button(loginActions, "Send OTP"); send.setOnClickListener(v -> otpRequest(false));
        verify = button(loginActions, "Login"); verify.setOnClickListener(v -> otpRequest(true));
        addText(right, "Import", 20, 0xffffffff);
        importConfigButton = button(right, "Import TOML config"); importConfigButton.setOnClickListener(v -> pickFile("application/toml", PICK_CONFIG));
        exportConfigButton = button(right, "Export TOML config"); exportConfigButton.setOnClickListener(v -> createConfigDocument());
        importStoreButton = button(right, "Import login store backup"); importStoreButton.setOnClickListener(v -> pickFile("application/toml", PICK_STORE));
        setOtpStepVisible(false);
        updateAutoStartControls();
        setContentView(scroll);
    }
    private void renderTv() {
        getWindow().setStatusBarColor(Color.rgb(11, 18, 32));
        getWindow().setNavigationBarColor(Color.rgb(11, 18, 32));
        getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);
        android.util.DisplayMetrics metrics = getResources().getDisplayMetrics();
        tvScaleX = metrics.widthPixels / 1920f;
        tvScaleY = metrics.heightPixels / 1080f;
        tvScale = Math.min(tvScaleX, tvScaleY);
        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(Color.rgb(11, 18, 32));
        root.setFocusable(false);
        root.setFocusableInTouchMode(false);

        TextView mark = tvText("▶", 27, 0xff092820, true); mark.setGravity(Gravity.CENTER); mark.setBackground(roundRect(0xff42d7ae, 14, 0, 0)); place(root, mark, 70, 54, 52, 52);
        place(root, tvText("Openstream TV Slim", 36, 0xfff5f9fc, true), 140, 50, 700, 48);
        place(root, tvText("Your local TV streaming companion", 20, 0xff91a3b7, false), 141, 101, 700, 32);
        serverStatus = tvText("●   SERVER STOPPED", 19, 0xffffa3a3, true); serverStatus.setGravity(Gravity.CENTER); serverStatus.setBackground(roundRect(0xff341e28, 25, 0xff934254, 2)); place(root, serverStatus, 1515, 61, 325, 50);
        place(root, tvDivider(), 70, 145, 1780, 2);

        place(root, tvText("Local server", 29, 0xfff4f8fc, true), 70, 180, 500, 40);
        place(root, tvText("Start the server and manage startup access.", 20, 0xff96a8ba, false), 70, 222, 700, 30);
        bootAutoStartButton = tvButton("Auto-start: ON"); place(root, bootAutoStartButton, 555, 181, 355, 62); bootAutoStartButton.setOnClickListener(v -> toggleAutoStart());
        service = tvButton("▶   Start local server"); place(root, service, 70, 270, 840, 68); service.setOnClickListener(v -> toggleService());
        adbButton = tvButton("Setup using ADB"); place(root, adbButton, 70, 352, 410, 62); adbButton.setOnClickListener(v -> showAdbSetup());
        shizukuButton = tvButton("Setup using Shizuku"); place(root, shizukuButton, 500, 352, 410, 62); shizukuButton.setOnClickListener(v -> shizukuSetup.setup());
        bootPermission = tvButton("Enable startup screen"); place(root, bootPermission, 70, 428, 840, 62); bootPermission.setOnClickListener(v -> requestBootPermission());

        place(root, tvText("Provider", 27, 0xfff4f8fc, true), 70, 530, 400, 38);
        place(root, tvText("Choose the service used for OTP login.", 19, 0xff96a8ba, false), 70, 568, 700, 28);
        tvPlusSelected = ServerService.isTvPlusEnabled(this);
        tvJioTvProvider = tvProviderButton(tvPlusSelected ? "○   JioTV" : "●   JioTV", !tvPlusSelected); place(root, tvJioTvProvider, 70, 606, 410, 62);
        tvPlusProvider = tvProviderButton(tvPlusSelected ? "●   JioTV+  ·  Beta" : "○   JioTV+  ·  Beta", tvPlusSelected); place(root, tvPlusProvider, 500, 606, 410, 62);
        tvJioTvProvider.setOnClickListener(v -> selectProvider(false));
        tvPlusProvider.setOnClickListener(v -> selectProvider(true));

        place(root, tvText("Playlist", 27, 0xfff4f8fc, true), 70, 716, 400, 38);
        playlist = tvText("Start the server to get your playlist URL", 20, 0xff8ea0b2, false); playlist.setGravity(Gravity.CENTER_VERTICAL); playlist.setPadding(tvX(20), 0, tvX(20), 0); playlist.setBackground(roundRect(0xff131f2e, 10, 0xff304052, 2)); place(root, playlist, 70, 758, 840, 60);
        copyButton = tvButton("Copy"); place(root, copyButton, 70, 837, 267, 60); copyButton.setOnClickListener(v -> copyUrl());
        savePlaylistButton = tvButton("Save M3U"); place(root, savePlaylistButton, 353, 837, 267, 60); savePlaylistButton.setOnClickListener(v -> savePlaylist());
        openButton = tvButton("Open in IPTV app"); place(root, openButton, 636, 837, 274, 60); openButton.setOnClickListener(v -> openUrl());

        place(root, tvDivider(), 958, 181, 2, 755);
        place(root, tvText("Provider login", 29, 0xfff4f8fc, true), 1010, 180, 600, 40);
        place(root, tvText("Sign in with the provider selected on the left.", 20, 0xff96a8ba, false), 1010, 222, 800, 30);
        place(root, tvText("Mobile number", 19, 0xffc7d4df, true), 1010, 270, 400, 28);
        number = tvField("Enter 10-digit mobile number"); place(root, number, 1010, 303, 840, 62); number.setInputType(InputType.TYPE_CLASS_NUMBER); number.setFilters(new InputFilter[]{new InputFilter.LengthFilter(10)});
        otpLabel = tvText("One-time password", 19, 0xffc7d4df, true); place(root, otpLabel, 1010, 391, 400, 28);
        otp = tvField("Enter 6-digit OTP"); place(root, otp, 1010, 424, 840, 62); otp.setInputType(InputType.TYPE_CLASS_NUMBER); otp.setFilters(new InputFilter[]{new InputFilter.LengthFilter(6)});
        send = tvButton("Send OTP"); place(root, send, 1010, 511, 410, 62); send.setOnClickListener(v -> otpRequest(false));
        verify = tvButton("Login"); place(root, verify, 1440, 511, 410, 62); verify.setOnClickListener(v -> otpRequest(true));

        place(root, tvText("Configuration & backup", 27, 0xfff4f8fc, true), 1010, 624, 600, 38);
        place(root, tvText("Import or export your configuration and login store.", 19, 0xff96a8ba, false), 1010, 662, 800, 28);
        importConfigButton = tvButton("Import TOML config"); place(root, importConfigButton, 1010, 703, 410, 62); importConfigButton.setOnClickListener(v -> pickFile("application/toml", PICK_CONFIG));
        exportConfigButton = tvButton("Export TOML config"); place(root, exportConfigButton, 1440, 703, 410, 62); exportConfigButton.setOnClickListener(v -> createConfigDocument());
        importStoreButton = tvButton("Import login store backup"); place(root, importStoreButton, 1010, 781, 840, 62); importStoreButton.setOnClickListener(v -> pickFile("application/toml", PICK_STORE));
        status = tvText("", 18, 0xff91a7b5, false); status.setGravity(Gravity.TOP | Gravity.LEFT); place(root, status, 1010, 872, 840, 90);
        place(root, tvDivider(), 70, 1000, 1780, 2);

        setOtpStepVisible(false);
        updateAutoStartControls();
        setContentView(root);
        configureTvFocus(root);
    }
    private void requestBootPermission() {
        if (Build.VERSION.SDK_INT < 29 || android.provider.Settings.canDrawOverlays(this)) {
            say("Startup screen permission enabled. Start the server to enable boot startup.");
            return;
        }
        try {
            startActivity(new Intent(android.provider.Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:" + getPackageName())));
        } catch (ActivityNotFoundException e) {
            say("Use ADB: adb shell appops set " + getPackageName() + " SYSTEM_ALERT_WINDOW allow");
        }
    }
    private void configureTvFocus(ViewGroup root) {
        // Rows represent actual controls and semantic peers. Hidden/disabled buttons are
        // skipped at navigation time, so changing the login step or auto-start is safe.
        tvLeftRows = new View[][] {
                {bootAutoStartButton}, {service}, {adbButton, shizukuButton},
                {bootPermission}, {tvJioTvProvider, tvPlusProvider},
                {copyButton, savePlaylistButton, openButton}
        };
        tvRightRows = new View[][] {
                {number}, {otp}, {send, verify},
                {importConfigButton, exportConfigButton}, {importStoreButton}
        };
        for (View[][] column : new View[][][]{tvLeftRows, tvRightRows}) {
            for (View[] row : column) for (View item : row) {
                item.setOnFocusChangeListener((v, focused) -> {
                    if (v == tvJioTvProvider || v == tvPlusProvider)
                        applyProviderStyle((Button)v, Boolean.TRUE.equals(v.getTag()), focused);
                    if (focused) {
                        lastTvFocus = v;
                        v.post(() -> v.requestRectangleOnScreen(
                                new android.graphics.Rect(0, 0, v.getWidth(), v.getHeight()), false));
                    } else if (v == editingField) {
                        finishTvFieldEditing();
                    }
                });
                if (item instanceof EditText) {
                    EditText field = (EditText)item;
                    field.setShowSoftInputOnFocus(false);
                    // Remote arrows navigate; OK explicitly enters editing mode.
                    field.setOnEditorActionListener((v, action, event) -> {
                        finishTvFieldEditing();
                        View below = tvNeighbor(field, View.FOCUS_DOWN);
                        if (below != null) below.requestFocus();
                        return true;
                    });
                }
            }
        }
    }

    private boolean tvSelectable(View view) {
        return view != null && view.getVisibility() == View.VISIBLE
                && view.isShown() && view.isEnabled() && view.isFocusable()
                && (!(view instanceof Button) || view.isClickable());
    }

    private int tvColumn(View view) {
        for (View[] row : tvLeftRows) for (View item : row) if (view == item) return 0;
        for (View[] row : tvRightRows) for (View item : row) if (view == item) return 1;
        return -1;
    }

    private int tvCenter(View view, boolean vertical) {
        return vertical ? view.getTop() + view.getHeight() / 2 : view.getLeft() + view.getWidth() / 2;
    }

    private View closestTvView(View[][] rows, View origin, boolean compareVertical) {
        View best = null;
        int bestDistance = Integer.MAX_VALUE;
        for (View[] row : rows) for (View candidate : row) {
            if (!tvSelectable(candidate)) continue;
            int distance = Math.abs(tvCenter(candidate, compareVertical) - tvCenter(origin, compareVertical));
            if (distance < bestDistance) { bestDistance = distance; best = candidate; }
        }
        return best;
    }

    private View tvNeighbor(View from, int direction) {
        if (tvLeftRows == null || tvRightRows == null) return null;
        int column = tvColumn(from);
        if (column < 0) return null; // Dialogs and the system keyboard own their own focus.
        View[][] rows = column == 0 ? tvLeftRows : tvRightRows;
        for (int rowIndex = 0; rowIndex < rows.length; rowIndex++) {
            View[] row = rows[rowIndex];
            for (int index = 0; index < row.length; index++) {
                if (row[index] != from) continue;
                if (direction == View.FOCUS_LEFT || direction == View.FOCUS_RIGHT) {
                    int step = direction == View.FOCUS_RIGHT ? 1 : -1;
                    for (int next = index + step; next >= 0 && next < row.length; next += step)
                        if (tvSelectable(row[next])) return row[next];
                    if (direction == View.FOCUS_RIGHT && column == 0)
                        return closestTvView(tvRightRows, from, true);
                    if (direction == View.FOCUS_LEFT && column == 1)
                        return closestTvView(tvLeftRows, from, true);
                    return null;
                }
                int step = direction == View.FOCUS_DOWN ? 1 : -1;
                for (int next = rowIndex + step; next >= 0 && next < rows.length; next += step) {
                    View candidate = closestTvView(new View[][]{rows[next]}, from, false);
                    if (candidate != null) return candidate;
                }
                return null;
            }
        }
        return null;
    }

    private void startTvFieldEditing(EditText field) {
        editingField = field;
        field.setShowSoftInputOnFocus(true);
        field.requestFocus();
        field.setSelection(field.getText().length());
        field.post(() -> ((InputMethodManager)getSystemService(INPUT_METHOD_SERVICE))
                .showSoftInput(field, InputMethodManager.SHOW_IMPLICIT));
    }

    private void finishTvFieldEditing() {
        if (editingField == null) return;
        EditText field = editingField;
        editingField = null;
        ((InputMethodManager)getSystemService(INPUT_METHOD_SERVICE))
                .hideSoftInputFromWindow(field.getWindowToken(), 0);
        field.setShowSoftInputOnFocus(false);
    }

    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        if (!television || tvLeftRows == null || tvRightRows == null) return super.dispatchKeyEvent(event);
        if (event.getKeyCode() == KeyEvent.KEYCODE_BACK) {
            if (event.getAction() == KeyEvent.ACTION_UP && consumeBackKeyUp) { consumeBackKeyUp = false; return true; }
            if (event.getAction() == KeyEvent.ACTION_DOWN && editingField != null) {
                finishTvFieldEditing(); consumeBackKeyUp = true; return true;
            }
        }
        View focused = getCurrentFocus();
        if (tvColumn(focused) < 0 || editingField != null) return super.dispatchKeyEvent(event);
        int code = event.getKeyCode();
        if (focused instanceof EditText && (code == KeyEvent.KEYCODE_DPAD_CENTER || code == KeyEvent.KEYCODE_ENTER)) {
            if (event.getAction() == KeyEvent.ACTION_UP) startTvFieldEditing((EditText)focused);
            return true;
        }
        int direction;
        switch (code) {
            case KeyEvent.KEYCODE_DPAD_UP: direction = View.FOCUS_UP; break;
            case KeyEvent.KEYCODE_DPAD_DOWN: direction = View.FOCUS_DOWN; break;
            case KeyEvent.KEYCODE_DPAD_LEFT: direction = View.FOCUS_LEFT; break;
            case KeyEvent.KEYCODE_DPAD_RIGHT: direction = View.FOCUS_RIGHT; break;
            default: return super.dispatchKeyEvent(event);
        }
        if (event.getAction() == KeyEvent.ACTION_DOWN) {
            View next = tvNeighbor(focused, direction);
            if (next != null) next.requestFocus();
        }
        return true;
    }

    @Override public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus && television && lastTvFocus != null && tvSelectable(lastTvFocus)
                && tvColumn(getCurrentFocus()) < 0)
            lastTvFocus.post(lastTvFocus::requestFocus);
    }
    private LinearLayout column() { LinearLayout c = new LinearLayout(this); c.setOrientation(LinearLayout.VERTICAL); return c; }
    private RadioButton providerChoice(String label, boolean checked) { RadioButton b = new RadioButton(this); b.setId(View.generateViewId()); b.setText(label); b.setTextColor(0xfff4f8fc); b.setTextSize(14); b.setChecked(checked); b.setFocusable(true); return b; }
    private void pickFile(String type, int request) {
        if (television) {
            if (Build.VERSION.SDK_INT >= 23 && Build.VERSION.SDK_INT <= 32 && checkSelfPermission(Manifest.permission.READ_EXTERNAL_STORAGE) != PackageManager.PERMISSION_GRANTED) {
                pendingFileRequest = request;
                requestPermissions(new String[]{Manifest.permission.READ_EXTERNAL_STORAGE}, STORAGE_PERMISSION);
                return;
            }
            launchTvPicker(request);
            return;
        }
        Intent document = new Intent(Intent.ACTION_OPEN_DOCUMENT); document.setType("*/*"); document.addCategory(Intent.CATEGORY_OPENABLE);
        try { startActivityForResult(document, request); }
        catch (ActivityNotFoundException missingPicker) {
            Intent fallback = new Intent(Intent.ACTION_GET_CONTENT); fallback.setType("*/*"); fallback.addCategory(Intent.CATEGORY_OPENABLE);
            try { startActivityForResult(fallback, request); }
            catch (ActivityNotFoundException noFallback) { say("Install a document/file picker to import files on this TV."); }
        }
    }
    private void launchTvPicker(int request) { Intent picker = new Intent(this, TvFilePickerActivity.class); picker.putExtra(TvFilePickerActivity.EXTRA_TITLE, request == PICK_CONFIG ? "Import configuration" : "Import login store backup"); startActivityForResult(picker, request); }
    private void createConfigDocument() { Intent i = new Intent(Intent.ACTION_CREATE_DOCUMENT); i.addCategory(Intent.CATEGORY_OPENABLE); i.setType("application/octet-stream"); i.putExtra(Intent.EXTRA_TITLE, "config.toml"); try { startActivityForResult(i, CREATE_CONFIG); } catch (ActivityNotFoundException e) { if (television) exportConfigFallback(); else say("Install a document/file picker to export config."); } }
    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
    private LinearLayout.LayoutParams spacedParams(int top, int bottom) { LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-1, -2); lp.topMargin = dp(top); lp.bottomMargin = dp(bottom); return lp; }
    private TextView addText(LinearLayout p, String s, int size, int color) { TextView t = new TextView(this); t.setText(s); t.setTextSize(size); t.setTextColor(color); t.setPadding(0, dp(10), 0, dp(10)); p.addView(t, spacedParams(4, 4)); return t; }
    private EditText field(LinearLayout p, String hint) { EditText e = new EditText(this); e.setId(View.generateViewId()); e.setSingleLine(true); e.setHint(hint); e.setTextColor(0xffffffff); e.setHintTextColor(0xff91a0b3); e.setTextSize(16); e.setPadding(dp(16), dp(8), dp(16), dp(8)); e.setBackgroundResource(org.jiotvgo.android.R.drawable.tv_field_background); p.addView(e, spacedParams(4, 4)); return e; }
    private Button button(LinearLayout p, String label) { Button b = new Button(this); b.setId(View.generateViewId()); b.setText(label); b.setAllCaps(false); b.setTextSize(14); b.setTextColor(getResources().getColorStateList(org.jiotvgo.android.R.color.tv_button_text)); b.setMinHeight(dp(48)); b.setPadding(dp(20), dp(8), dp(20), dp(8)); b.setBackgroundResource(org.jiotvgo.android.R.drawable.tv_button_background); b.setFocusable(true); b.setFocusableInTouchMode(false); LinearLayout.LayoutParams lp = p.getOrientation() == LinearLayout.HORIZONTAL ? new LinearLayout.LayoutParams(0, -2, 1) : spacedParams(4, 4); if (p.getOrientation() == LinearLayout.HORIZONTAL) { lp.leftMargin = dp(4); lp.rightMargin = dp(4); } p.addView(b, lp); return b; }
    private int tvX(int value) { return Math.round(value * tvScaleX); }
    private int tvY(int value) { return Math.round(value * tvScaleY); }
    private void place(FrameLayout root, View view, int x, int y, int width, int height) { FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(tvX(width), tvY(height)); lp.leftMargin = tvX(x); lp.topMargin = tvY(y); root.addView(view, lp); }
    private TextView tvText(String text, int size, int color, boolean bold) { TextView view = new TextView(this); view.setText(text); view.setTextColor(color); view.setTextSize(TypedValue.COMPLEX_UNIT_PX, size * tvScale); view.setTypeface(Typeface.create("sans-serif", bold ? Typeface.BOLD : Typeface.NORMAL)); view.setGravity(Gravity.CENTER_VERTICAL); view.setIncludeFontPadding(false); return view; }
    private View tvDivider() { View view = new View(this); view.setBackgroundColor(0xff263548); return view; }
    private GradientDrawable roundRect(int fill, int radius, int stroke, int strokeWidth) { GradientDrawable d = new GradientDrawable(); d.setColor(fill); d.setCornerRadius(tvScale * radius); if (strokeWidth > 0) d.setStroke(Math.max(1, Math.round(tvScale * strokeWidth)), stroke); return d; }
    private Button tvButton(String label) { Button b = new Button(this); b.setId(View.generateViewId()); b.setText(label); b.setAllCaps(false); b.setGravity(Gravity.CENTER_VERTICAL | Gravity.LEFT); b.setTextSize(TypedValue.COMPLEX_UNIT_PX, 23 * tvScale); b.setTypeface(Typeface.create("sans-serif", Typeface.BOLD)); b.setTextColor(getResources().getColorStateList(org.jiotvgo.android.R.color.tv_button_text)); b.setPadding(tvX(22), 0, tvX(18), 0); b.setBackgroundResource(org.jiotvgo.android.R.drawable.tv_button_background); b.setFocusable(true); b.setFocusableInTouchMode(true); if (Build.VERSION.SDK_INT >= 21) b.setStateListAnimator(null); return b; }
    private Button tvProviderButton(String label, boolean selected) { Button b = tvButton(label); b.setTag(Boolean.valueOf(selected)); applyProviderStyle(b, selected, false); b.setOnFocusChangeListener((v, focused) -> applyProviderStyle((Button)v, Boolean.TRUE.equals(v.getTag()), focused)); return b; }
    private void applyProviderStyle(Button b, boolean selected, boolean focused) { if (focused) { b.setBackground(roundRect(0xff42d7ae, 12, 0xffa7f8de, 2)); b.setTextColor(0xff071e1b); } else { b.setBackground(roundRect(0xff1b2b3b, 12, selected ? 0xff42d7ae : 0xff354459, 2)); b.setTextColor(selected ? 0xffddf9f0 : 0xffdce7ef); } }
    private EditText tvField(String hint) { EditText e = new EditText(this); e.setId(View.generateViewId()); e.setSingleLine(true); e.setHint(hint); e.setTextColor(0xfff4f8fc); e.setHintTextColor(0xff8194a7); e.setTextSize(TypedValue.COMPLEX_UNIT_PX, 21 * tvScale); e.setPadding(tvX(20), 0, tvX(20), 0); e.setBackgroundResource(org.jiotvgo.android.R.drawable.tv_field_background); e.setFocusableInTouchMode(true); return e; }
    private void selectProvider(boolean tvPlus) {
        if (tvPlusSelected == tvPlus && ServerService.isTvPlusEnabled(this) == tvPlus) return;
        tvPlusSelected = tvPlus;
        ServerService.setTvPlusEnabled(this, tvPlus);
        otpPendingNumber = "";
        setOtpStepVisible(false);
        if (jiotvProvider != null) jiotvProvider.setChecked(!tvPlus);
        if (phoneTvPlusProvider != null) phoneTvPlusProvider.setChecked(tvPlus);
        if (tvJioTvProvider != null) {
            tvJioTvProvider.setTag(Boolean.valueOf(!tvPlus));
            tvJioTvProvider.setText(tvPlus ? "○   JioTV" : "●   JioTV");
            applyProviderStyle(tvJioTvProvider, !tvPlus, tvJioTvProvider.hasFocus());
        }
        if (tvPlusProvider != null) {
            tvPlusProvider.setTag(Boolean.valueOf(tvPlus));
            tvPlusProvider.setText(tvPlus ? "●   JioTV+  ·  Beta" : "○   JioTV+  ·  Beta");
            applyProviderStyle(tvPlusProvider, tvPlus, tvPlusProvider.hasFocus());
        }
        if (!isServiceRunning()) {
            say(tvPlus ? "JioTV+ Beta selected. Start the server to use it." : "JioTV selected. Start the server to use it.");
            return;
        }
        say("Switching provider and restarting the local server…");
        io.execute(() -> {
            try {
                if (!stopServiceAndWait()) throw new IOException("server did not stop");
                startServerService();
                pollServer(0);
            } catch (Exception e) {
                say("Provider changed. Restart the local server to apply it.");
            }
        });
    }

    /// Mirrors the in-app selection into the server's single active-provider
    /// setting. The server persists it in the login store, so it survives
    /// restarts and stays consistent with a choice made in the web interface.
    private void syncActiveProvider() {
        if (keyedPrefix.isEmpty()) return;
        try {
            JSONObject body = new JSONObject();
            body.put("provider", ServerService.isTvPlusEnabled(this) ? "jiotv_plus" : "jiotv");
            post(keyedPrefix + "api/providers/activate", body);
        } catch (Exception ignored) {
            // The selection is retried the next time the server becomes ready.
        }
    }
    private void showAdbSetup() {
        say("Connecting to ADB on 127.0.0.1:5555…");
        AdbSetup.start(this, new AdbSetup.Callback() {
            @Override public void onStatus(String message) { say(message); }
            @Override public void onComplete(AdbSetup.State state, String message) { say(message); }
        });
    }
    private void say(String s) { runOnUiThread(() -> { if (status != null) status.setText(s); if (television) Toast.makeText(this, s, Toast.LENGTH_SHORT).show(); }); }
    private void updateServerStatus(boolean running) { if (serverStatus == null) return; runOnUiThread(() -> { serverStatus.setText(running ? "●   SERVER RUNNING" : "●   SERVER STOPPED"); serverStatus.setTextColor(running ? 0xffa3dfcc : 0xffffa3a3); serverStatus.setBackground(running ? roundRect(0xff182b2c, 25, 0xff30574d, 2) : roundRect(0xff341e28, 25, 0xff934254, 2)); }); }
    private void setServiceButtonState(boolean running) { if (service != null) service.setText(television ? (running ? "■   Stop local server" : "▶   Start local server") : (running ? "Stop local server" : "Start local server")); }

    private void toggleAutoStart() {
        boolean enabled = !ServerService.isBootStartEnabled(this);
        if (!ServerService.setBootStartEnabled(this, enabled)) {
            say("Could not save the auto-start setting. Try again.");
            return;
        }
        updateAutoStartControls();
        if (enabled && !isServiceRunning()) { startServerService(); pollServer(0); }
        say(enabled ? "Auto-start enabled. Server will start on boot." :
                "Auto-start disabled. Manual server and setup controls available.");
        bootAutoStartButton.requestFocus();
    }

    private void updateAutoStartControls() {
        if (bootAutoStartButton == null) return;
        boolean enabled = ServerService.isBootStartEnabled(this);
        bootAutoStartButton.setText("Auto-start: " + (enabled ? "ON" : "OFF"));
        int manualVisibility = enabled ? View.GONE : View.VISIBLE;
        if (service != null) service.setVisibility(manualVisibility);
        if (adbButton != null) adbButton.setVisibility(manualVisibility);
        if (shizukuButton != null) shizukuButton.setVisibility(manualVisibility);
        if (television && lastTvFocus != null && !tvSelectable(lastTvFocus))
            bootAutoStartButton.requestFocus();
    }

    private void setOtpStepVisible(boolean visible) {
        otpRequested = visible;
        if (otpLabel != null) otpLabel.setVisibility(visible ? View.VISIBLE : View.GONE);
        if (otp != null) otp.setVisibility(visible ? View.VISIBLE : View.GONE);
        if (verify != null) verify.setVisibility(visible ? View.VISIBLE : View.GONE);
        if (television && send != null) {
            FrameLayout.LayoutParams params = (FrameLayout.LayoutParams)send.getLayoutParams();
            params.topMargin = tvY(visible ? 511 : 424);
            params.width = tvX(visible ? 410 : 840);
            send.setLayoutParams(params);
        }
        if (visible && television && otp != null) otp.post(otp::requestFocus);
    }

    private void markOtpSent(String mobile, boolean tvPlus) {
        runOnUiThread(() -> {
            otpPendingNumber = mobile;
            otpPendingTvPlus = tvPlus;
            setOtpStepVisible(true);
            say(tvPlus ? "JioTV+ OTP sent. Enter 6 digits and select Login." :
                    "OTP sent. Enter 6 digits and select Login.");
        });
    }

    private void toggleService() {
        if (isServiceRunning()) { startService(new Intent(this, ServerService.class).setAction(ServerService.ACTION_STOP)); setServiceButtonState(false); updateServerStatus(false); say("Server stopped"); }
        else { Intent i = new Intent(this, ServerService.class).setAction(ServerService.ACTION_START); if (Build.VERSION.SDK_INT >= 26) startForegroundService(i); else startService(i); setServiceButtonState(true); say("Starting server…"); pollServer(0); }
    }
    private boolean isServiceRunning() { android.app.ActivityManager m = (android.app.ActivityManager)getSystemService(ACTIVITY_SERVICE); for (android.app.ActivityManager.RunningServiceInfo i:m.getRunningServices(Integer.MAX_VALUE)) if (ServerService.class.getName().equals(i.service.getClassName())) return true; return false; }
    private void showPlaylistUrl() { runOnUiThread(() -> playlist.setText(PlaylistDisplay.masked(playlistUrl))); }
    private void refreshUrl() { io.execute(() -> { try { String path = runKeyShow(); playlistUrl=PlaylistLink.loopbackUrl(path); boolean running=serverHealthy(); showPlaylistUrl(); updateServerStatus(running); runOnUiThread(() -> setServiceButtonState(running)); if (running) syncActiveProvider(); say(running ? "Server is running on this device. Log in to load channels." : "Server is stopped. Start it to log in and stream."); } catch (Exception e) { android.util.Log.e("JioTVGo", "Cannot initialize local playlist", e); updateServerStatus(false); say("Could not read playlist key. Check the bundled server and imported config."); } }); }
    private void pollServer(int attempt) { io.execute(() -> { boolean running=false; try { String path=runKeyShow(); playlistUrl=PlaylistLink.loopbackUrl(path); showPlaylistUrl(); running=serverHealthy(); } catch(Exception ignored) {} updateServerStatus(running); if(running) { syncActiveProvider(); runOnUiThread(() -> setServiceButtonState(true)); say("Server is running. Log in to load channels."); return; } if(attempt<12) { new android.os.Handler(getMainLooper()).postDelayed(() -> pollServer(attempt+1),750); } else { runOnUiThread(() -> setServiceButtonState(false)); say("Server did not become ready. Check port 5001, config, and the APK ABI."); } }); }
    private boolean serverHealthy() throws Exception { HttpURLConnection c=(HttpURLConnection)new URL("http://127.0.0.1:5001"+keyedPrefix+"api/status").openConnection(); c.setConnectTimeout(1200); c.setReadTimeout(1800); c.setRequestMethod("GET"); try { int code=c.getResponseCode(); if(code!=200) return false; ByteArrayOutputStream out=new ByteArrayOutputStream(); try(InputStream in=c.getInputStream()){ byte[] buf=new byte[1024]; int n; while((n=in.read(buf))!=-1) out.write(buf,0,n); } return new JSONObject(out.toString("UTF-8")).has("jiotv"); } catch (ConnectException e) { return false; } finally { c.disconnect(); } }
    private String runKeyShow() throws Exception { File binary=new File(getApplicationInfo().nativeLibraryDir,"libjiotv_exec.so"); java.util.List<String> command=new java.util.ArrayList<>(); command.add(binary.getAbsolutePath()); command.add("--skip-update-check"); File config=new File(getFilesDir(),"config.toml"); if(config.isFile()) { command.add("--config"); command.add(config.getAbsolutePath()); } command.add("key"); command.add("show"); ProcessBuilder b=new ProcessBuilder(command); b.directory(getFilesDir()); b.redirectErrorStream(true); b.environment().put("JIOTV_PATH_PREFIX",new File(getFilesDir(),"data").getAbsolutePath()); b.environment().put("HOME",getFilesDir().getAbsolutePath()); Process p=b.start(); ByteArrayOutputStream out=new ByteArrayOutputStream(); byte[] buf=new byte[256]; int n; while((n=p.getInputStream().read(buf))!=-1) out.write(buf,0,n); if(p.waitFor()!=0) throw new IOException("key command failed"); String result=out.toString("UTF-8").trim(); keyedPrefix=PlaylistLink.keyedPrefix(result); return result; }

    private void otpRequest(boolean verifyRequest) {
        String n = number.getText().toString().trim();
        String code = otp.getText().toString().trim();
        if (!n.matches("[0-9]{10}")) {
            say("Enter exactly 10 digits for the mobile number.");
            return;
        }
        if (verifyRequest && (!otpRequested || !n.equals(otpPendingNumber) || tvPlusSelected != otpPendingTvPlus)) {
            say("Send OTP for the selected provider and mobile number first.");
            return;
        }
        if (verifyRequest && !code.matches("[0-9]{6}")) {
            say("Enter exactly 6 digits for the OTP.");
            return;
        }
        if (loginInFlight) return;
        loginInFlight = true;
        send.setClickable(false);
        verify.setClickable(false);
        if (!verifyRequest) {
            otpPendingNumber = "";
            otp.setText("");
            setOtpStepVisible(false);
        }
        say(verifyRequest ? "Verifying OTP…" : "Requesting OTP…");
        io.execute(() -> {
            try {
                if (keyedPrefix.isEmpty()) runKeyShow();
                if (tvPlusSelected) {
                    if (verifyRequest) verifyTvPlusOtp(n, code);
                    else requestTvPlusOtp(n);
                } else {
                    JSONObject body = new JSONObject();
                    body.put("number", n);
                    if (verifyRequest) body.put("otp", code);
                    JSONObject response = post(keyedPrefix + "login/" + (verifyRequest ? "verifyOTP" : "sendOTP"), body);
                    if (verifyRequest && "success".equals(response.optString("status"))) say("Login complete. Playlist is ready.");
                    else if (!verifyRequest && response.optBoolean("status", false)) markOtpSent(n, false);
                    else say(responseMessage(response, verifyRequest ? "OTP verification failed." : "OTP could not be sent."));
                }
            } catch (Exception e) {
                say("Could not complete login. Check the local server and your connection.");
            } finally {
                runOnUiThread(() -> { loginInFlight = false; send.setClickable(true); verify.setClickable(true); });
            }
        });
    }
    private void requestTvPlusOtp(String numberValue) throws Exception {
        JSONObject body = new JSONObject();
        body.put("number", numberValue);
        JSONObject response = post(keyedPrefix + "extras/login/sendOTP", body);
        if (response.optBoolean("status", false)) {
            markOtpSent(numberValue, true);
            return;
        }
        JSONArray connections = response.optJSONArray("connections");
        if (connections == null || connections.length() == 0) {
            say(responseMessage(response, "JioTV+ OTP could not be sent."));
            return;
        }
        if (connections.length() == 1) {
            JSONObject onlyConnection = connections.optJSONObject(0);
            sendTvPlusConnectionOtp(numberValue, onlyConnection == null ? 0 : onlyConnection.optInt("index", 0));
            return;
        }
        String[] labels = new String[connections.length()];
        int[] indexes = new int[connections.length()];
        for (int i = 0; i < connections.length(); i++) {
            JSONObject connection = connections.optJSONObject(i);
            indexes[i] = connection == null ? i : connection.optInt("index", i);
            String name = connection == null ? "Connection " + (i + 1) : connection.optString("name", "Connection " + (i + 1));
            String product = connection == null ? "" : connection.optString("product", "");
            String tail = connection == null ? "" : connection.optString("lineEndsWith", "");
            labels[i] = name + (product.isEmpty() ? "" : " · " + product) + (tail.isEmpty() ? "" : " · ••••" + tail);
        }
        runOnUiThread(() -> new AlertDialog.Builder(this)
                .setTitle("Choose JioTV+ connection")
                .setItems(labels, (dialog, which) -> sendTvPlusConnectionOtpAsync(numberValue, indexes[which]))
                .setNegativeButton("Cancel", null)
                .show());
    }
    private void sendTvPlusConnectionOtpAsync(String numberValue, int index) {
        if (loginInFlight) return;
        loginInFlight = true;
        send.setClickable(false);
        verify.setClickable(false);
        say("Requesting JioTV+ OTP…");
        io.execute(() -> {
            try { sendTvPlusConnectionOtp(numberValue, index); }
            catch (Exception e) { say("JioTV+ OTP could not be sent. Try again."); }
            finally { runOnUiThread(() -> { loginInFlight = false; send.setClickable(true); verify.setClickable(true); }); }
        });
    }
    private void sendTvPlusConnectionOtp(String numberValue, int index) throws Exception {
        JSONObject body = new JSONObject();
        body.put("number", numberValue);
        body.put("connection", index);
        JSONObject response = post(keyedPrefix + "extras/login/sendOTP", body);
        if (response.optBoolean("status", false)) markOtpSent(numberValue, true);
        else say(responseMessage(response, "JioTV+ OTP could not be sent."));
    }
    private void verifyTvPlusOtp(String numberValue, String code) throws Exception {
        JSONObject body = new JSONObject();
        body.put("number", numberValue);
        body.put("otp", code);
        JSONObject response = post(keyedPrefix + "extras/login/verifyOTP", body);
        if (response.optBoolean("status", false)) say("JioTV+ login complete. Playlist is ready.");
        else say(responseMessage(response, "JioTV+ OTP verification failed."));
    }
    private String responseMessage(JSONObject response, String fallback) {
        String message = response.optString("message", "");
        if (message.isEmpty()) message = response.optString("error", "");
        return message.isEmpty() ? fallback : message;
    }
    private JSONObject post(String path, JSONObject data) throws Exception { HttpURLConnection c=(HttpURLConnection)new URL("http://127.0.0.1:5001"+path).openConnection(); c.setRequestMethod("POST"); c.setConnectTimeout(8000); c.setReadTimeout(20000); c.setDoOutput(true); c.setRequestProperty("Content-Type","application/json"); try(OutputStream o=c.getOutputStream()){o.write(data.toString().getBytes(StandardCharsets.UTF_8));} InputStream in=c.getResponseCode()<400?c.getInputStream():c.getErrorStream(); ByteArrayOutputStream out=new ByteArrayOutputStream(); byte[] buf=new byte[512]; int n; while((n=in.read(buf))!=-1) out.write(buf,0,n); return new JSONObject(out.toString("UTF-8")); }
    private void copyUrl() { if(playlistUrl.isEmpty()) return; ((android.content.ClipboardManager)getSystemService(CLIPBOARD_SERVICE)).setPrimaryClip(ClipData.newPlainText("JioTV playlist",playlistUrl)); say("Playlist URL copied"); }
    private void openUrl() { if(playlistUrl.isEmpty()) return; Intent i=new Intent(Intent.ACTION_VIEW); i.setDataAndType(Uri.parse(playlistUrl),"audio/x-mpegurl"); try { startActivity(Intent.createChooser(i,"Open playlist with")); } catch(Exception e) { say("No app can open IPTV links. Copy or save the M3U instead."); } }
    private void savePlaylist() {
        if (Build.VERSION.SDK_INT <= 28 && checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) != PackageManager.PERMISSION_GRANTED) {
            pendingPlaylistSave = true;
            requestPermissions(new String[]{Manifest.permission.WRITE_EXTERNAL_STORAGE}, STORAGE_PERMISSION);
            return;
        }
        say("Saving playlist to Downloads…");
        io.execute(this::savePlaylistToDownloads);
    }
    private void savePlaylistToDownloads() {
        HttpURLConnection connection = null;
        try {
            if (playlistUrl.isEmpty()) {
                String path = runKeyShow();
                playlistUrl = PlaylistLink.loopbackUrl(path);
            }
            connection = (HttpURLConnection)new URL(playlistUrl).openConnection();
            connection.setConnectTimeout(2500);
            connection.setReadTimeout(15000);
            connection.setRequestMethod("GET");
            int code = connection.getResponseCode();
            if (code != 200) throw new IOException("playlist returned HTTP " + code);
            try (InputStream input = connection.getInputStream(); OutputStream output = openDownloadsOutput("openstream-tv.m3u", "audio/x-mpegurl")) {
                byte[] buffer = new byte[8192];
                int count;
                while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                output.flush();
            }
            say("Playlist saved to Downloads as openstream-tv.m3u.");
        } catch (Exception e) {
            say("Playlist save failed. Start the local server and try again.");
        } finally {
            if (connection != null) connection.disconnect();
        }
    }
    private void exportConfigFallback() {
        if (Build.VERSION.SDK_INT <= 28 && checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) != PackageManager.PERMISSION_GRANTED) {
            pendingConfigExport = true;
            requestPermissions(new String[]{Manifest.permission.WRITE_EXTERNAL_STORAGE}, STORAGE_PERMISSION);
            return;
        }
        say("Exporting config to Downloads…");
        io.execute(this::exportConfigToDownloads);
    }
    private void exportConfigToDownloads() {
        try {
            ConfigExport.write(new File(getFilesDir(), "config.toml"), openDownloadsOutput("config.toml", "application/toml"));
            say("Config exported to Downloads as config.toml. Login data was not included.");
        } catch (Exception e) {
            say("Config export failed. Check storage access and try again.");
        }
    }
    private OutputStream openDownloadsOutput(String filename, String mimeType) throws IOException {
        if (Build.VERSION.SDK_INT >= 29) {
            ContentValues values = new ContentValues();
            values.put(MediaStore.Downloads.DISPLAY_NAME, filename);
            values.put(MediaStore.Downloads.MIME_TYPE, mimeType);
            values.put(MediaStore.Downloads.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS);
            Uri uri = getContentResolver().insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values);
            if (uri == null) throw new IOException("Cannot create download");
            OutputStream output = getContentResolver().openOutputStream(uri, "w");
            if (output == null) throw new IOException("Cannot open download");
            return output;
        }
        File downloads = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS);
        if (!downloads.isDirectory() && !downloads.mkdirs()) throw new IOException("Cannot create Downloads directory");
        return new FileOutputStream(new File(downloads, filename), false);
    }
    @Override public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode != STORAGE_PERMISSION) return;
        boolean granted = grantResults.length > 0 && grantResults[0] == PackageManager.PERMISSION_GRANTED;
        int fileRequest = pendingFileRequest;
        boolean savePlaylistNow = pendingPlaylistSave;
        boolean exportConfigNow = pendingConfigExport;
        pendingFileRequest = -1;
        pendingPlaylistSave = false;
        pendingConfigExport = false;
        if (!granted) {
            say("Storage access is required for this action.");
            return;
        }
        if (fileRequest == PICK_CONFIG || fileRequest == PICK_STORE) launchTvPicker(fileRequest);
        else if (savePlaylistNow) savePlaylist();
        else if (exportConfigNow) exportConfigFallback();
    }
    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if ((request != PICK_CONFIG && request != PICK_STORE && request != CREATE_CONFIG) || result != RESULT_OK || data == null) return;
        Uri uri = data.getData();
        if (uri == null && (request == PICK_CONFIG || request == PICK_STORE)) {
            String path = data.getStringExtra(TvFilePickerActivity.EXTRA_PATH);
            if (path != null && !path.isEmpty()) uri = Uri.fromFile(new File(path));
        }
        if (uri == null) return;
        final Uri selected = uri;
        if (request == PICK_CONFIG) { say("Validating config locally…"); io.execute(() -> importConfig(selected)); }
        else if (request == PICK_STORE) { say("Validating login store backup…"); io.execute(() -> importStore(selected)); }
        else { say("Exporting config…"); io.execute(() -> exportConfig(selected)); }
    }
    private void exportConfig(Uri uri) { try { OutputStream out=getContentResolver().openOutputStream(uri); if(out==null) throw new IOException("Cannot open export destination"); ConfigExport.write(new File(getFilesDir(), "config.toml"), out); say("Config exported. Login data was not included."); } catch(Exception e) { say("Config export failed. Choose a writable destination and try again."); } }
    private void importConfig(Uri uri) {
        File staged = new File(getCacheDir(), "config.import.toml");
        File validationData = new File(getCacheDir(), "validate-" + System.nanoTime());
        File temp = new File(getFilesDir(), "config.toml.tmp");
        try {
            try (InputStream in = getContentResolver().openInputStream(uri);
                 OutputStream out = new FileOutputStream(staged)) {
                if (in == null) throw new IOException("Cannot read config");
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
                    new File(getApplicationInfo().nativeLibraryDir, "libjiotv_exec.so").getAbsolutePath(),
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
            try (InputStream in = getContentResolver().openInputStream(uri)) {
                if (in == null) throw new IOException("Cannot read store backup");
                StoreBackup.copyBounded(in, staged, 2 * 1024 * 1024);
            }
            if (!StoreBackup.declaresAccessKey(staged)) throw new IOException("Store backup has no access key");
            if (!validationData.mkdir()) throw new IOException("Cannot create validation directory");
            File candidate = new File(validationData, "store_v4.toml");
            try (InputStream in = new FileInputStream(staged)) { StoreBackup.copyBounded(in, candidate, 2 * 1024 * 1024); }
            ProcessBuilder pb = new ProcessBuilder(new File(getApplicationInfo().nativeLibraryDir, "libjiotv_exec.so").getAbsolutePath(), "--skip-update-check", "key", "show");
            pb.directory(validationData);
            pb.redirectOutput(new File("/dev/null"));
            pb.redirectError(new File("/dev/null"));
            pb.environment().put("JIOTV_PATH_PREFIX", validationData.getAbsolutePath());
            pb.environment().put("HOME", getFilesDir().getAbsolutePath());
            Process process = pb.start();
            if (!process.waitFor(10, java.util.concurrent.TimeUnit.SECONDS)) { process.destroyForcibly(); throw new IOException("Store validation timed out"); }
            if (process.exitValue() != 0) throw new IOException("Invalid store schema");
            if (!liveData.isDirectory() && !liveData.mkdirs()) throw new IOException("Cannot access app data directory");
            try (InputStream in = new FileInputStream(staged)) { StoreBackup.copyBounded(in, temp, 2 * 1024 * 1024); }
            restart = isServiceRunning();
            if (restart && !stopServiceAndWait()) throw new IOException("Could not stop server safely");
            Os.rename(temp.getAbsolutePath(), new File(liveData, "store_v4.toml").getAbsolutePath());
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
        runOnUiThread(() -> setServiceButtonState(true));
    }
    @Override protected void onDestroy() { if (shizukuSetup != null) shizukuSetup.close(); io.shutdownNow(); super.onDestroy(); }
}
