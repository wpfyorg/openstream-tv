package org.opebstreamstv.android;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;

final class PlaylistLink {
    private PlaylistLink() {}
    static String keyedPrefix(String path) {
        if (path == null || !path.startsWith("/k/") || !path.endsWith("/playlist.m3u")) throw new IllegalArgumentException("Invalid keyed playlist path");
        String prefix = path.substring(0, path.length() - "playlist.m3u".length());
        String key = prefix.substring(3, prefix.length() - 1);
        if (!key.matches("[A-Za-z0-9._~-]+")) throw new IllegalArgumentException("Invalid access key");
        return prefix;
    }
    static String loopbackUrl(String playlistPath, int port) { return LocalServerEndpoint.baseUrl(port) + playlistPath; }
    static String readValidatedPath(InputStream input) throws IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] buffer = new byte[512];
        int count;
        while ((count = input.read(buffer)) != -1) {
            if (out.size() + count > 4096) throw new IOException("Playlist path is too long");
            out.write(buffer, 0, count);
        }
        String path = new String(out.toByteArray(), StandardCharsets.UTF_8).trim();
        keyedPrefix(path);
        return path;
    }
}
