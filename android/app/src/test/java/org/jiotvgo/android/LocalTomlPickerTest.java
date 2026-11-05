package org.jiotvgo.android;

import static org.junit.Assert.assertEquals;

import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

import java.io.File;
import java.util.List;

public final class LocalTomlPickerTest {
    @Rule public final TemporaryFolder temp = new TemporaryFolder();

    @Test public void listsFoldersFirstAndOnlyTomlFilesForImport() throws Exception {
        File root = temp.newFolder("storage");
        File beta = new File(root, "Beta");
        File alpha = new File(root, "alpha");
        alpha.mkdir();
        beta.mkdir();
        temp.newFile("storage/notes.txt");
        temp.newFile("storage/z-store.TOML");
        temp.newFile("storage/a-config.toml");

        List<File> entries = LocalTomlPicker.listEntries(root, true);

        assertEquals(4, entries.size());
        assertEquals("alpha", entries.get(0).getName());
        assertEquals("Beta", entries.get(1).getName());
        assertEquals("a-config.toml", entries.get(2).getName());
        assertEquals("z-store.TOML", entries.get(3).getName());
    }

    @Test public void exportModeListsFoldersOnly() throws Exception {
        File root = temp.newFolder("export");
        File folder = new File(root, "Download");
        folder.mkdir();
        temp.newFile("export/config.toml");

        List<File> entries = LocalTomlPicker.listEntries(root, false);

        assertEquals(1, entries.size());
        assertEquals("Download", entries.get(0).getName());
    }
}
