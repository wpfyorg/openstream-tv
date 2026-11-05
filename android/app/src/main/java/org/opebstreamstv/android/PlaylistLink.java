package org.opebstreamstv.android;

final class PlaylistLink {
    private PlaylistLink() {}
    static String keyedPrefix(String path) {
        if (path == null || !path.startsWith("/k/") || !path.endsWith("/playlist.m3u")) throw new IllegalArgumentException("Invalid keyed playlist path");
        String prefix = path.substring(0, path.length() - "playlist.m3u".length());
        String key = prefix.substring(3, prefix.length() - 1);
        if (key.isEmpty() || key.indexOf('/') >= 0) throw new IllegalArgumentException("Invalid access key");
        return prefix;
    }
    static String loopbackUrl(String playlistPath) { return "http://127.0.0.1:5001" + playlistPath; }
}
