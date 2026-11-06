package org.opebstreamstv.android;

import android.app.Activity;
import android.app.AlertDialog;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;

/** Small shared-storage browser for TVs that do not ship an Android document picker. */
final class LocalTomlPicker {
    // Compile-time colors also allow pure-JVM file-listing tests to load this class.
    private static final int COLOR_BACKGROUND = 0xff0b1220;
    private static final int COLOR_PANEL = 0xff152234;
    private static final int COLOR_ROW = 0xff1e2e40;
    private static final int COLOR_BORDER = 0xff34485b;
    private static final int COLOR_TEXT = 0xffeaf2f5;
    private static final int COLOR_MUTED = 0xff93a6b7;
    private static final int COLOR_ACCENT = 0xff42d7ae;

    interface Callback {
        void onImport(File file);
        void onExport(File file);
        void onCancel();
        void onError(String message);
    }

    private final Activity activity;
    private final File root;
    private final boolean export;
    private final String exportFilename;
    private final boolean loginStoreImport;
    private final Callback callback;
    private AlertDialog browserDialog;

    private LocalTomlPicker(Activity activity, File root, boolean export, String exportFilename, boolean loginStoreImport, Callback callback) throws IOException {
        this.activity = activity;
        this.root = root.getCanonicalFile();
        this.export = export;
        this.exportFilename = exportFilename;
        this.loginStoreImport = loginStoreImport;
        this.callback = callback;
    }

    static void showImport(Activity activity, File root, File start, Callback callback) {
        showImport(activity, root, start, false, callback);
    }

    static void showImport(Activity activity, File root, File start, boolean loginStoreImport, Callback callback) {
        show(activity, root, start, false, "config.toml", loginStoreImport, callback);
    }

    static void showExport(Activity activity, File root, File start, Callback callback) {
        showExport(activity, root, start, "config.toml", callback);
    }

    static void showExport(Activity activity, File root, File start, String filename, Callback callback) {
        if (!isSafeFilename(filename)) {
            callback.onError("That file name is not supported.");
            return;
        }
        show(activity, root, start, true, filename, false, callback);
    }

    private static void show(Activity activity, File root, File start, boolean export, String exportFilename, boolean loginStoreImport, Callback callback) {
        try {
            LocalTomlPicker picker = new LocalTomlPicker(activity, root, export, exportFilename, loginStoreImport, callback);
            picker.showDirectory(picker.insideRoot(start) ? start.getCanonicalFile() : picker.root);
        } catch (IOException e) {
            callback.onError("Shared storage is unavailable on this device.");
        }
    }

    private void showDirectory(File directory) {
        dismissBrowser();
        final File current;
        try {
            current = directory.getCanonicalFile();
        } catch (IOException e) {
            callback.onError("That storage folder cannot be opened.");
            return;
        }
        if (!insideRoot(current) || !current.isDirectory()) {
            callback.onError("That storage folder cannot be opened.");
            return;
        }

        List<PickerEntry> entries = new ArrayList<>();
        if (export) {
            entries.add(new PickerEntry("▣", "Save " + exportFilename + " in this folder", "Save here", null, true));
        }
        if (!sameFile(current, root)) {
            entries.add(new PickerEntry("↑", "..", "Parent folder", current.getParentFile(), false));
        }
        for (File entry : listEntries(current, !export)) {
            entries.add(new PickerEntry(entry.isDirectory() ? "▰" : "▣", entry.getName(),
                    entry.isDirectory() ? "Folder" : "TOML file", entry, false));
        }

        LinearLayout panel = new LinearLayout(activity);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setPadding(dp(28), dp(24), dp(28), dp(20));
        panel.setMinimumWidth(dp(720));
        panel.setBackground(surface(COLOR_PANEL, COLOR_BORDER, 2, 24));

        TextView appTitle = text("Openstream TV Slim", 22, COLOR_TEXT, Typeface.BOLD);
        panel.addView(appTitle, matchWrap());
        boolean playlistExport = export && exportFilename.endsWith(".m3u");
        String importSubtitle = loginStoreImport ? "Import login store backup" : "Import configuration";
        TextView appSubtitle = text(export ? (playlistExport ? "Save M3U playlist" : "Export configuration") : importSubtitle, 14, COLOR_MUTED, Typeface.NORMAL);
        panel.addView(appSubtitle, matchWrap());

        addSpace(panel, 20);
        panel.addView(text(export ? "Choose a folder" : "Choose a file", 25, COLOR_TEXT, Typeface.BOLD), matchWrap());
        String importDescription = loginStoreImport
                ? "Select a login store backup from local storage."
                : "Select a TOML configuration file from local storage.";
        panel.addView(text(export ? (playlistExport ? "Choose where to save the actual M3U playlist." : "Choose where to save the TOML configuration.") : importDescription,
                15, COLOR_MUTED, Typeface.NORMAL), matchWrap());

        addSpace(panel, 16);
        TextView path = text(displayPath(current), 15, Color.rgb(180, 201, 211), Typeface.NORMAL);
        path.setSingleLine(true);
        path.setEllipsize(android.text.TextUtils.TruncateAt.MIDDLE);
        path.setPadding(dp(14), dp(12), dp(14), dp(12));
        path.setBackground(surface(COLOR_BACKGROUND, COLOR_BORDER, 1, 10));
        panel.addView(path, matchWrap());

        addSpace(panel, 12);
        ScrollView scroll = new ScrollView(activity);
        scroll.setFillViewport(false);
        LinearLayout rows = new LinearLayout(activity);
        rows.setOrientation(LinearLayout.VERTICAL);
        for (PickerEntry entry : entries) {
            View row = pickerRow(entry);
            row.setOnClickListener(v -> {
                if (export && entry.saveHere) {
                    confirmExport(current);
                } else if (entry.target != null && entry.target.isDirectory()) {
                    showDirectory(entry.target);
                } else if (entry.target != null) {
                    dismissBrowser();
                    callback.onImport(entry.target);
                }
            });
            LinearLayout.LayoutParams rowParams = matchWrap();
            rowParams.bottomMargin = dp(8);
            rows.addView(row, rowParams);
        }
        scroll.addView(rows);
        int maxRowsHeight = Math.min(dp(288), activity.getResources().getDisplayMetrics().heightPixels / 2);
        panel.addView(scroll, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, maxRowsHeight));

