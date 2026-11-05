package org.opebstreamstv.android;

import org.junit.Test;
import static org.junit.Assert.*;

public class PlaylistLinkTest {
    @Test public void derivesKeyedPrefixAndLoopbackPlaylist() {
        String path = "/k/example-secret/playlist.m3u";
        assertEquals("/k/example-secret/", PlaylistLink.keyedPrefix(path));
        assertEquals("http://127.0.0.1:5001" + path, PlaylistLink.loopbackUrl(path));
    }
    @Test(expected = IllegalArgumentException.class) public void rejectsUnkeyedPath() { PlaylistLink.keyedPrefix("/playlist.m3u"); }
    @Test(expected = IllegalArgumentException.class) public void rejectsEmptyKey() { PlaylistLink.keyedPrefix("/k//playlist.m3u"); }
    @Test(expected = IllegalArgumentException.class) public void rejectsAmbiguousNestedKey() { PlaylistLink.keyedPrefix("/k/key/extra/playlist.m3u"); }
}
