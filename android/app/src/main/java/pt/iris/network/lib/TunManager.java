package pt.iris.network.lib;

import android.content.Context;
import android.net.VpnService;
import android.os.ParcelFileDescriptor;

import java.net.Inet6Address;
import java.net.UnknownHostException;
import java.security.SecureRandom;

public class TunManager {
    private final Context context;
    private final VpnService vpnService;
    private VpnService.Builder builder;
    private ParcelFileDescriptor tunInterface;
    private String ulaAddress;

    // Fixed ULA subnet prefix (first 64 bits)
    private static final byte[] ULA_PREFIX = new byte[] {
            (byte) 0xfd, 0x00, 0x12, 0x34, // fd00:1234::
            0x00, 0x00, 0x00, 0x00
    };

    public TunManager(Context context, VpnService vpnService) {
        this.context = context;
        this.vpnService = vpnService;
    }

    /**
     * Spins up a TUN interface with a random ULA IPv6 address
     * in the fixed fd00:1234::/64 subnet.
     *
     * @return The assigned ULA IPv6 address as a string.
     */
    public String spinTun() throws Exception {
        builder = vpnService.new Builder();
        ulaAddress = generateRandomUlaInSubnet();

        builder.addAllowedApplication(context.getPackageName());

        // Configure IPv6 TUN interface with /64 mask
        tunInterface = builder
                .addAddress(ulaAddress, 64)
                .addRoute("fd00:1234::", 64) // route only your ULA subnet
                .setSession("TunManagerSession")
                .establish();

        if (tunInterface == null) {
            throw new Exception("Failed to establish TUN interface");
        }

        return ulaAddress;
    }

    /**
     * Generates a random IPv6 address within the fixed ULA subnet fd00:1234::/64
     */
    private String generateRandomUlaInSubnet() throws UnknownHostException {
        SecureRandom random = new SecureRandom();
        byte[] addr = new byte[16];

        // Copy prefix first 8 bytes (64 bits)
        System.arraycopy(ULA_PREFIX, 0, addr, 0, ULA_PREFIX.length);

        // Generate random interface ID (last 64 bits)
        byte[] interfaceId = new byte[8];
        random.nextBytes(interfaceId);
        System.arraycopy(interfaceId, 0, addr, 8, 8);

        Inet6Address inet6 = (Inet6Address) Inet6Address.getByAddress(addr);
        return inet6.getHostAddress();
    }

    /**
     * Closes the TUN interface.
     */
    public void closeTun() {
        try {
            if (tunInterface != null) {
                tunInterface.close();
                tunInterface = null;
            }
        } catch (Exception ignored) {}
    }

    /**
     * Gets the TUN interface file descriptor.
     */
    public ParcelFileDescriptor getTunInterface() {
        return tunInterface;
    }
}
