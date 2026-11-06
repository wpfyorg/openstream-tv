package org.opebstreamstv.android;

import org.junit.Test;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.fail;

public class AdbEndpointTest {
    @Test public void acceptsPortsFromOneThrough65535() {
        assertEquals(1, AdbEndpoint.parsePort("1"));
        assertEquals(45678, AdbEndpoint.parsePort(" 45678 "));
        assertEquals(65535, AdbEndpoint.parsePort("65535"));
    }
    @Test public void rejectsMissingMalformedAndOutOfRangePorts() {
        for (String value : new String[] {null, "", "abc", "0", "65536"}) {
            try { AdbEndpoint.parsePort(value); fail("accepted invalid port"); }
            catch (IllegalArgumentException expected) { }
        }
    }
}
