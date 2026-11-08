package org.jiotvgo.android;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;

final class StoreBackup {
    private StoreBackup() {}

    static void copyBounded(InputStream input, File target, int maxBytes) throws IOException {
        int total = 0;
        try (FileOutputStream output = new FileOutputStream(target)) {
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) {
                total += count;
                if (total > maxBytes) throw new IOException("Store backup is too large");
                output.write(buffer, 0, count);
            }
            output.getFD().sync();
        } catch (IOException e) {
            target.delete();
            throw e;
        }
    }

    static boolean declaresAccessKey(File file) throws IOException {
        boolean inData = false;
        try (java.io.BufferedReader reader = new java.io.BufferedReader(new java.io.FileReader(file))) {
            String line;
            while ((line = reader.readLine()) != null) {
                String trimmed = line.trim();
                if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
                    inData = "[data]".equals(trimmed);
                } else if (inData && trimmed.matches("(?:access_key|\"access_key\"|'access_key')\\s*=.*")) {
                    return true;
                }
            }
        }
        return false;
    }
}
