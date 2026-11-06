package org.opebstreamstv.android;

import org.junit.Test;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.nio.file.Files;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

public class ConfigExportTest {
    @Test public void copiesExistingConfigExactly() throws Exception {
        File file = Files.createTempFile("config", ".toml").toFile();
        try {
            byte[] expected = "tv_plus = true\n".getBytes("UTF-8");
            Files.write(file.toPath(), expected);
            ByteArrayOutputStream output = new ByteArrayOutputStream();
            ConfigExport.write(file, output);
            assertEquals(new String(expected, "UTF-8"), output.toString("UTF-8"));
        } finally { file.delete(); }
    }

    @Test public void writesValidRuntimeDefaultsWhenConfigIsAbsent() throws Exception {
        File dir = Files.createTempDirectory("config-export").toFile();
        try {
            File absent = new File(dir, "config.toml");
            ByteArrayOutputStream output = new ByteArrayOutputStream();
            ConfigExport.write(absent, output);
            String text = output.toString("UTF-8");
            assertTrue(text.contains("epg = true"));
            assertTrue(text.contains("title = \"OpenStream TV\""));
            assertTrue(text.contains("drm = true"));
            assertTrue(text.contains("tv_plus = false"));
            assertTrue(text.contains("default_categories = []"));
            assertTrue(text.contains("default_languages = []"));
            assertTrue(!text.contains("access_key"));
        } finally { dir.delete(); }
    }

    @Test public void writesAndroidDefaultsWhenConfigIsEmpty() throws Exception {
        File file = Files.createTempFile("config", ".toml").toFile();
        try {
            ByteArrayOutputStream output = new ByteArrayOutputStream();
            ConfigExport.write(file, output);
            String text = output.toString("UTF-8");
            assertTrue(text.contains("epg = true"));
            assertTrue(!text.contains("access_key"));
        } finally { file.delete(); }
    }
}
