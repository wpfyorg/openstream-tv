package org.opebstreamstv.android;

interface IAdbPairingService {
    boolean pair(String host, int port, String pairingCode);
    void cancel();
}
