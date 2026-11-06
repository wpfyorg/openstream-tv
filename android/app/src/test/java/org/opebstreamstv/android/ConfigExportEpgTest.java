package org.opebstreamstv.android;

import org.junit.Test;

import static org.junit.Assert.assertTrue;

public class ConfigExportEpgTest {
    @Test public void generatedAndroidConfigEnablesEpg() {
        assertTrue(ConfigExport.DEFAULT_TOML.contains("\nepg = true\n"));
    }
}
