package com.codingadventures.ircserver;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

/**
 * End-to-end tests: start the real Rust IRC engine on an ephemeral port and
 * drive live IRC clients over real TCP sockets.
 */
class IrcServerTest {

    private static Socket connect(int port) throws IOException {
        Socket sock = new Socket();
        sock.connect(new InetSocketAddress("127.0.0.1", port), 2000);
        sock.setSoTimeout(300);
        return sock;
    }

    private static String recvUntil(Socket sock, String needle) throws IOException {
        long deadline = System.currentTimeMillis() + 5000;
        StringBuilder buf = new StringBuilder();
        byte[] chunk = new byte[4096];
        InputStream in = sock.getInputStream();
        while (System.currentTimeMillis() < deadline) {
            try {
                int n = in.read(chunk);
                if (n < 0) {
                    break;
                }
                buf.append(new String(chunk, 0, n, StandardCharsets.UTF_8));
                if (buf.indexOf(needle) >= 0) {
                    break;
                }
            } catch (java.net.SocketTimeoutException e) {
                // keep polling until the deadline
            }
        }
        return buf.toString();
    }

    private static void send(Socket sock, String line) throws IOException {
        OutputStream out = sock.getOutputStream();
        out.write(line.getBytes(StandardCharsets.UTF_8));
        out.flush();
    }

    private static void register(Socket sock, String nick) throws IOException {
        send(sock, "NICK " + nick + "\r\nUSER " + nick + " 0 * :" + nick + "\r\n");
        String welcome = recvUntil(sock, "001");
        assertTrue(welcome.contains("001"), "expected 001 welcome for " + nick);
    }

    @Test
    void reportsEphemeralBoundAddress() {
        try (IrcServer server = IrcServer.builder().port(0).build()) {
            assertEquals("127.0.0.1", server.localHost());
            assertTrue(server.localPort() > 0);
        }
    }

    @Test
    void registrationAndPing() throws IOException, InterruptedException {
        try (IrcServer server = IrcServer.builder().port(0).serverName("irc.test").build()) {
            server.serveBackground();
            Thread.sleep(100);
            try (Socket alice = connect(server.localPort())) {
                register(alice, "alice");
                send(alice, "PING :liveness\r\n");
                String pong = recvUntil(alice, "PONG");
                assertTrue(pong.contains("PONG"), "expected PONG, got: " + pong);
            }
            server.stop();
        }
    }

    @Test
    void privmsgBroadcastsBetweenClients() throws IOException, InterruptedException {
        try (IrcServer server = IrcServer.builder().port(0).serverName("irc.test").build()) {
            server.serveBackground();
            Thread.sleep(100);
            try (Socket alice = connect(server.localPort());
                 Socket bob = connect(server.localPort())) {
                register(alice, "alice");
                register(bob, "bob");
                send(alice, "JOIN #test\r\n");
                send(bob, "JOIN #test\r\n");
                recvUntil(alice, "JOIN");
                recvUntil(bob, "JOIN");

                // Alice speaks; Bob (a different connection) must receive it —
                // this exercises the Rust engine's in-process mailbox fan-out.
                send(alice, "PRIVMSG #test :hello bob\r\n");
                String received = recvUntil(bob, "hello bob");
                assertTrue(received.contains("PRIVMSG") && received.contains("hello bob"),
                    "bob should receive alice's broadcast, got: " + received);
            }
            server.stop();
        }
    }

    @Test
    void runningFlipsAfterServe() throws InterruptedException {
        try (IrcServer server = IrcServer.builder().port(0).build()) {
            assertTrue(!server.running());
            server.serveBackground();
            long deadline = System.currentTimeMillis() + 5000;
            while (!server.running() && System.currentTimeMillis() < deadline) {
                Thread.sleep(10);
            }
            assertTrue(server.running());
            server.stop();
        }
    }
}
