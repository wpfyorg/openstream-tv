package org.opebstreamstv.android;

import org.junit.Test;

import static org.junit.Assert.assertEquals;

public class LocalServerEndpointTest {
    @Test public void fallsBackWhenDefaultAlreadyOccupied() {
        assertEquals(5002, LocalServerEndpoint.firstFreePort(5001, port -> port == 5002));
    }

    @Test public void retainsPreviousFallbackPortWhenFree() {
        assertEquals(5002, LocalServerEndpoint.firstFreePort(5002, port -> port == 5001 || port == 5002));
    }

    @Test public void reportsExhaustedPortRange() {
        assertEquals(-1, LocalServerEndpoint.firstFreePort(5001, port -> false));
    }
}
