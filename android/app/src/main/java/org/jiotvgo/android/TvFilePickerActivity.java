package org.jiotvgo.android;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.os.Environment;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import java.io.File;
import java.io.IOException;
import java.util.Arrays;
import java.util.Comparator;

public final class TvFilePickerActivity extends Activity {
    static final String EXTRA_TITLE = "org.jiotvgo.android.FILE_PICKER_TITLE";
    static final String EXTRA_PATH = "org.jiotvgo.android.FILE_PICKER_PATH";
    private static final int SYSTEM_PICKER = 91;

    private final int background = 0xff0b1220;
    private final int panel = 0xff152234;
    private final int row = 0xff1e2e40;
    private final int border = 0xff34485b;
    private final int mint = 0xff42d7ae;
    private final int text = 0xffedf5f8;
    private final int muted = 0xff93a6b7;
    private float scaleX = 1f;
    private float scaleY = 1f;
    private float scale = 1f;
    private File rootDirectory;
    private File currentDirectory;
    private TextView pathView;
    private LinearLayout rows;
    private ScrollView scroll;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        getWindow().setStatusBarColor(background);
        getWindow().setNavigationBarColor(background);
        getWindow().getDecorView().setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);

        android.util.DisplayMetrics metrics = getResources().getDisplayMetrics();
        scaleX = metrics.widthPixels / 1920f;
        scaleY = metrics.heightPixels / 1080f;
        scale = Math.min(scaleX, scaleY);

        rootDirectory = Environment.getExternalStorageDirectory();
        File downloads = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS);
        currentDirectory = downloads.isDirectory() ? downloads : rootDirectory;

        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(background);

        String title = getIntent().getStringExtra(EXTRA_TITLE);
        if (title == null || title.trim().isEmpty()) title = "Choose a file";
        place(root, label("Openstream TV Slim", 35, text, true), 140, 80, 720, 48);
        place(root, label(title, 21, muted, false), 140, 130, 720, 30);

        View card = new View(this);
        card.setBackground(roundRect(panel, 24, border, 2));
        place(root, card, 360, 210, 1200, 670);

        place(root, label("Choose a file", 32, 0xfff5f9fc, true), 415, 255, 900, 44);
        place(root, label(title.toLowerCase().contains("login store")
                ? "Select a TOML login store backup from local storage."
                : "Select a TOML configuration file from local storage.", 21, 0xffa9b9c6, false), 415, 305, 1000, 30);

        pathView = label("", 20, 0xffb4c9d3, false);
        pathView.setPadding(x(20), 0, x(20), 0);
        pathView.setBackground(roundRect(0xff101b2a, 10, border, 2));
        place(root, pathView, 415, 357, 1090, 58);

        scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        scroll.setSmoothScrollingEnabled(true);
        rows = new LinearLayout(this);
        rows.setOrientation(LinearLayout.VERTICAL);
        rows.setPadding(0, 0, 0, 0);
        scroll.addView(rows, new ScrollView.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        place(root, scroll, 415, 440, 1090, 300);

        Button cancel = actionButton("Cancel");
        cancel.setOnClickListener(v -> finish());
        place(root, cancel, 1253, 754, 252, 60);
        place(root, label("D-PAD  Navigate     OK  Open     BACK  Cancel", 20, 0xffa8b9c6, false), 415, 823, 760, 34);

        setContentView(root);
        refreshDirectory();
    }

    private void refreshDirectory() {
        rows.removeAllViews();
        pathView.setText(displayPath(currentDirectory));

        View firstFocusable = null;
        View firstFile = null;
        if (!sameFile(currentDirectory, rootDirectory)) {
            LinearLayout parent = rowView("↑   ..", "Parent folder", true);
            parent.setOnClickListener(v -> navigateUp());
            rows.addView(parent, rowParams());
            firstFocusable = parent;
        }

        File[] files;
        try { files = currentDirectory.listFiles(); }
        catch (SecurityException denied) { files = null; }
        if (files == null) files = new File[0];
        Arrays.sort(files, Comparator
                .comparing((File file) -> !file.isDirectory())
                .thenComparing(File::getName, String.CASE_INSENSITIVE_ORDER));

        int shown = 0;
        for (File file : files) {
            if (!file.isDirectory() && !file.isFile()) continue;
            String prefix = file.isDirectory() ? "▣   " : "▢   ";
            String kind = file.isDirectory() ? "Folder" : (file.getName().toLowerCase().endsWith(".toml") ? "TOML file" : "File");
            LinearLayout item = rowView(prefix + file.getName(), kind, file.isDirectory());
            item.setOnClickListener(v -> {
                if (file.isDirectory()) {
                    currentDirectory = file;
                    refreshDirectory();
                } else {
                    Intent result = new Intent();
                    result.putExtra(EXTRA_PATH, file.getAbsolutePath());
                    setResult(RESULT_OK, result);
                    finish();
                }
            });
            rows.addView(item, rowParams());
            if (firstFocusable == null) firstFocusable = item;
            if (!file.isDirectory() && firstFile == null) firstFile = item;
            shown++;
        }

        if (shown == 0) {
            if (android.os.Build.VERSION.SDK_INT >= 33) {
                LinearLayout browse = rowView("▢   Browse device files", "System file picker", false);
                browse.setOnClickListener(v -> openSystemPicker());
                rows.addView(browse, rowParams());
                firstFocusable = browse;
                firstFile = browse;
            } else {
                TextView empty = label("No files or folders here.", 19, muted, false);
                empty.setGravity(Gravity.CENTER);
                rows.addView(empty, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, y(90)));
            }
        }

        View focusTarget = firstFile != null ? firstFile : firstFocusable;
        if (focusTarget != null) focusTarget.post(focusTarget::requestFocus);
    }

    private LinearLayout rowView(String value, String kind, boolean directory) {
        LinearLayout view = new LinearLayout(this);
        view.setId(View.generateViewId());
        view.setOrientation(LinearLayout.HORIZONTAL);
        view.setGravity(Gravity.CENTER_VERTICAL);
        view.setPadding(x(23), 0, 0, 0);
        view.setFocusable(true);
        view.setFocusableInTouchMode(true);
        view.setClickable(true);
        view.setBackground(roundRect(row, 10, 0xff304357, 2));

        TextView name = label(value, 22, 0xffeaf2f5, true);
        TextView type = label(kind, 20, 0xff8da0b1, false);
        type.setGravity(Gravity.CENTER_VERTICAL | Gravity.RIGHT);
        view.addView(name, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f));
        view.addView(type, new LinearLayout.LayoutParams(x(255), ViewGroup.LayoutParams.MATCH_PARENT));
        view.setOnFocusChangeListener((v, focused) -> {
            view.setBackground(focused ? roundRect(mint, 10, 0xffb0f9e1, 2) : roundRect(row, 10, 0xff304357, 2));
            name.setTextColor(focused ? 0xff09251d : 0xffeaf2f5);
            type.setTextColor(focused ? 0xff225649 : 0xff8da0b1);
            if (focused) v.post(() -> v.requestRectangleOnScreen(new android.graphics.Rect(0, 0, v.getWidth(), v.getHeight()), false));
        });
        return view;
    }

    private LinearLayout.LayoutParams rowParams() {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, y(72));
        params.bottomMargin = y(14);
        return params;
    }

    private Button actionButton(String value) {
        Button button = new Button(this);
        button.setId(View.generateViewId());
        button.setText(value);
        button.setAllCaps(false);
        button.setTextSize(TypedValue.COMPLEX_UNIT_PX, 23 * scale);
        button.setTypeface(Typeface.create("sans-serif", Typeface.BOLD));
        button.setTextColor(text);
        button.setGravity(Gravity.CENTER);
        button.setFocusable(true);
        button.setFocusableInTouchMode(true);
        button.setBackground(roundRect(0xff202d3e, 12, 0xff354459, 2));
        button.setPadding(0, 0, 0, 0);
        button.setOnFocusChangeListener((v, focused) -> {
            button.setBackground(focused ? roundRect(mint, 12, 0xffb0f9e1, 2) : roundRect(0xff202d3e, 12, 0xff354459, 2));
            button.setTextColor(focused ? 0xff071e1b : text);
        });
        return button;
    }

    private void navigateUp() {
        File parentDirectory = currentDirectory.getParentFile();
        if (parentDirectory != null && isWithinRoot(parentDirectory)) {
            currentDirectory = parentDirectory;
            refreshDirectory();
        }
    }

    private void openSystemPicker() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("*/*");
        startActivityForResult(intent, SYSTEM_PICKER);
    }

    @Override protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == SYSTEM_PICKER && resultCode == RESULT_OK && data != null && data.getData() != null) {
            Intent result = new Intent();
            result.setData(data.getData());
            setResult(RESULT_OK, result);
            finish();
        }
    }

    @Override public void onBackPressed() {
        if (!sameFile(currentDirectory, rootDirectory)) navigateUp();
        else super.onBackPressed();
    }

    private boolean isWithinRoot(File candidate) {
        try {
            String rootPath = rootDirectory.getCanonicalPath();
            String candidatePath = candidate.getCanonicalPath();
            return candidatePath.equals(rootPath) || candidatePath.startsWith(rootPath + File.separator);
        } catch (IOException e) {
            return false;
        }
    }

    private boolean sameFile(File first, File second) {
        try { return first.getCanonicalFile().equals(second.getCanonicalFile()); }
        catch (IOException e) { return first.equals(second); }
    }

    private String displayPath(File directory) {
        try {
            String rootPath = rootDirectory.getCanonicalPath();
            String currentPath = directory.getCanonicalPath();
            if (currentPath.equals(rootPath)) return "Internal storage";
            if (currentPath.startsWith(rootPath + File.separator)) {
                String relative = currentPath.substring(rootPath.length() + 1);
                return "Internal storage / " + relative.replace(File.separator, " / ");
            }
        } catch (IOException ignored) {}
        return directory.getAbsolutePath();
    }

    private int x(int value) { return Math.round(value * scaleX); }
    private int y(int value) { return Math.round(value * scaleY); }
    private void place(FrameLayout root, View view, int left, int top, int width, int height) {
        FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(x(width), y(height));
        params.leftMargin = x(left);
        params.topMargin = y(top);
        root.addView(view, params);
    }
    private TextView label(String value, int size, int color, boolean bold) {
        TextView view = new TextView(this);
        view.setText(value);
        view.setTextColor(color);
        view.setTextSize(TypedValue.COMPLEX_UNIT_PX, size * scale);
        view.setTypeface(Typeface.create("sans-serif", bold ? Typeface.BOLD : Typeface.NORMAL));
        view.setGravity(Gravity.CENTER_VERTICAL);
        view.setIncludeFontPadding(false);
        return view;
    }
    private GradientDrawable roundRect(int fill, int radius, int stroke, int strokeWidth) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(fill);
        drawable.setCornerRadius(scale * radius);
        if (strokeWidth > 0) drawable.setStroke(Math.max(1, Math.round(scale * strokeWidth)), stroke);
        return drawable;
    }
}
