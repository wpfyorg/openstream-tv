package org.jiotvgo.android;

import org.junit.Test;

import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.interfaces.RSAPublicKey;
import java.util.Base64;
import javax.crypto.Cipher;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

public class AdbSetupTest {
    @Test public void commandIsFixedToOverlayPermissionAndValidatedPackage() {
        assertEquals("shell,v2,raw:appops set org.jiotvgo.android SYSTEM_ALERT_WINDOW allow\0",
                AdbSetup.overlayCommandFor("org.jiotvgo.android"));
        assertThrows(IllegalArgumentException.class, () -> AdbSetup.overlayCommandFor("org.example; reboot"));
        assertThrows(IllegalArgumentException.class, () -> AdbSetup.overlayCommandFor(null));
    }

    @Test public void adbPublicKeyUsesAndroidRsaPublicKeyBlobLayout() throws Exception {
        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        KeyPair pair = generator.generateKeyPair();
        RSAPublicKey publicKey = (RSAPublicKey) pair.getPublic();
        byte[] blob = Base64.getDecoder().decode(AdbSetup.adbPublicKey(publicKey));

        assertEquals(524, blob.length);
        ByteBuffer fields = ByteBuffer.wrap(blob).order(ByteOrder.LITTLE_ENDIAN);
        assertEquals(64, fields.getInt());
        int n0inv = fields.getInt();
        byte[] modulusLe = new byte[256];
        fields.get(modulusLe);
        byte[] rr = new byte[256];
        fields.get(rr);
        assertEquals(65537, fields.getInt());

        BigInteger lowWord = new BigInteger(1, new byte[] {
                modulusLe[3], modulusLe[2], modulusLe[1], modulusLe[0]
        });
        BigInteger product = lowWord.multiply(BigInteger.valueOf(n0inv & 0xffffffffL))
                .and(BigInteger.ONE.shiftLeft(32).subtract(BigInteger.ONE));
        assertEquals(BigInteger.ONE.shiftLeft(32).subtract(BigInteger.ONE), product);
        assertTrue(new BigInteger(1, rr).signum() > 0);
    }

    @Test public void shellV2ExitFrameReturnsStatusAndOutputFrameIsIgnored() throws Exception {
        assertEquals(0, AdbSetup.shellExitCode(new byte[] {3, 1, 0, 0, 0, 0}));
        assertEquals(7, AdbSetup.shellExitCode(new byte[] {3, 1, 0, 0, 0, 7}));
        assertEquals(-1, AdbSetup.shellExitCode(new byte[] {1, 2, 0, 0, 0, 'o', 'k'}));
        assertThrows(java.io.IOException.class,
                () -> AdbSetup.shellExitCode(new byte[] {3, 2, 0, 0, 0, 0}));
    }

    @Test public void endpointIsExplicitLoopbackAndPort() {
        assertEquals("127.0.0.1", AdbSetup.LOOPBACK_HOST);
        assertEquals(5555, AdbSetup.LOOPBACK_PORT);
        assertFalse(AdbSetup.LOOPBACK_HOST.isEmpty());
    }

    @Test public void adbAuthSignsOriginalDigestWithoutHashingItAgain() throws Exception {
        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        KeyPair pair = generator.generateKeyPair();
        byte[] token = new byte[20];
        for (int i = 0; i < token.length; i++) token[i] = (byte)(i + 1);
        byte[] digestInfo = AdbSetup.adbTokenDigestInfo(token);
        assertEquals(35, digestInfo.length);
        assertThrows(java.io.IOException.class, () -> AdbSetup.adbTokenDigestInfo(new byte[19]));

        java.security.Signature signature = java.security.Signature.getInstance("NONEwithRSA");
        signature.initSign(pair.getPrivate());
        signature.update(digestInfo);
        Cipher cipher = Cipher.getInstance("RSA/ECB/PKCS1Padding");
        cipher.init(Cipher.DECRYPT_MODE, pair.getPublic());
        org.junit.Assert.assertArrayEquals(digestInfo, cipher.doFinal(signature.sign()));
    }
}
