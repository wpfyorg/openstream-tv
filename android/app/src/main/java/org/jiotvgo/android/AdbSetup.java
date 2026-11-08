package org.jiotvgo.android;

import android.content.Context;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;

import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.math.BigInteger;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.Key;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.KeyStore;
import java.security.Signature;
import java.security.interfaces.RSAPublicKey;
import java.security.spec.AlgorithmParameterSpec;
import java.util.Arrays;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

/** User-triggered ADB AUTH flow, limited to this app's overlay app-op. */
final class AdbSetup {
    static final String LOOPBACK_HOST = "127.0.0.1";
    static final int LOOPBACK_PORT = 5555;
    private static final int A_CNXN = 0x4e584e43;
    private static final int A_AUTH = 0x48545541;
    private static final int A_OPEN = 0x4e45504f;
    private static final int A_OKAY = 0x59414b4f;
    private static final int A_CLSE = 0x45534c43;
    private static final int A_WRTE = 0x45545257;
    private static final int AUTH_TOKEN = 1;
    private static final int AUTH_SIGNATURE = 2;
    private static final int AUTH_PUBLIC_KEY = 3;
    private static final int ADB_VERSION = 0x01000001;
    private static final int MAX_DATA = 4096;
    private static final int MAX_PACKET = 1024 * 1024;
    private static final int CONNECT_TIMEOUT_MS = 4000;
    private static final int AUTH_TIMEOUT_MS = 120000;
    private static final String KEY_ALIAS = "jiotvgo_adb_auth_rsa";
    // ADB sends a SHA-1 digest, not the original message. Prefix it with the
    // SHA-1 DigestInfo structure for RSA PKCS#1 v1.5 signing without rehashing.
    private static final byte[] SHA1_DIGEST_INFO = {
            0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a,
            0x05, 0x00, 0x04, 0x14
    };
    private static final AtomicBoolean RUNNING = new AtomicBoolean();
    private static final ExecutorService WORKER = Executors.newSingleThreadExecutor(r -> {
        Thread thread = new Thread(r, "adb-overlay-setup");
        thread.setDaemon(true);
        return thread;
    });

    enum State { SUCCESS, FAILED, UNAVAILABLE, AUTH_REQUIRED, TIMEOUT, BUSY }

    interface Callback {
        void onStatus(String message);
        void onComplete(State state, String message);
    }

    private AdbSetup() { }

    /** Start only from an explicit user action. Callbacks arrive on the main thread. */
    static boolean start(Context context, Callback callback) {
        if (context == null || callback == null) throw new IllegalArgumentException("context and callback required");
        if (!RUNNING.compareAndSet(false, true)) {
            postMain(() -> callback.onComplete(State.BUSY, "ADB setup is already running."));
            return false;
        }
        Context appContext = context.getApplicationContext();
        WORKER.execute(() -> {
            try {
                runSetup(appContext, callback);
            } finally {
                RUNNING.set(false);
            }
        });
        return true;
    }

    private static void runSetup(Context context, Callback callback) {
        try (Socket socket = new Socket()) {
            socket.connect(new InetSocketAddress(LOOPBACK_HOST, LOOPBACK_PORT), CONNECT_TIMEOUT_MS);
            socket.setSoTimeout(AUTH_TIMEOUT_MS);
            AdbWire wire = new AdbWire(socket);
            wire.send(A_CNXN, ADB_VERSION, MAX_DATA,
                    "host::features=shell_v2\0".getBytes(StandardCharsets.UTF_8));
            KeyPair keyPair = getOrCreateKeyPair();
            authenticate(wire, keyPair, callback);
            executeOverlayAppOp(wire, context.getPackageName());
            complete(callback, State.SUCCESS, "ADB connection authorized. Startup screen permission enabled.");
        } catch (java.net.SocketTimeoutException e) {
            complete(callback, State.TIMEOUT,
                    "ADB authorization timed out. Accept the Android ADB key prompt, then try again.");
        } catch (java.net.ConnectException e) {
            complete(callback, State.UNAVAILABLE,
                    "ADB is not listening on 127.0.0.1:5555. Enable authorized ADB on this device, then try again.");
        } catch (Exception e) {
            complete(callback, State.FAILED, "ADB setup failed. Check ADB authorization and try again.");
        }
    }

    private static void authenticate(AdbWire wire, KeyPair keyPair, Callback callback) throws Exception {
        boolean publicKeySent = false;
        long deadline = System.currentTimeMillis() + AUTH_TIMEOUT_MS;
        while (System.currentTimeMillis() < deadline) {
            wire.setReadTimeout((int) Math.max(1, deadline - System.currentTimeMillis()));
            Packet packet = wire.read();
            if (packet.command == A_AUTH && packet.arg0 == AUTH_TOKEN) {
                if (!publicKeySent) {
                    byte[] signature = signToken(keyPair, packet.payload);
                    wire.send(A_AUTH, AUTH_SIGNATURE, 0, signature);
                    // A fresh A_AUTH token means the signature was not recognized.
                    // Sending the public key asks Android to show its normal approval UI.
                    Packet next = wire.read();
                    if (next.command == A_CNXN) return;
                    if (next.command != A_AUTH || next.arg0 != AUTH_TOKEN) {
                        throw new IOException("Unexpected ADB auth response");
                    }
                    wire.send(A_AUTH, AUTH_PUBLIC_KEY, 0, (adbPublicKey((RSAPublicKey) keyPair.getPublic())
                            + " jiotvgo@android\0").getBytes(StandardCharsets.US_ASCII));
                    publicKeySent = true;
                    status(callback, "Waiting for Android's ADB authorization prompt…");
                }
            } else if (packet.command == A_CNXN) {
                // Previously authorized keys receive CNXN without a new prompt.
                return;
            } else {
                throw new IOException("Unexpected ADB handshake packet");
            }
        }
        throw new java.net.SocketTimeoutException("ADB authorization timed out");
    }

