package org.jiotvgo.android;

import org.junit.Test;
import static org.junit.Assert.assertEquals;

public class PlaylistDisplayTest {
    @Test public void masksCredentialInKeyedPath() {
        assertEquals("http://127.0.0.1:5001/k/••••/playlist.m3u",
                PlaylistDisplay.masked("http://127.0.0.1:5001/k/sensitive-key/playlist.m3u"));
    }
}
