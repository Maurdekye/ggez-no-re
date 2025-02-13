use std::{
    io::{self, ErrorKind, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    ops::{Deref, DerefMut},
    sync::mpsc::{Receiver, Sender, channel},
    thread::{self, JoinHandle},
    time::Duration,
};

use log::{debug, error};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const MAX_PACKET_SIZE: usize = 16 * 1024 * 1024;

pub trait ClientServerMessage {
    type ClientMessage;
    type ServerMessage;
}

#[derive(Debug)]
pub enum NetworkEvent<T, M> {
    Connect {
        transport: T,
        my_socket_addr: SocketAddr,
    },
    Message(M),
    Disconnect,
}

pub type ServerNetworkEvent<M> =
    NetworkEvent<ServersideTransport, <M as ClientServerMessage>::ClientMessage>;
pub type ClientNetworkEvent<M> =
    NetworkEvent<ClientsideTransport, <M as ClientServerMessage>::ServerMessage>;

#[derive(Debug)]
pub struct MessageTransporter(TcpStream);

impl MessageTransporter {
    fn new(stream: TcpStream) -> Self {
        Self(stream)
    }

    fn try_clone(&self) -> Result<Self, io::Error> {
        Ok(MessageTransporter(self.0.try_clone()?))
    }

    fn send<M>(&mut self, message: &M) -> Result<(), io::Error>
    where
        M: Serialize,
    {
        let encoded_message: Vec<u8> =
            bincode::serialize(message).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        let len = u64::to_le_bytes(encoded_message.len() as u64);
        self.0.write_all(&len)?;
        self.0.write_all(&encoded_message)?;
        Ok(())
    }

    fn recv<M>(&mut self) -> Result<M, io::Error>
    where
        M: DeserializeOwned,
    {
        let mut len_buf = [0u8; 8];
        self.0.read_exact(&mut len_buf)?;
        let len = u64::from_le_bytes(len_buf) as usize;
        if len > MAX_PACKET_SIZE {
            return Err(io::Error::new(
                ErrorKind::FileTooLarge,
                format!("Message size cannot exceed {} bytes", MAX_PACKET_SIZE),
            ));
        }
        let mut buf = vec![0; len];
        self.0.read_exact(&mut buf)?;
        let message: M = bincode::deserialize(&buf[..])
            .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        Ok(message)
    }

    pub fn shutdown(&mut self) -> Result<(), std::io::Error> {
        self.0.shutdown(std::net::Shutdown::Both)
    }
}

impl Deref for MessageTransporter {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct ClientsideTransport(MessageTransporter);

impl ClientsideTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self(MessageTransporter::new(stream))
    }

    pub fn try_clone(&self) -> Result<Self, io::Error> {
        Ok(Self(self.0.try_clone()?))
    }

    pub fn send<M>(&mut self, message: M::ClientMessage) -> Result<(), io::Error>
    where
        M: ClientServerMessage + From<M::ClientMessage> + Serialize,
    {
        self.0.send(&M::from(message))
    }

    pub fn blind_send<M>(&mut self, message: M::ClientMessage)
    where
        M: ClientServerMessage + From<M::ClientMessage> + Serialize,
    {
        let _ = self.send::<M>(message);
    }

    pub fn recv<M>(&mut self) -> Result<M::ServerMessage, io::Error>
    where
        M: ClientServerMessage + DeserializeOwned,
        M::ServerMessage: TryFrom<M>,
    {
        let Ok(message) = self.0.recv::<M>()?.try_into() else {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Received a clientside message from the server",
            ));
        };
        Ok(message)
    }
}

impl Deref for ClientsideTransport {
    type Target = MessageTransporter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ClientsideTransport {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct ServersideTransport(MessageTransporter);

impl ServersideTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self(MessageTransporter::new(stream))
    }

    pub fn try_clone(&self) -> Result<Self, io::Error> {
        Ok(Self(self.0.try_clone()?))
    }

    pub fn send<M>(&mut self, message: M::ServerMessage) -> Result<(), io::Error>
    where
        M: ClientServerMessage + From<M::ServerMessage> + Serialize,
    {
        self.0.send(&M::from(message))
    }

    pub fn blind_send<M>(&mut self, message: M::ServerMessage)
    where
        M: ClientServerMessage + From<M::ServerMessage> + Serialize,
    {
        let _ = self.send::<M>(message);
    }

    pub fn recv<M>(&mut self) -> Result<M::ClientMessage, io::Error>
    where
        M: ClientServerMessage + DeserializeOwned,
        M::ClientMessage: TryFrom<M>,
    {
        let Ok(message) = self.0.recv::<M>()?.try_into() else {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Received a serverside message from a client",
            ));
        };
        Ok(message)
    }
}

impl Deref for ServersideTransport {
    type Target = MessageTransporter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ServersideTransport {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Serialize, Deserialize)]
struct SocketMessage(SocketAddr);

pub struct MessageServer {
    listener_thread: Option<JoinHandle<()>>,
    thread_kill: Sender<()>,
}

