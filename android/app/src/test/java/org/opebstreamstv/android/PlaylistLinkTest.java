package org.opebstreamstv.android;

import org.junit.Test;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
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
    @Test public void validatesCliOutput() throws Exception {
        assertEquals("/k/key/playlist.m3u", PlaylistLink.readValidatedPath(new ByteArrayInputStream("/k/key/playlist.m3u\n".getBytes(StandardCharsets.UTF_8))));
        assertThrows(IllegalArgumentException.class, () -> PlaylistLink.readValidatedPath(new ByteArrayInputStream("/k/a/b/playlist.m3u\n".getBytes(StandardCharsets.UTF_8))));
        assertThrows(IOException.class, () -> PlaylistLink.readValidatedPath(new ByteArrayInputStream(new byte[4097])));
    }
}
