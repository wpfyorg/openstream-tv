package org.jiotvgo.android;

interface IAdbPairingService {
    boolean pair(String host, int port, String pairingCode);
    void cancel();
}