        LinearLayout actions = new LinearLayout(activity);
        actions.setGravity(Gravity.RIGHT | Gravity.CENTER_VERTICAL);
        TextView cancel = text("Cancel", 16, COLOR_TEXT, Typeface.BOLD);
        cancel.setGravity(Gravity.CENTER);
        cancel.setFocusable(true);
        cancel.setClickable(true);
        cancel.setPadding(dp(24), dp(12), dp(24), dp(12));
        cancel.setBackground(surface(COLOR_ROW, COLOR_BORDER, 1, 12));
        cancel.setOnFocusChangeListener((v, focused) -> v.setBackground(surface(focused ? COLOR_ACCENT : COLOR_ROW,
                focused ? Color.rgb(176, 249, 225) : COLOR_BORDER, 2, 12)));
        actions.addView(cancel, new LinearLayout.LayoutParams(dp(166), dp(48)));
        panel.addView(actions, matchWrap());

        addSpace(panel, 8);
        TextView hint = text("D-PAD  Navigate     OK  Open     BACK  Cancel", 14, Color.rgb(168, 185, 198), Typeface.NORMAL);
        panel.addView(hint, matchWrap());

        AlertDialog dialog = new AlertDialog.Builder(activity).setView(panel).setOnCancelListener(d -> callback.onCancel()).create();
        browserDialog = dialog;
        dialog.setOnDismissListener(d -> {
            if (browserDialog == dialog) browserDialog = null;
        });
        cancel.setOnClickListener(v -> dialog.cancel());
        dialog.setCanceledOnTouchOutside(false);
        dialog.setOnShowListener(d -> {
            Window window = dialog.getWindow();
            if (window != null) {
                window.setBackgroundDrawableResource(android.R.color.transparent);
                int screenWidth = activity.getResources().getDisplayMetrics().widthPixels;
                window.setLayout(Math.min(dp(880), screenWidth - dp(48)), WindowManager.LayoutParams.WRAP_CONTENT);
            }
            if (rows.getChildCount() > 0) rows.getChildAt(0).requestFocus();
            else cancel.requestFocus();
        });
        dialog.show();
    }

    private View pickerRow(PickerEntry entry) {
        LinearLayout row = new LinearLayout(activity);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setPadding(dp(14), 0, dp(14), 0);
        row.setBackground(surface(COLOR_ROW, COLOR_BORDER, 1, 10));
        row.setFocusable(true);
        row.setClickable(true);
        row.setOnFocusChangeListener((v, focused) -> {
            v.setBackground(surface(focused ? COLOR_ACCENT : COLOR_ROW,
                    focused ? Color.rgb(176, 249, 225) : COLOR_BORDER, 2, 10));
            for (int i = 0; i < row.getChildCount(); i++) {
                View child = row.getChildAt(i);
                if (child instanceof TextView) {
                    TextView label = (TextView) child;
                    label.setTextColor(focused ? Color.rgb(9, 37, 29) : (i == 0 ? Color.rgb(159, 220, 203) : COLOR_TEXT));
                }
            }
        });
        TextView icon = text(entry.icon, 18, Color.rgb(159, 220, 203), Typeface.BOLD);
        icon.setGravity(Gravity.CENTER);
        row.addView(icon, new LinearLayout.LayoutParams(dp(34), LinearLayout.LayoutParams.WRAP_CONTENT));
        TextView label = text(entry.label, 16, COLOR_TEXT, Typeface.BOLD);
        label.setSingleLine(true);
        label.setEllipsize(android.text.TextUtils.TruncateAt.END);
        row.addView(label, new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1));
        TextView detail = text(entry.detail, 14, Color.rgb(141, 160, 177), Typeface.NORMAL);
        detail.setGravity(Gravity.RIGHT | Gravity.CENTER_VERTICAL);
        row.addView(detail, new LinearLayout.LayoutParams(dp(150), LinearLayout.LayoutParams.WRAP_CONTENT));
        row.setMinimumHeight(dp(58));
        row.setContentDescription(entry.label + ", " + entry.detail);
        return row;
    }

    private TextView text(String value, int sizeSp, int color, int style) {
        TextView view = new TextView(activity);
        view.setText(value);
        view.setTextSize(TypedValue.COMPLEX_UNIT_SP, sizeSp);
        view.setTextColor(color);
        view.setTypeface(Typeface.DEFAULT, style);
        return view;
    }

    private GradientDrawable surface(int color, int strokeColor, int strokeDp, int radiusDp) {
        GradientDrawable shape = new GradientDrawable();
        shape.setColor(color);
        shape.setCornerRadius(dp(radiusDp));
        shape.setStroke(dp(strokeDp), strokeColor);
        return shape;
    }

    private int dp(int value) {
        return Math.round(value * activity.getResources().getDisplayMetrics().density);
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
    }

    private void addSpace(LinearLayout parent, int heightDp) {
        parent.addView(new View(activity), new LinearLayout.LayoutParams(1, dp(heightDp)));
    }

    private static final class PickerEntry {
        final String icon;
        final String label;
        final String detail;
        final File target;
        final boolean saveHere;

        PickerEntry(String icon, String label, String detail, File target, boolean saveHere) {
            this.icon = icon;
            this.label = label;
            this.detail = detail;
            this.target = target;
            this.saveHere = saveHere;
        }
    }

    private void confirmExport(File directory) {
        dismissBrowser();
        File destination = new File(directory, exportFilename);
        if (!destination.exists()) {
            callback.onExport(destination);
            return;
        }
        new AlertDialog.Builder(activity)
                .setTitle("Replace " + exportFilename + "?")
                .setMessage(exportFilename + " already exists in this folder.")
                .setNegativeButton("Back", (d, which) -> showDirectory(directory))
                .setPositiveButton("Replace", (d, which) -> callback.onExport(destination))
                .setOnCancelListener(d -> showDirectory(directory))
                .show();
    }

    private void dismissBrowser() {
        AlertDialog dialog = browserDialog;
        browserDialog = null;
        if (dialog != null && dialog.isShowing()) dialog.dismiss();
    }

    private static boolean isSafeFilename(String filename) {
        return filename != null && filename.matches("[A-Za-z0-9._-]+")
                && !".".equals(filename) && !"..".equals(filename);
    }

    private String displayPath(File current) {
        if (sameFile(current, root)) return "Shared storage";
        String rootPath = root.getAbsolutePath();
        String path = current.getAbsolutePath();
        if (path.startsWith(rootPath + File.separator)) {
            return "Shared storage / " + path.substring(rootPath.length() + 1);
        }
        return path;
    }

    private boolean insideRoot(File candidate) {
        if (candidate == null) return false;
        try {
            String rootPath = root.getCanonicalPath();
            String candidatePath = candidate.getCanonicalPath();
            return candidatePath.equals(rootPath) || candidatePath.startsWith(rootPath + File.separator);
        } catch (IOException e) {
            return false;
        }
    }

    private static boolean sameFile(File first, File second) {
        try {
            return first.getCanonicalFile().equals(second.getCanonicalFile());
        } catch (IOException e) {
            return first.equals(second);
        }
    }

    static List<File> listEntries(File directory, boolean includeTomlFiles) {
        File[] children = directory.listFiles();
        if (children == null) return new ArrayList<>();
        List<File> entries = new ArrayList<>();
        for (File child : children) {
            if (child.isHidden()) continue;
            if (child.isDirectory() || (includeTomlFiles && child.canRead() && isToml(child))) entries.add(child);
        }
        entries.sort(Comparator
                .comparing((File file) -> !file.isDirectory())
                .thenComparing(file -> file.getName().toLowerCase(Locale.ROOT)));
        return entries;
    }

    private static boolean isToml(File file) {
        return file.isFile() && file.getName().toLowerCase(Locale.ROOT).endsWith(".toml");
    }
}