    private static void executeOverlayAppOp(AdbWire wire, String packageName) throws Exception {
        if (packageName == null || !packageName.matches("[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*")) {
            throw new IOException("Invalid app package");
        }
        String service = overlayCommandFor(packageName);
        wire.send(A_OPEN, 1, 0, service.getBytes(StandardCharsets.UTF_8));
        Packet packet = wire.read();
        if (packet.command == A_CLSE) throw new IOException("ADB shell service rejected request");
        if (packet.command != A_OKAY || packet.arg1 != 1 || packet.arg0 == 0) {
            throw new IOException("ADB shell stream did not open");
        }
        int remoteId = packet.arg0;
        while (true) {
            packet = wire.read();
            if (packet.command == A_WRTE && packet.arg0 == remoteId && packet.arg1 == 1) {
                wire.send(A_OKAY, 1, remoteId, new byte[0]);
                int exitCode = shellExitCode(packet.payload);
                if (exitCode >= 0) {
                    wire.send(A_CLSE, 1, remoteId, new byte[0]);
                    if (exitCode != 0) throw new IOException("ADB app-op command failed");
                    return;
                }
            } else if (packet.command == A_CLSE && packet.arg1 == 1) {
                throw new IOException("ADB shell closed before reporting command status");
            } else {
                throw new IOException("Unexpected ADB shell packet");
            }
        }
    }

    static String overlayCommandFor(String packageName) {
        if (packageName == null || !packageName.matches("[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*")) {
            throw new IllegalArgumentException("Invalid Android package name");
        }
        return "shell,v2,raw:appops set " + packageName + " SYSTEM_ALERT_WINDOW allow\0";
    }

    /** Returns the shell-v2 exit code, or -1 for a non-exit frame. */
    static int shellExitCode(byte[] frame) throws IOException {
        if (frame.length < 5) throw new IOException("Truncated shell-v2 frame");
        int length = (frame[1] & 0xff) | ((frame[2] & 0xff) << 8)
                | ((frame[3] & 0xff) << 16) | ((frame[4] & 0xff) << 24);
        if (length != frame.length - 5) throw new IOException("Invalid shell-v2 frame length");
        if (frame[0] != 3) return -1;
        if (length != 1) throw new IOException("Invalid shell-v2 exit frame");
        return frame[5] & 0xff;
    }

    private static KeyPair getOrCreateKeyPair() throws Exception {
        KeyStore keyStore = KeyStore.getInstance("AndroidKeyStore");
        keyStore.load(null);
        if (keyStore.containsAlias(KEY_ALIAS)) {
            Key privateKey = keyStore.getKey(KEY_ALIAS, null);
            java.security.cert.Certificate certificate = keyStore.getCertificate(KEY_ALIAS);
            if (privateKey != null && certificate != null) {
                return new KeyPair(certificate.getPublicKey(), (java.security.PrivateKey) privateKey);
            }
            keyStore.deleteEntry(KEY_ALIAS);
        }
        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA", "AndroidKeyStore");
        AlgorithmParameterSpec spec;
        if (android.os.Build.VERSION.SDK_INT >= 23) {
            spec = new KeyGenParameterSpec.Builder(KEY_ALIAS, KeyProperties.PURPOSE_SIGN)
                    .setKeySize(2048)
                    .setDigests(KeyProperties.DIGEST_NONE)
                    .setSignaturePaddings(KeyProperties.SIGNATURE_PADDING_RSA_PKCS1)
                    .setUserAuthenticationRequired(false)
                    .build();
        } else {
            throw new IllegalStateException("Android Keystore RSA requires API 23");
        }
        generator.initialize(spec);
        return generator.generateKeyPair();
    }

    private static byte[] signToken(KeyPair keyPair, byte[] token) throws Exception {
        Signature signature = Signature.getInstance("NONEwithRSA");
        signature.initSign(keyPair.getPrivate());
        signature.update(adbTokenDigestInfo(token));
        return signature.sign();
    }

    static byte[] adbTokenDigestInfo(byte[] token) throws IOException {
        if (token.length != 20) throw new IOException("Invalid ADB AUTH token length");
        byte[] value = Arrays.copyOf(SHA1_DIGEST_INFO, SHA1_DIGEST_INFO.length + token.length);
        System.arraycopy(token, 0, value, SHA1_DIGEST_INFO.length, token.length);
        return value;
    }

