package org.opebstreamstv.android;

import android.content.Context;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.util.function.IntPredicate;

/** Selects a loopback port without disturbing a different app's local server. */
final class LocalServerEndpoint {
    private static final String PREFS = "local_server_endpoint";
    private static final String PORT = "port";
    private static final int DEFAULT_PORT = 5001;
    private static final int MAX_PORT = 5010;

    private LocalServerEndpoint() {}

    static int port(Context context) {
        int port = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
                .getInt(PORT, DEFAULT_PORT);
        return port >= DEFAULT_PORT && port <= MAX_PORT ? port : DEFAULT_PORT;
    }

    static String baseUrl(Context context) {
        return baseUrl(port(context));
    }

    static String baseUrl(int port) {
        return "http://127.0.0.1:" + port;
    }

    static int choosePort(Context context) throws IOException {
        int selected = firstFreePort(port(context), LocalServerEndpoint::available);
        if (selected < 0) throw new IOException("All local server ports are occupied");
        if (!context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
                .putInt(PORT, selected).commit()) throw new IOException("Could not persist local port");
        return selected;
    }

    // Pure selection logic permits an offline test of conflict handling.
    static int firstFreePort(int preferred, IntPredicate available) {
        if (preferred >= DEFAULT_PORT && preferred <= MAX_PORT && available.test(preferred))
            return preferred;
        for (int candidate = DEFAULT_PORT; candidate <= MAX_PORT; candidate++) {
            if (candidate != preferred && available.test(candidate)) return candidate;
        }
        return -1;
    }

    private static boolean available(int port) {
        try (ServerSocket socket = new ServerSocket()) {
            // Tokio's TcpListener::bind enables SO_REUSEADDR on Unix. Match that
            // behavior so a recent server stop does not move the persisted port.
            socket.setReuseAddress(true);
            socket.bind(new InetSocketAddress("127.0.0.1", port));
            return true;
        } catch (IOException e) {
            return false;
        }
    }
}
