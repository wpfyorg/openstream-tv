package org.opebstreamstv.android;

import android.app.Activity;
import android.app.AlertDialog;
import android.widget.ListView;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;

/** Small shared-storage browser for TVs that do not ship an Android document picker. */
final class LocalTomlPicker {
    interface Callback {
        void onImport(File file);
        void onExport(File file);
        void onCancel();
        void onError(String message);
    }

    private final Activity activity;
    private final File root;
    private final boolean export;
    private final Callback callback;

    private LocalTomlPicker(Activity activity, File root, boolean export, Callback callback) throws IOException {
        this.activity = activity;
        this.root = root.getCanonicalFile();
        this.export = export;
        this.callback = callback;
    }

    static void showImport(Activity activity, File root, File start, Callback callback) {
        show(activity, root, start, false, callback);
    }

    static void showExport(Activity activity, File root, File start, Callback callback) {
        show(activity, root, start, true, callback);
    }

    private static void show(Activity activity, File root, File start, boolean export, Callback callback) {
        try {
            LocalTomlPicker picker = new LocalTomlPicker(activity, root, export, callback);
            picker.showDirectory(picker.insideRoot(start) ? start.getCanonicalFile() : picker.root);
        } catch (IOException e) {
            callback.onError("Shared storage is unavailable on this device.");
        }
    }

    private void showDirectory(File directory) {
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

        List<File> entries = listEntries(current, !export);
        List<String> labels = new ArrayList<>();
        List<File> targets = new ArrayList<>();
        if (export) {
            labels.add("Save config.toml in this folder");
            targets.add(null);
        }
        if (!sameFile(current, root)) {
            labels.add("..  Parent folder");
            targets.add(current.getParentFile());
        }
        for (File entry : entries) {
            labels.add(entry.isDirectory() ? "[Folder]  " + entry.getName() : entry.getName());
            targets.add(entry);
        }

        AlertDialog dialog = new AlertDialog.Builder(activity)
                .setTitle((export ? "Export config" : "Import TOML") + "\n" + displayPath(current))
                .setItems(labels.toArray(new String[0]), (d, which) -> {
                    File target = targets.get(which);
                    if (export && target == null) {
                        confirmExport(current);
                    } else if (target != null && target.isDirectory()) {
                        showDirectory(target);
                    } else if (target != null) {
                        callback.onImport(target);
                    }
                })
                .setNegativeButton("Cancel", (d, which) -> callback.onCancel())
                .setOnCancelListener(d -> callback.onCancel())
                .create();
        dialog.setOnShowListener(d -> {
            ListView list = dialog.getListView();
            if (list != null && list.getCount() > 0) {
                list.setSelection(0);
                list.requestFocus();
            }
        });
        dialog.show();
    }

    private void confirmExport(File directory) {
        File destination = new File(directory, "config.toml");
        if (!destination.exists()) {
            callback.onExport(destination);
            return;
        }
        new AlertDialog.Builder(activity)
                .setTitle("Replace config.toml?")
                .setMessage("A config.toml already exists in this folder.")
                .setNegativeButton("Back", (d, which) -> showDirectory(directory))
                .setPositiveButton("Replace", (d, which) -> callback.onExport(destination))
                .setOnCancelListener(d -> showDirectory(directory))
                .show();
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
