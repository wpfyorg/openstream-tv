package org.opebstreamstv.android;

import org.junit.Test;
import java.io.File;
import java.io.ByteArrayInputStream;
import java.io.IOException;
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
            Files.write(file.toPath(), "[ data ] # comment\naccess_key = \"redacted\"\n".getBytes("UTF-8"));
            assertTrue(StoreBackup.declaresAccessKey(file));
            Files.write(file.toPath(), "[data] # comment\ndeviceId = \"redacted\"\n[ other ] # comment\naccess_key = \"redacted\"\n".getBytes("UTF-8"));
            assertFalse(StoreBackup.declaresAccessKey(file));
        } finally { file.delete(); }
    }

    @Test public void oversizedCopyDeletesPartialFile() throws Exception {
        File file = Files.createTempFile("store-too-large", ".toml").toFile();
        try {
            assertThrows(IOException.class, () -> StoreBackup.copyBounded(new ByteArrayInputStream(new byte[8193]), file, 8192));
            assertFalse(file.exists());
        } finally { Files.deleteIfExists(file.toPath()); }
    }
}
