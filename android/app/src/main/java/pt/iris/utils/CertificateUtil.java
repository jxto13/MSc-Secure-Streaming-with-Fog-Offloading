package pt.iris.utils;

import android.content.Context;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.security.KeyStore;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManagerFactory;
import javax.net.ssl.HttpsURLConnection;
import java.net.URL;

public class CertificateUtil {

    public static void copyCertificateToDataFolder(Context context) throws IOException {
        InputStream inputStream = context.getAssets().open("ca_certificate.pem");
        File outFile = new File(context.getFilesDir(), "ca_certificate.pem");
        FileOutputStream outputStream = new FileOutputStream(outFile);

        byte[] buffer = new byte[1024];
        int length;
        while ((length = inputStream.read(buffer)) > 0) {
            outputStream.write(buffer, 0, length);
        }

        outputStream.close();
        inputStream.close();
    }

    public static SSLContext loadCertificate(Context context) throws Exception {
        // Load the certificate from the assets folder
        InputStream certInputStream = context.getAssets().open("ca_certificate.pem");

        // Create a CertificateFactory and generate a certificate from the InputStream
        CertificateFactory certificateFactory = CertificateFactory.getInstance("X.509");
        Certificate certificate = certificateFactory.generateCertificate(certInputStream);

        // Create a KeyStore and load the certificate into it
        KeyStore keyStore = KeyStore.getInstance(KeyStore.getDefaultType());
        keyStore.load(null, null); // Initialize an empty KeyStore
        keyStore.setCertificateEntry("ca", certificate);

        // Create a TrustManager that trusts the CAs in our KeyStore
        TrustManagerFactory tmf = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());
        tmf.init(keyStore);

        // Create an SSLContext that uses our TrustManager
        SSLContext sslContext = SSLContext.getInstance("TLS");
        sslContext.init(null, tmf.getTrustManagers(), null);

        // Close the InputStream
        certInputStream.close();

        return sslContext;
    }

    public static void useCertificateForHttps(Context context) throws Exception {
        // Load the SSLContext with the certificate
        SSLContext sslContext = loadCertificate(context);

        // Example usage with an HTTPS connection
        URL url = new URL("https://example.com");
        HttpsURLConnection urlConnection = (HttpsURLConnection) url.openConnection();
        urlConnection.setSSLSocketFactory(sslContext.getSocketFactory());

        // Read the response
        InputStream in = urlConnection.getInputStream();
        // Process the input stream as needed
        in.close();
    }
}
