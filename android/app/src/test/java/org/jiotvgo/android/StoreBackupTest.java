package org.jiotvgo.android;

import org.junit.Test;
import java.io.File;
import java.nio.file.Files;
import static org.junit.Assert.*;

public class StoreBackupTest {
    @Test public void requiresAccessKeyInsideDataTable() throws Exception {
        File file = Files.createTempFile("store-schema", ".toml").toFile();
        try {
            Files.write(file.toPath(), "[data]\ndeviceId = \"redacted\"\naccess_key = \"redacted\"\n".getBytes("UTF-8"));
            assertTrue(StoreBackup.declaresAccessKey(file));
            Files.write(file.toPath(), "[other]\naccess_key = \"redacted\"\n[data]\ndeviceId = \"redacted\"\n".getBytes("UTF-8"));
            assertFalse(StoreBackup.declaresAccessKey(file));
        } finally { file.delete(); }
    }
}
