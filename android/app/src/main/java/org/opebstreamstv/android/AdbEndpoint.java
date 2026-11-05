package org.opebstreamstv.android;

/** Validation shared by the ADB setup UI before opening a network connection. */
final class AdbEndpoint {
    private AdbEndpoint() { }
    static int parsePort(String value) {
        if (value == null || value.trim().isEmpty()) throw new IllegalArgumentException("ADB port is required");
        try {
            int port = Integer.parseInt(value.trim());
            if (port < 1 || port > 65535) throw new IllegalArgumentException("ADB port is out of range");
            return port;
        } catch (NumberFormatException e) { throw new IllegalArgumentException("ADB port is invalid", e); }
    }
}