    static String adbPublicKey(RSAPublicKey key) {
        final int words = 64;
        final BigInteger radix = BigInteger.ONE.shiftLeft(32);
        final BigInteger modulus = key.getModulus();
        byte[] blob = new byte[4 + 4 + 256 + 256 + 4];
        ByteBuffer out = ByteBuffer.wrap(blob).order(ByteOrder.LITTLE_ENDIAN);
        out.putInt(words);
        byte[] modulusBytes = fixedUnsigned(modulus, 256);
        BigInteger lowWord = new BigInteger(1, Arrays.copyOfRange(modulusBytes, 252, 256));
        long n0inv = lowWord.modInverse(radix).negate().mod(radix).longValue();
        out.putInt((int) n0inv);
        out.put(reversed(modulusBytes, 0, 256));
        BigInteger r = BigInteger.ONE.shiftLeft(2048);
        out.put(reversed(fixedUnsigned(r.multiply(r).mod(modulus), 256), 0, 256));
        out.putInt(key.getPublicExponent().intValue());
        return base64NoPadding(blob);
    }

    private static String base64NoPadding(byte[] bytes) {
        final char[] alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".toCharArray();
        StringBuilder result = new StringBuilder((bytes.length * 4 + 2) / 3);
        for (int i = 0; i < bytes.length; i += 3) {
            int a = bytes[i] & 0xff;
            int b = i + 1 < bytes.length ? bytes[i + 1] & 0xff : 0;
            int c = i + 2 < bytes.length ? bytes[i + 2] & 0xff : 0;
            result.append(alphabet[a >>> 2]);
            result.append(alphabet[((a & 3) << 4) | (b >>> 4)]);
            if (i + 1 < bytes.length) result.append(alphabet[((b & 15) << 2) | (c >>> 6)]);
            if (i + 2 < bytes.length) result.append(alphabet[c & 63]);
        }
        return result.toString();
    }

    private static byte[] fixedUnsigned(BigInteger value, int size) {
        byte[] encoded = value.toByteArray();
        int offset = encoded.length > size ? encoded.length - size : 0;
        int length = encoded.length - offset;
        if (length > size) throw new IllegalArgumentException("RSA component too large");
        byte[] result = new byte[size];
        System.arraycopy(encoded, offset, result, size - length, length);
        return result;
    }

    private static byte[] reversed(byte[] input, int offset, int length) {
        byte[] result = new byte[length];
        for (int i = 0; i < length; i++) result[i] = input[offset + length - 1 - i];
        return result;
    }

    private static void status(Callback callback, String message) {
        postMain(() -> callback.onStatus(message));
    }

    private static void complete(Callback callback, State state, String message) {
        postMain(() -> callback.onComplete(state, message));
    }

    private static void postMain(Runnable callback) {
        new android.os.Handler(android.os.Looper.getMainLooper()).post(callback);
    }

    private static final class Packet {
        final int command, arg0, arg1;
        final byte[] payload;
        Packet(int command, int arg0, int arg1, byte[] payload) {
            this.command = command;
            this.arg0 = arg0;
            this.arg1 = arg1;
            this.payload = payload;
        }
    }

    private static final class AdbWire {
        private final DataInputStream input;
        private final DataOutputStream output;
        private final Socket socket;

        AdbWire(Socket socket) throws IOException {
            this.socket = socket;
            input = new DataInputStream(socket.getInputStream());
            output = new DataOutputStream(socket.getOutputStream());
        }

        void setReadTimeout(int timeout) throws IOException { socket.setSoTimeout(timeout); }

        void send(int command, int arg0, int arg1, byte[] payload) throws IOException {
            if (payload.length > MAX_DATA) throw new IOException("ADB payload exceeds negotiated packet size");
            ByteBuffer header = ByteBuffer.allocate(24).order(ByteOrder.LITTLE_ENDIAN);
            header.putInt(command).putInt(arg0).putInt(arg1).putInt(payload.length)
                    .putInt(checksum(payload)).putInt(command ^ 0xffffffff);
            output.write(header.array());
            output.write(payload);
            output.flush();
        }

        Packet read() throws IOException {
            byte[] headerBytes = new byte[24];
            input.readFully(headerBytes);
            ByteBuffer header = ByteBuffer.wrap(headerBytes).order(ByteOrder.LITTLE_ENDIAN);
            int command = header.getInt();
            int arg0 = header.getInt();
            int arg1 = header.getInt();
            int length = header.getInt();
            int expectedChecksum = header.getInt();
            int magic = header.getInt();
            if ((command ^ 0xffffffff) != magic || length < 0 || length > MAX_PACKET) {
                throw new IOException("Invalid ADB packet header");
            }
            byte[] payload = new byte[length];
            input.readFully(payload);
            if (checksum(payload) != expectedChecksum) throw new IOException("Invalid ADB packet checksum");
            return new Packet(command, arg0, arg1, payload);
        }

        private static int checksum(byte[] bytes) {
            int sum = 0;
            for (byte value : bytes) sum += value & 0xff;
            return sum;
        }
    }
}