impl MessageServer {
    pub fn start<M>(
        event_sender: Sender<impl From<(IpAddr, ServerNetworkEvent<M>)> + Send + 'static>,
        port: u16,
    ) -> Self
    where
        M: ClientServerMessage + DeserializeOwned,
        M::ClientMessage: TryFrom<M>,
    {
        let (thread_kill, deathswitch) = channel();
        let listener_thread = {
            Some(thread::spawn(move || {
                Self::listener_thread::<M>(event_sender, deathswitch, port)
            }))
        };
        MessageServer {
            listener_thread,
            thread_kill,
        }
    }

    fn listener_thread<M>(
        event_sender: Sender<impl From<(IpAddr, ServerNetworkEvent<M>)> + Send + 'static>,
        deathswitch: Receiver<()>,
        port: u16,
    ) where
        M: ClientServerMessage + DeserializeOwned,
        M::ClientMessage: TryFrom<M>,
    {
        debug!("message server starting");
        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
        let listener = TcpListener::bind(addr).unwrap();
        listener.set_nonblocking(true).unwrap();
        while deathswitch.try_recv().is_err() {
            match listener.accept() {
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }
                Ok((stream, socket)) => {
                    debug!("new client connected: {socket}");
                    let event_sender = event_sender.clone();
                    stream.set_nonblocking(false).unwrap();
                    let mut transport = ServersideTransport::new(stream);
                    let _ = transport.0.send(&SocketMessage(socket));
                    let Ok(SocketMessage(my_socket_addr)) = transport.0.recv() else {
                        error!("Expected socket response from client");
                        continue;
                    };
                    {
                        let transport = transport.try_clone().unwrap();
                        event_sender
                            .send(
                                ((socket.ip(), NetworkEvent::Connect {
                                    transport,
                                    my_socket_addr,
                                }))
                                    .into(),
                            )
                            .unwrap();
                    }
                    thread::spawn(move || {
                        Self::connection_thread::<M>(event_sender, transport, socket)
                    });
                }
                Err(e) => {
                    error!("Message server listener error: {e}");
                    panic!();
                }
            }
        }
        debug!("message server shutting down");
    }

    fn connection_thread<M>(
        event_sender: Sender<impl From<(IpAddr, ServerNetworkEvent<M>)> + Send + 'static>,
        mut transport: ServersideTransport,
        socket: SocketAddr,
    ) where
        M: ClientServerMessage + DeserializeOwned,
        M::ClientMessage: TryFrom<M>,
    {
        let src_addr = socket.ip();
        let send_event = |event: ServerNetworkEvent<M>| event_sender.send((src_addr, event).into());
        let Err(err): Result<(), io::Error> = (try {
            loop {
                (send_event)(NetworkEvent::Message(transport.recv::<M>()?))
                    .map_err(|_| io::Error::new(ErrorKind::ConnectionAborted, "Channel closed"))?;
            }
        }) else {
            return;
        };
        debug!("connection ended with {socket}: {err}");
        let _ = (send_event)(NetworkEvent::Disconnect);
    }
}

impl Drop for MessageServer {
    fn drop(&mut self) {
        self.thread_kill.send(()).unwrap();
        self.listener_thread.take().unwrap().join().unwrap();
    }
}

pub struct MessageClient {
    connection_thread: Option<JoinHandle<()>>,
    thread_kill: Sender<()>,
}

impl MessageClient {
    pub fn start<M>(
        event_sender: Sender<impl From<ClientNetworkEvent<M>> + Send + 'static>,
        socket: SocketAddr,
    ) -> Self
    where
        M: ClientServerMessage + DeserializeOwned,
        M::ServerMessage: TryFrom<M>,
    {
        let (thread_kill, deathswitch) = channel();
        let connection_thread = Some(thread::spawn(move || {
            Self::connection_thread::<M>(event_sender, socket, deathswitch)
        }));
        MessageClient {
            connection_thread,
            thread_kill,
        }
    }

    fn connection_thread<M>(
        event_sender: Sender<impl From<ClientNetworkEvent<M>> + Send + 'static>,
        socket: SocketAddr,
        deathswitch: Receiver<()>,
    ) where
        M: ClientServerMessage + DeserializeOwned,
        M::ServerMessage: TryFrom<M>,
    {
        while deathswitch.try_recv().is_err() {
            let _: Result<_, io::Error> = try {
                let stream = TcpStream::connect(socket)?;
                debug!("connected to {socket}");
                let Err(err): Result<_, io::Error> = try {
                    let mut transport = ClientsideTransport::new(stream);
                    {
                        let mut transport = transport.try_clone().unwrap();
                        let Ok(SocketMessage(my_socket_addr)) = transport.0.recv() else {
                            error!("Expected socket message from server");
                            continue;
                        };
                        let _ = transport.0.send(&SocketMessage(socket));
                        event_sender
                            .send(
                                NetworkEvent::Connect {
                                    transport,
                                    my_socket_addr,
                                }
                                .into(),
                            )
                            .unwrap();
                    }
                    loop {
                        event_sender
                            .send(NetworkEvent::Message(transport.recv::<M>()?).into())
                            .map_err(|_| {
                                io::Error::new(ErrorKind::ConnectionAborted, "Channel Closed")
                            })?;
                    }
                };
                debug!("connection ended with {socket}: {err}");
                let _ = event_sender.send(NetworkEvent::Disconnect.into());
            };
        }
    }
}

impl Drop for MessageClient {
    fn drop(&mut self) {
        self.thread_kill.send(()).unwrap();
        self.connection_thread.take().unwrap().join().unwrap();
    }
}
