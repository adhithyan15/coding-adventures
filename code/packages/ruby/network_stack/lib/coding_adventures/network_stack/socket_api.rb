# frozen_string_literal: true

# ============================================================================
# Socket API — The Berkeley Sockets Interface
# ============================================================================
#
# The Socket API is the application-facing interface to the network stack.
# It was invented at UC Berkeley in 1983 and has been the standard network
# programming interface ever since. Every operating system implements it
# (Linux, macOS, Windows), and every networked application uses it.
#
# The key insight of the Berkeley Sockets API is that network connections
# are treated like files — you open them, read/write data, and close them.
# This fits perfectly into Unix's "everything is a file" philosophy.
#
# The basic workflow for a TCP client:
#
#   fd = socket(STREAM)           # Create a socket
#   bind(fd, ip, port)            # Optional: bind to a specific address
#   connect(fd, server_ip, 80)    # Connect to a server
#   send(fd, "GET / HTTP/1.1")    # Send data
#   data = recv(fd)               # Receive response
#   close(fd)                     # Tear down the connection
#
# The basic workflow for a TCP server:
#
#   fd = socket(STREAM)           # Create a socket
#   bind(fd, ip, 80)              # Bind to port 80
#   listen(fd, 5)                 # Start listening (queue up to 5)
#   client_fd = accept(fd)        # Wait for a client connection
#   data = recv(client_fd)        # Receive request
#   send(client_fd, response)     # Send response
#   close(client_fd)              # Close this connection
#   close(fd)                     # Close the server socket
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    # Socket type constants — these tell the kernel what kind of socket to create.
    #
    # STREAM = TCP: reliable, ordered, connection-oriented byte stream.
    # DGRAM  = UDP: unreliable, unordered, connectionless datagrams.
    #
    module SocketType
      STREAM = 1  # TCP
      DGRAM  = 2  # UDP
    end

    # ========================================================================
    # Socket — A Single Network Endpoint
    # ========================================================================
    #
    # A socket represents one end of a network connection (or potential
    # connection). It stores:
    #   - A file descriptor (fd) that identifies it to the application
    #   - The socket type (TCP or UDP)
    #   - Local and remote addresses
    #   - The underlying TCPConnection or UDPSocket
    #   - Whether it's a listening (server) socket
    #   - A queue of accepted connections (for server sockets)
    #
    # ========================================================================
    class Socket
      attr_accessor :fd, :socket_type, :local_ip, :local_port,
        :remote_ip, :remote_port, :tcp_connection, :udp_socket,
        :listening, :accept_queue

      def initialize(fd:, socket_type:)
        @fd             = fd
        @socket_type    = socket_type
        @local_ip       = nil
        @local_port     = nil
        @remote_ip      = nil
        @remote_port    = nil
        @tcp_connection = nil
        @udp_socket     = nil
        @listening      = false
        @accept_queue   = []
      end

      def listening?
        @listening
      end
    end

    # ========================================================================
    # SocketManager — Manages All Sockets in the System
    # ========================================================================
    #
    # The SocketManager is the kernel-side component that implements the
    # Berkeley Sockets API. It allocates file descriptors, tracks all open
    # sockets, and dispatches operations to the appropriate protocol handler.
    #
    # In a real kernel, the SocketManager would be part of the network
    # subsystem and would interact with the VFS (Virtual File System) to
    # make sockets behave like files.
    #
    # ========================================================================
    class SocketManager
      attr_reader :sockets

      def initialize
        @sockets   = {}
        @next_fd   = 3  # 0=stdin, 1=stdout, 2=stderr; sockets start at 3
        @used_ports = {}
      end

      # Create a new socket. Returns a file descriptor (integer).
      #
      # This is analogous to the socket(2) system call.
      #
      def create_socket(socket_type)
        fd = @next_fd
        @next_fd += 1

        sock = Socket.new(fd: fd, socket_type: socket_type)
        @sockets[fd] = sock
        fd
      end

      # Bind a socket to a local IP address and port.
      #
      # Returns true on success, false if the port is already in use.
      # In a real kernel, binding to port 0 would auto-assign an ephemeral port.
      #
      def bind(fd, ip, port)
        sock = @sockets[fd]
        return false unless sock

        # Check if port is already in use
        return false if @used_ports[port]

        sock.local_ip   = ip
        sock.local_port = port
        @used_ports[port] = fd

        # Create the underlying protocol socket
        if sock.socket_type == SocketType::DGRAM
          sock.udp_socket = UDPSocket.new(local_port: port)
        end

        true
      end

      # Mark a TCP socket as listening for incoming connections.
      #
      # The backlog parameter limits how many connections can queue up
      # before the kernel starts refusing new ones. Typical values are
      # 5 (old default), 128 (modern Linux), or SOMAXCONN.
      #
      def listen(fd, backlog = 5)
        sock = @sockets[fd]
        return false unless sock
        return false unless sock.socket_type == SocketType::STREAM

        sock.listening = true
        sock.tcp_connection = TCPConnection.new(local_port: sock.local_port)
        sock.tcp_connection.initiate_listen
        true
      end

      # Accept an incoming connection on a listening socket.
      #
      # Returns [new_fd, remote_ip, remote_port] or nil if no connections
      # are waiting in the accept queue.
      #
      def accept(fd)
        sock = @sockets[fd]
        return nil unless sock&.listening?
        return nil if sock.accept_queue.empty?

        conn_info = sock.accept_queue.shift
        new_fd = create_socket(SocketType::STREAM)
        new_sock = @sockets[new_fd]
        new_sock.local_ip       = sock.local_ip
        new_sock.local_port     = sock.local_port
        new_sock.remote_ip      = conn_info[:remote_ip]
        new_sock.remote_port    = conn_info[:remote_port]
        new_sock.tcp_connection = conn_info[:connection]

        [new_fd, conn_info[:remote_ip], conn_info[:remote_port]]
      end

      # Initiate a TCP connection to a remote host.
      #
      # Returns the SYN header to send, or nil on failure.
      #
      def connect(fd, remote_ip, remote_port)
        sock = @sockets[fd]
        return nil unless sock
        return nil unless sock.socket_type == SocketType::STREAM

        # Auto-assign an ephemeral port if not bound
        unless sock.local_port
          port = allocate_ephemeral_port
          sock.local_port = port
          @used_ports[port] = fd
        end

        sock.remote_ip   = remote_ip
        sock.remote_port = remote_port
        sock.tcp_connection = TCPConnection.new(
          local_port: sock.local_port,
          remote_ip: remote_ip,
          remote_port: remote_port
        )
        sock.tcp_connection.initiate_connect(remote_ip, remote_port)
      end

      # Send data on a connected socket.
      #
      # For TCP: adds data to the send buffer and returns a segment header.
      # For UDP: creates and returns [header, data].
      #
      def send_data(fd, data)
        sock = @sockets[fd]
        return nil unless sock

        if sock.socket_type == SocketType::STREAM
          sock.tcp_connection&.send_data(data)
        elsif sock.socket_type == SocketType::DGRAM && sock.udp_socket
          sock.udp_socket.send_to(data, sock.remote_port || 0)
        end
      end

      # Receive data from a socket.
      #
      # For TCP: reads from the receive buffer.
      # For UDP: dequeues the next datagram.
      #
      def recv(fd, count = 65535)
        sock = @sockets[fd]
        return nil unless sock

        if sock.socket_type == SocketType::STREAM
          sock.tcp_connection&.receive(count)
        elsif sock.socket_type == SocketType::DGRAM && sock.udp_socket
          sock.udp_socket.receive_from
        end
      end

      # Send a UDP datagram to a specific destination (no connection needed).
      def sendto(fd, data, dst_ip, dst_port)
        sock = @sockets[fd]
        return nil unless sock
        return nil unless sock.socket_type == SocketType::DGRAM && sock.udp_socket

        sock.udp_socket.send_to(data, dst_port)
      end

      # Receive a UDP datagram (returns data + sender info).
      def recvfrom(fd)
        sock = @sockets[fd]
        return nil unless sock
        return nil unless sock.socket_type == SocketType::DGRAM && sock.udp_socket

        sock.udp_socket.receive_from
      end

      # Close a socket and free its resources.
      #
      # For TCP, this initiates the connection teardown (sends FIN).
      # Returns the FIN header for TCP, or true for UDP.
      #
      def close(fd)
        sock = @sockets[fd]
        return false unless sock

        result = nil
        if sock.socket_type == SocketType::STREAM && sock.tcp_connection
          result = sock.tcp_connection.initiate_close
        end

        @used_ports.delete(sock.local_port)
        @sockets.delete(fd)
        result || true
      end

      private

      # Allocate an ephemeral port (49152-65535 range, per IANA).
      def allocate_ephemeral_port
        port = 49152
        port += 1 while @used_ports[port]
        port
      end
    end
  end
end
