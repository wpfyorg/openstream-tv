package org.opebstreamstv.android;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

final class ConfigExport {
    static final String DEFAULT_TOML =
            "# OpenStream TV configuration. Values match the Rust runtime defaults.\n" +
            "epg = true\n" +
            "debug = false\n" +
            "disable_ts_handler = false\n" +
            "disable_logout = false\n" +
            "drm = true\n" +
            "tv_plus = false\n" +
            "disable_auth = false\n" +
            "title = \"OpenStream TV\"\n" +
            "disable_url_encryption = false\n" +
            "proxy = \"\"\n" +
            "public_origin = \"\"\n" +
            "path_prefix = \"\"\n" +
            "log_path = \"\"\n" +
            "log_to_stdout = true\n" +
            "custom_channels_file = \"\"\n" +
            "default_categories = []\n" +
            "default_languages = []\n";

    private ConfigExport() {}

    static void write(File config, OutputStream destination) throws IOException {
        if (destination == null) throw new IOException("Cannot open destination");
        try (OutputStream output = destination) {
            if (config.isFile() && config.length() > 0) {
                try (InputStream input = new FileInputStream(config)) {
                    byte[] buffer = new byte[8192];
                    int count;
                    while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                }
            } else {
                output.write(DEFAULT_TOML.getBytes(java.nio.charset.StandardCharsets.UTF_8));
            }
            output.flush();
        }
    }
}
