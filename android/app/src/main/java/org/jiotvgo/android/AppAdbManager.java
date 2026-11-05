package org.jiotvgo.android;

import android.content.Context;
import android.os.Build;
import androidx.annotation.NonNull;
import io.github.muntashirakon.adb.AbsAdbConnectionManager;
import io.github.muntashirakon.adb.AdbStream;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.security.*;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.util.Base64;
import java.math.BigInteger;
import org.bouncycastle.asn1.x500.X500Name;
import org.bouncycastle.cert.jcajce.JcaX509CertificateConverter;
import org.bouncycastle.cert.jcajce.JcaX509v3CertificateBuilder;
import org.bouncycastle.jce.provider.BouncyCastleProvider;
import org.bouncycastle.operator.jcajce.JcaContentSignerBuilder;
import java.security.spec.PKCS8EncodedKeySpec;
import java.util.Date;
import java.util.concurrent.TimeUnit;

/** Restricts the ADB protocol library to one appops request. */
final class AppAdbManager extends AbsAdbConnectionManager {
    private static final String COMMAND = "shell:appops set org.jiotvgo.android SYSTEM_ALERT_WINDOW allow";
    private static final String KEY_FILE = "startup-adb-key.pk8";
    private static final String CERT_FILE = "startup-adb-cert.pem";
    private final PrivateKey privateKey;
    private final Certificate certificate;

    AppAdbManager(Context context) throws Exception {
        setApi(Build.VERSION.SDK_INT);
        setTimeout(45, TimeUnit.SECONDS);
        // False sends the public key after the first rejected signature, which triggers Android's authorization prompt for this new app identity. True would abort before the prompt.
        setThrowOnUnauthorised(false);
        File key = new File(context.getNoBackupFilesDir(), KEY_FILE);
        File cert = new File(context.getNoBackupFilesDir(), CERT_FILE);
        if (key.isFile() && cert.isFile()) {
            privateKey = KeyFactory.getInstance("RSA").generatePrivate(new PKCS8EncodedKeySpec(readBounded(key, 8192)));
            try (InputStream in = new FileInputStream(cert)) { certificate = CertificateFactory.getInstance("X.509").generateCertificate(in); }
        } else {
            java.security.KeyPairGenerator generator = java.security.KeyPairGenerator.getInstance("RSA");
            generator.initialize(2048); java.security.KeyPair pair = generator.generateKeyPair();
            privateKey = pair.getPrivate();
            String alg = "SHA256withRSA"; Date from = new Date(System.currentTimeMillis() - 86400000L);
            Date to = new Date(System.currentTimeMillis() + 3650L * 86400000L);
            X500Name name = new X500Name("CN=JioTV Go startup permission");
            JcaX509v3CertificateBuilder builder = new JcaX509v3CertificateBuilder(name,
                    BigInteger.valueOf(new SecureRandom().nextInt() & Integer.MAX_VALUE), from, to, name, pair.getPublic());
            BouncyCastleProvider provider = new BouncyCastleProvider();
            certificate = new JcaX509CertificateConverter().setProvider(provider).getCertificate(
                    builder.build(new JcaContentSignerBuilder(alg).setProvider(provider).build(privateKey)));
            writeAtomic(key, privateKey.getEncoded());
            String encoded = Base64.getMimeEncoder(64, new byte[] {'\n'}).encodeToString(certificate.getEncoded());
            byte[] pem = ("-----BEGIN CERTIFICATE-----\n" + encoded + "\n-----END CERTIFICATE-----\n").getBytes(StandardCharsets.UTF_8);
            writeAtomic(cert, pem);
        }
    }
    private static byte[] readBounded(File f, int max) throws IOException { if (f.length() <= 0 || f.length() > max) throw new IOException("Invalid private ADB identity"); byte[] b = new byte[(int)f.length()]; try (InputStream in=new FileInputStream(f)) { int off=0,n; while(off<b.length && (n=in.read(b,off,b.length-off))>0) off+=n; if(off!=b.length) throw new EOFException(); } return b; }
    private static void writeAtomic(File target, byte[] bytes) throws IOException { File temp=new File(target.getParentFile(),target.getName()+".tmp"); try(FileOutputStream out=new FileOutputStream(temp)){out.write(bytes);out.getFD().sync();} if(!temp.renameTo(target)){temp.delete();throw new IOException("Could not save ADB identity");} }
    @NonNull @Override protected PrivateKey getPrivateKey(){return privateKey;}
    @NonNull @Override protected Certificate getCertificate(){return certificate;}
    @NonNull @Override protected String getDeviceName(){return "JioTV Go";}

    void runStartupGrant() throws Exception {
        try (AdbStream stream = openStream(COMMAND); InputStream in = stream.openInputStream()) {
            byte[] buf = new byte[256]; int total=0, n;
            while (true) {
                try {
                    n = in.read(buf);
                } catch (IOException e) {
                    if (stream.isClosed() && "Stream closed.".equals(e.getMessage())) break;
                    throw e;
                }
                if (n == -1) break;
                total += n;
                if (total > 4096) throw new IOException("ADB response exceeded limit");
            }
        }
    }
}
