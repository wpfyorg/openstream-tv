package org.jiotvgo.android;

import java.util.regex.Pattern;

final class PlaylistDisplay {
    private static final Pattern KEYED_SEGMENT = Pattern.compile("(/k/)[^/]+(/)");
    private PlaylistDisplay() {}

    static String masked(String url) {
        return KEYED_SEGMENT.matcher(url).replaceFirst("$1••••$2");
    }
}
