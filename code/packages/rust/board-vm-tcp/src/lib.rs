use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use board_vm_client::{RawFrameTransport, TransportError};
use board_vm_stream::{StreamTransport, StreamTransportError};

pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 1_000;
pub const DEFAULT_IO_TIMEOUT_MS: u64 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpConfig {
    pub endpoint: String,
    pub connect_timeout: Duration,
    pub io_timeout: Duration,
    pub nodelay: bool,
}

impl TcpConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            connect_timeout: Duration::from_millis(DEFAULT_CONNECT_TIMEOUT_MS),
            io_timeout: Duration::from_millis(DEFAULT_IO_TIMEOUT_MS),
            nodelay: true,
        }
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn io_timeout(mut self, timeout: Duration) -> Self {
        self.io_timeout = timeout;
        self
    }

    pub fn nodelay(mut self, nodelay: bool) -> Self {
        self.nodelay = nodelay;
        self
    }

    pub fn authority(&self) -> &str {
        tcp_endpoint_authority(&self.endpoint)
    }
}

#[derive(Debug)]
pub enum TcpTransportError {
    Resolve(io::Error),
    NoResolvedAddress,
    Connect(io::Error),
    Configure(io::Error),
    Stream(StreamTransportError),
}

impl From<StreamTransportError> for TcpTransportError {
    fn from(value: StreamTransportError) -> Self {
        Self::Stream(value)
    }
}

pub struct BoardTcpTransport<S, const WIRE_BYTES: usize = 1024> {
    inner: StreamTransport<S, WIRE_BYTES>,
}

impl<S, const WIRE_BYTES: usize> BoardTcpTransport<S, WIRE_BYTES> {
    pub fn from_stream(stream: S) -> Self {
        Self {
            inner: StreamTransport::new(stream),
        }
    }

    pub fn into_inner(self) -> StreamTransport<S, WIRE_BYTES> {
        self.inner
    }

    pub fn stream_transport(&self) -> &StreamTransport<S, WIRE_BYTES> {
        &self.inner
    }

    pub fn stream_transport_mut(&mut self) -> &mut StreamTransport<S, WIRE_BYTES> {
        &mut self.inner
    }

    pub fn send_raw_frame(&mut self, raw_frame: &[u8]) -> Result<usize, TcpTransportError>
    where
        S: Write,
    {
        Ok(self.inner.send_raw_frame(raw_frame)?)
    }

    pub fn receive_raw_frame(&mut self, raw_out: &mut [u8]) -> Result<usize, TcpTransportError>
    where
        S: Read,
    {
        Ok(self.inner.receive_raw_frame(raw_out)?)
    }

    pub fn exchange_raw_frame_checked(
        &mut self,
        raw_request: &[u8],
        raw_response_out: &mut [u8],
    ) -> Result<usize, TcpTransportError>
    where
        S: Read + Write,
    {
        Ok(self
            .inner
            .exchange_raw_frame_checked(raw_request, raw_response_out)?)
    }
}

impl<const WIRE_BYTES: usize> BoardTcpTransport<TcpStream, WIRE_BYTES> {
    pub fn connect(config: &TcpConfig) -> Result<Self, TcpTransportError> {
        let address = resolve_first_socket_addr(config.authority())?;
        let stream = TcpStream::connect_timeout(&address, config.connect_timeout)
            .map_err(TcpTransportError::Connect)?;
        stream
            .set_read_timeout(Some(config.io_timeout))
            .map_err(TcpTransportError::Configure)?;
        stream
            .set_write_timeout(Some(config.io_timeout))
            .map_err(TcpTransportError::Configure)?;
        stream
            .set_nodelay(config.nodelay)
            .map_err(TcpTransportError::Configure)?;
        Ok(Self::from_stream(stream))
    }
}

impl<S, const WIRE_BYTES: usize> RawFrameTransport for BoardTcpTransport<S, WIRE_BYTES>
where
    S: Read + Write,
{
    fn exchange_raw_frame(
        &mut self,
        request: &[u8],
        response_out: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.inner.exchange_raw_frame(request, response_out)
    }
}

pub fn tcp_endpoint_authority(endpoint: &str) -> &str {
    endpoint.strip_prefix("tcp://").unwrap_or(endpoint)
}

fn resolve_first_socket_addr(endpoint: &str) -> Result<SocketAddr, TcpTransportError> {
    let mut addresses = endpoint
        .to_socket_addrs()
        .map_err(TcpTransportError::Resolve)?;
    addresses.next().ok_or(TcpTransportError::NoResolvedAddress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    use board_vm_client::BoardVmClient;
    use board_vm_loopback::LoopbackBoard;

    #[test]
    fn strips_tcp_scheme_from_endpoint_authority() {
        assert_eq!(
            tcp_endpoint_authority("tcp://board-vm.local:4170"),
            "board-vm.local:4170"
        );
        assert_eq!(tcp_endpoint_authority("127.0.0.1:4170"), "127.0.0.1:4170");
    }

    #[test]
    fn tcp_transport_exchanges_board_vm_frames() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut transport = StreamTransport::<_, 1024>::new(stream);
            let mut board = LoopbackBoard::<256, 8, 8>::new();
            let mut request_raw = [0u8; 768];
            let mut response_payload = [0u8; 512];
            let mut response_raw = [0u8; 768];

            let request_len = transport.receive_raw_frame(&mut request_raw).unwrap();
            let response_len = board
                .handle_raw_frame(
                    &request_raw[..request_len],
                    &mut response_payload,
                    &mut response_raw,
                )
                .unwrap();
            transport
                .send_raw_frame(&response_raw[..response_len])
                .unwrap();
        });

        let config = TcpConfig::new(format!("tcp://{address}"))
            .connect_timeout(Duration::from_millis(500))
            .io_timeout(Duration::from_millis(500));
        let transport = BoardTcpTransport::<_, 1024>::connect(&config).unwrap();
        let mut client: BoardVmClient<_, 512, 768, 768> = BoardVmClient::new(transport);

        let hello = client.hello_with_name("tcp-test", 0xCAFE_BABE).unwrap();

        assert_eq!(hello.host_nonce, 0xCAFE_BABE);
        server.join().unwrap();
    }
}
