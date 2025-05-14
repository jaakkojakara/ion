use core::panic;
use std::collections::HashMap;
use std::net::AddrParseError;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::sync::{Mutex, mpsc};
use std::{
    cmp::min,
    collections::VecDeque,
    fmt::Debug,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle, sleep},
    time::{Duration, Instant},
};

use bincode::{Decode, Encode, config::Configuration, error::DecodeError};

use crate::Map;
use crate::math::rand::Rng;

#[cfg(not(target_arch = "wasm32"))]
use crate::util::native_spin_sleep;

// ---------------------------------------------------------- //
// ----------------------- Constants ------------------------ //
// ---------------------------------------------------------- //

const PROTOCOL_ID: u32 = 1225163695;

const MAX_UDP_PAYLOAD: usize = 1128;
// Should fit into a single MTU in most modern networks
const MSG_FRAGMENT_SIZE: usize = 1024;
// 1 KiBiByte
const MSG_MAX_TOTAL_SIZE: usize = 250 * 1024 * 1024;
// 250 MiBiBytes
const MSG_BUFFER_SIZE: usize = 512; // Number of both incoming and outgoing messages that can be buffered before the backpressure kicks in

const MIN_ACK_TIMEOUT: Duration = Duration::from_millis(50);
const MAX_ACK_TIMEOUT: Duration = Duration::from_millis(1000);

const BINCODE_CONFIG: Configuration = bincode::config::standard();

// ---------------------------------------------------------- //
// ------------------------ Socket -------------------------- //
// ---------------------------------------------------------- //

pub struct UdpNetworkSocket<T>
where
    T: 'static + Debug + Send + Encode + Decode<()>,
{
    socket: Arc<UdpSocket>,
    socket_on: Arc<AtomicBool>,
    socket_addr: SocketAddr,
    socket_handle: Option<JoinHandle<()>>,
    msg_out_sender: SyncSender<(SocketAddr, T, Duration)>,
    msg_in_receiver: Mutex<Receiver<(SocketAddr, T)>>,
    address_latencies: Arc<RwLock<Map<IpAddr, AtomicU64>>>,
}

impl<T> UdpNetworkSocket<T>
where
    T: 'static + Debug + Send + Encode + Decode<()>,
{
    pub fn new(bind_addr: SocketAddr) -> Self {
        // log_info!("Starting up udp network socket");

        let (msg_in_sender, msg_in_receiver) = mpsc::sync_channel::<(SocketAddr, T)>(MSG_BUFFER_SIZE);
        let (msg_out_sender, msg_out_receiver) = mpsc::sync_channel::<(SocketAddr, T, Duration)>(MSG_BUFFER_SIZE);

        let address_latencies = Arc::new(RwLock::new(Map::default()));

        let socket = Arc::new(UdpSocket::bind(bind_addr).unwrap());
        let socket_on = Arc::new(AtomicBool::new(true));

        socket.set_nonblocking(true).unwrap();

        let socket_handle = Self::build_network_thread(
            socket.clone(),
            socket_on.clone(),
            msg_in_sender,
            msg_out_receiver,
            address_latencies.clone(),
        );

        Self {
            socket,
            socket_on,
            socket_addr: bind_addr,
            socket_handle: Some(socket_handle),
            msg_out_sender,
            msg_in_receiver: Mutex::new(msg_in_receiver),
            address_latencies,
        }
    }

    // ---------------------------------------------------------- //
    // ------------------- Public functions --------------------- //
    // ---------------------------------------------------------- //

    #[allow(dead_code)]
    pub fn send(&self, addr: SocketAddr, msg: T, timeout: Duration) {
        match self.msg_out_sender.send((addr, msg, timeout)) {
            Ok(_) => {}
            Err(_) => panic!("Network sender disconnected"),
        }
    }

    #[allow(dead_code)]
    pub fn send_broadcast(&self, msg: T) {
        let addr = SocketAddr::from(([255, 255, 255, 255], self.socket_addr.port()));
        self.send(addr, msg, Duration::ZERO);
    }

    #[allow(dead_code)]
    pub fn recv_blocking(&self) -> (SocketAddr, T) {
        let msg_in_receiver = self.msg_in_receiver.lock().unwrap();
        match msg_in_receiver.recv() {
            Ok((from_addr, message)) => (from_addr, message),
            Err(_) => panic!("Network receiver disconnected"),
        }
    }

    #[allow(dead_code)]
    pub fn try_recv(&self) -> Option<(SocketAddr, T)> {
        let msg_in_receiver = self.msg_in_receiver.lock().unwrap();
        match msg_in_receiver.try_recv() {
            Ok((from_addr, message)) => Some((from_addr, message)),
            Err(err) => match err {
                TryRecvError::Empty => None,
                TryRecvError::Disconnected => panic!("Network receiver disconnected"),
            },
        }
    }

    #[allow(dead_code)]
    pub fn try_recv_all(&self) -> Vec<(SocketAddr, T)> {
        let msg_in_receiver = self.msg_in_receiver.lock().unwrap();
        msg_in_receiver.try_iter().collect()
    }

    #[allow(dead_code)]
    pub fn try_recv_timeout(&self, timeout: Duration) -> Option<(SocketAddr, T)> {
        let msg_in_receiver = self.msg_in_receiver.lock().unwrap();
        match msg_in_receiver.recv_timeout(timeout) {
            Ok((from_addr, message)) => Some((from_addr, message)),
            Err(err) => match err {
                RecvTimeoutError::Timeout => None,
                RecvTimeoutError::Disconnected => panic!("Network receiver disconnected"),
            },
        }
    }

    #[allow(dead_code)]
    pub fn latency_of(&self, addr: SocketAddr) -> Option<Duration> {
        self.address_latencies
            .read()
            .unwrap()
            .get(&addr.ip())
            .map(|latency_ms| Duration::from_millis(latency_ms.load(Ordering::Relaxed)))
    }

    pub fn enable_broadcast(&self) {
        self.socket.set_broadcast(true).unwrap();
    }

    pub fn disable_broadcast(&self) {
        self.socket.set_broadcast(false).unwrap();
    }

    #[allow(dead_code)]
    pub fn join_multicast(&self, multicast_ip: IpAddr) -> io::Result<()> {
        match multicast_ip {
            IpAddr::V4(ip) => self.socket.join_multicast_v4(&ip, &Ipv4Addr::from([0, 0, 0, 0])),
            IpAddr::V6(ip) => self.socket.join_multicast_v6(&ip, 0),
        }
    }

    #[allow(dead_code)]
    pub fn leave_multicast(&self, multicast_ip: IpAddr) -> io::Result<()> {
        match multicast_ip {
            IpAddr::V4(ip) => self.socket.leave_multicast_v4(&ip, &Ipv4Addr::from([0, 0, 0, 0])),
            IpAddr::V6(ip) => self.socket.leave_multicast_v6(&ip, 0),
        }
    }

    #[allow(dead_code)]
    pub fn is_loopback(&self) -> bool {
        self.socket.local_addr().unwrap().ip().is_loopback()
    }

    pub fn local_ip_addr(&self) -> Option<IpAddr> {
        let is_loopback = match self.socket_addr {
            SocketAddr::V4(addr) => *addr.ip().octets().first().unwrap() == 127,
            SocketAddr::V6(_) => panic!("Loopback ipv6 not supported"),
        };

        if is_loopback {
            Some(self.socket_addr.ip())
        } else {
            #[cfg(target_os = "macos")]
            return Self::local_ip_mac();

            #[cfg(target_os = "windows")]
            return Self::local_ip_windows();

            #[cfg(target_os = "linux")]
            return None;

            #[cfg(target_arch = "wasm32")]
            return None;
        }
    }

    // ---------------------------------------------------------- //
    // ---------------- Private implementation ------------------ //
    // ---------------------------------------------------------- //

    fn build_network_thread(
        socket: Arc<UdpSocket>,
        socket_on: Arc<AtomicBool>,
        msg_in_sender: SyncSender<(SocketAddr, T)>,
        msg_out_receiver: Receiver<(SocketAddr, T, Duration)>,
        address_latencies: Arc<RwLock<Map<IpAddr, AtomicU64>>>,
    ) -> JoinHandle<()> {
        thread::Builder::new()
            .name("udp_network_socket".to_owned())
            .spawn({
                // Default HashMap is used instead of faster 'Map' from this crate.
                // This is done to protect against hash-based ddos attacks, as u64 message ids are untrusted inputs

                let mut inc_data_buf = [0; MAX_UDP_PAYLOAD];
                let mut inc_fragment_buf = HashMap::default();

                let mut waiting_acks: HashMap<u64, SingleFrameAckDetails> = HashMap::default();
                let mut waiting_multiframe_acks: HashMap<u64, MultiFrameAckDetails> = HashMap::default();

                let mut send_queue = VecDeque::new();
                let mut send_multiframe_queue = VecDeque::new();

                let mut rng = Rng::new(None);

                move || {
                    while socket_on.load(Ordering::Relaxed) {
                        // Send frames
                        Self::execute_frame_sends(&socket, &mut send_queue, &mut send_multiframe_queue);

                        // Receive frames
                        Self::execute_frame_receives(
                            &socket,
                            &mut inc_data_buf,
                            &mut inc_fragment_buf,
                            &mut waiting_acks,
                            &mut waiting_multiframe_acks,
                            &mut send_queue,
                            &msg_in_sender,
                            address_latencies.clone(),
                        );

                        // Take in messages
                        Self::process_msg_sends(
                            &mut rng,
                            &msg_out_receiver,
                            &mut waiting_acks,
                            &mut waiting_multiframe_acks,
                            &mut send_queue,
                            &mut send_multiframe_queue,
                            address_latencies.clone(),
                        );

                        // Check waiting acks
                        Self::process_msg_resends(
                            &mut waiting_acks,
                            &mut waiting_multiframe_acks,
                            &mut send_queue,
                            &mut send_multiframe_queue,
                            address_latencies.clone(),
                        );

                        // Clean up old broken transactions from inc_fragment_buf
                        let now = Instant::now();
                        inc_fragment_buf.retain(|_, (timestamp, _, _)| *timestamp + Duration::from_secs(60) > now);

                        // Don't hot loop on non-windows platforms
                        // On windows we need to hot loop to keep the latency small
                        #[cfg(not(target_os = "windows"))]
                        sleep(Duration::from_micros(1));
                    }
                }
            })
            .unwrap()
    }

    fn parse_frame(data: &[u8]) -> Option<(u64, FrameBody)> {
        match NetworkFrame::try_from(data) {
            Ok(network_frame) => {
                if network_frame.protocol_id == PROTOCOL_ID {
                    Some((network_frame.frame_id, network_frame.frame_body))
                } else {
                    // log_warn!("Udp network frame received with invalid protocol id");
                    None
                }
            }
            Err(_) => {
                // log_warn!("Udp network frame decoding failed");
                None
            }
        }
    }

    fn parse_user_msg(data: &[u8]) -> Option<T> {
        let parse_result = bincode::decode_from_slice::<T, _>(data, BINCODE_CONFIG);

        match parse_result {
            Ok((message, size_used)) => {
                if size_used == data.len() {
                    Some(message)
                } else {
                    // log_warn!(
                    //     "Network user message decoding did not use all of the data. dat_len: {}, size_used: {} ",
                    //     data.len(),
                    //     size_used
                    // );
                    None
                }
            }
            _ => {
                // log_warn!("Network user message decoding failed");
                None
            }
        }
    }

    fn execute_frame_sends(
        socket: &UdpSocket,
        send_queue: &mut VecDeque<(SocketAddr, NetworkFrame)>,
        send_multiframe_queue: &mut VecDeque<(SocketAddr, NetworkFrame)>,
    ) {
        let mut singleframe_to_send = send_queue.pop_front();
        let mut multiframe_to_send = send_multiframe_queue.pop_front();

        while singleframe_to_send.is_some() || multiframe_to_send.is_some() {
            if let Some((addr, frame)) = singleframe_to_send.take() {
                // log_trc!(
                //     "Sending singleframe with id {:?} to {:?}",
                //     frame.frame_id,
                //     addr
                // );
                let frame_byte_vec: Vec<u8> = frame.clone().into();
                match socket.send_to(&frame_byte_vec, addr) {
                    Ok(_) => {}
                    Err(err) => match err.kind() {
                        io::ErrorKind::WouldBlock => {
                            send_queue.push_front((addr, frame));
                            break;
                        }
                        _ => panic!("Error sending udp frame to {:?}: {:?}", addr, err),
                    },
                }
            }

            if let Some((addr, frame)) = multiframe_to_send.take() {
                // log_trc!(
                //     "Sending multiframe with id {:?} to {:?}",
                //     frame.frame_id,
                //     addr
                // );
                let frame_byte_vec: Vec<u8> = frame.clone().into();
                match socket.send_to(&frame_byte_vec, addr) {
                    Ok(_) => {}
                    Err(err) => match err.kind() {
                        io::ErrorKind::WouldBlock => {
                            send_multiframe_queue.push_front((addr, frame));
                            break;
                        }
                        _ => panic!("Error sending udp frame: {:?}", err),
                    },
                }
            }

            singleframe_to_send = send_queue.pop_front();
            multiframe_to_send = send_multiframe_queue.pop_front();

            // Sleep to not flood the socket
            #[cfg(not(target_arch = "wasm32"))]
            native_spin_sleep(Duration::from_nanos(100));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_frame_receives(
        socket: &UdpSocket,
        inc_data_buf: &mut [u8; MAX_UDP_PAYLOAD],
        inc_fragment_buf: &mut HashMap<u64, (Instant, Vec<bool>, Vec<u8>)>,
        waiting_acks: &mut HashMap<u64, SingleFrameAckDetails>,
        waiting_multiframe_acks: &mut HashMap<u64, MultiFrameAckDetails>,
        send_queue: &mut VecDeque<(SocketAddr, NetworkFrame)>,
        msg_in_sender: &SyncSender<(SocketAddr, T)>,
        address_latencies: Arc<RwLock<Map<IpAddr, AtomicU64>>>,
    ) {
        while let Ok((recv_size, from_addr)) = socket.recv_from(inc_data_buf) {
            if let Some((id, frame_body)) = Self::parse_frame(&inc_data_buf[0..recv_size]) {
                match frame_body {
                    FrameBody::SingleFrameMessage { data } => {
                        // log_trc!("Received SingleFrameMessage from {:?}", from_addr);
                        if let Some(msg) = Self::parse_user_msg(&data) {
                            // log_dbg!("Received msg {:?} from {:?}", &msg, from_addr);
                            let ack_frame = NetworkFrame::new(id, FrameBody::SingleFrameMessageAck);
                            msg_in_sender.send((from_addr, msg)).unwrap();
                            send_queue.push_back((from_addr, ack_frame));
                        }
                    }
                    FrameBody::SingleFrameMessageAck => {
                        // log_trc!("Received SingleFrameMessageAck from {:?}", from_addr);
                        if let Some(ack_details) = waiting_acks.remove(&id) {
                            let new_latency = (Instant::now() - ack_details.sent_at_original) / 2; // Latency is half or RTT
                            Self::update_latency_estimate(ack_details.addr, new_latency, address_latencies.clone());
                        }
                    }
                    FrameBody::MultiFrameMessageBegin {
                        total_fragments,
                        total_size,
                    } => {
                        // log_trc!("Received MultiFrameMessageBegin from {:?}", from_addr);
                        if total_size < MSG_MAX_TOTAL_SIZE
                            && total_fragments > 1
                            && total_size <= total_fragments * MSG_FRAGMENT_SIZE
                            && total_size > (total_fragments - 1) * MSG_FRAGMENT_SIZE
                        {
                            let fragment_vec = vec![false; total_fragments];
                            let data_vec = vec![0_u8; total_size];
                            inc_fragment_buf.insert(id, (Instant::now(), fragment_vec, data_vec));
                        } else {
                            // log_warn!(
                            //     "Received UDP MultiFrameMessageBegin with total size larger than max size. total_size: {}",
                            //     total_size
                            // );
                        }
                    }
                    FrameBody::MultiFrameMessageFragment { fragment_id, data } => {
                        // log_trc!(
                        //     "Received MultiFrameMessageFragment {:?} from {:?}",
                        //     fragment_id,
                        //     from_addr
                        // );
                        inc_fragment_buf
                            .entry(id)
                            .and_modify(|(timestamp, fragment_vec, data_vec)| {
                                *timestamp = Instant::now();
                                fragment_vec[fragment_id] = true;
                                let fragment_start_i = fragment_id * MSG_FRAGMENT_SIZE;
                                let fragment_end_i = fragment_id * MSG_FRAGMENT_SIZE + data.len();

                                data_vec[fragment_start_i..fragment_end_i].copy_from_slice(&data);
                            });
                    }
                    FrameBody::MultiFrameMessageEnd => {
                        // log_trc!("Received MultiFrameMessageEnd from {:?}", from_addr);
                        if let Some((timestamp, fragment_vec, data_vec)) = inc_fragment_buf.remove(&id) {
                            let missing_fragments: Vec<usize> = fragment_vec
                                .iter()
                                .enumerate()
                                .filter(|(_, was_received)| !**was_received)
                                .map(|(i, _)| i)
                                .collect();

                            if missing_fragments.is_empty() {
                                let ack_frame = NetworkFrame::new(id, FrameBody::MultiFrameMessageAck);
                                send_queue.push_back((from_addr, ack_frame));

                                if let Some(msg) = Self::parse_user_msg(&data_vec) {
                                    // log_dbg!("Received msg {:?} from {:?}", &msg, from_addr);
                                    msg_in_sender.send((from_addr, msg)).unwrap();
                                }
                            } else {
                                inc_fragment_buf.insert(id, (timestamp, fragment_vec, data_vec));
                                let missing_fragments: Vec<_> = missing_fragments[0..min(missing_fragments.len(), 200)]
                                    .iter()
                                    .copied()
                                    .map(|i| i as u32)
                                    .collect();
                                let ack_frame =
                                    NetworkFrame::new(id, FrameBody::MultiFrameMessageAckFail { missing_fragments });

                                send_queue.push_back((from_addr, ack_frame));
                            }
                        }
                    }
                    FrameBody::MultiFrameMessageAck => {
                        // log_trc!("Received MultiFrameMessageAck from {:?}", from_addr);
                        waiting_multiframe_acks.remove(&id);
                    }
                    FrameBody::MultiFrameMessageAckFail { missing_fragments } => {
                        // log_trc!("Received MultiFrameMessageAckFail from {:?}", from_addr);
                        waiting_multiframe_acks.entry(id).and_modify(|ack_details| {
                            ack_details.missing_frames = missing_fragments;
                        });
                    }
                }
            }
        }
    }

    fn process_msg_resends(
        waiting_acks: &mut HashMap<u64, SingleFrameAckDetails>,
        waiting_multiframe_acks: &mut HashMap<u64, MultiFrameAckDetails>,
        send_queue: &mut VecDeque<(SocketAddr, NetworkFrame)>,
        send_multiframe_queue: &mut VecDeque<(SocketAddr, NetworkFrame)>,
        address_latencies: Arc<RwLock<Map<IpAddr, AtomicU64>>>,
    ) {
        let now = Instant::now();
        waiting_acks.retain(|_msg_id, ack_details| {
            let retain = now < ack_details.timeout_at;
            if !retain {
                // log_warn!(
                //     "Failed to send singleframe message {} to {:?}",
                //     msg_id,
                //     ack_details.addr
                // );
            }
            retain
        });
        waiting_multiframe_acks.retain(|_, ack_details| {
            let retain = now < ack_details.timeout_at;
            if !retain {
                // log_warn!(
                //     "Failed to send multiframe message {} to {:?}",
                //     ack_details.msg_id,
                //     ack_details.addr
                // );
            }
            retain
        });

        waiting_acks.iter_mut().for_each(|(_msg_id, ack_details)| {
            if now > ack_details.next_resend_at {
                // log_dbg!("Resending msg {} to {}", _msg_id, ack_details.addr);
                let latency = address_latencies
                    .read()
                    .unwrap()
                    .get(&ack_details.addr.ip())
                    .map(|latency_ms| Duration::from_millis(latency_ms.load(Ordering::Relaxed)))
                    .unwrap_or(Duration::from_millis(100));
                let next_send = now
                    + (5 * ack_details.sent_count * latency)
                        .max(MIN_ACK_TIMEOUT)
                        .min(MAX_ACK_TIMEOUT);
                ack_details.sent_count += 1;
                ack_details.next_resend_at = next_send;
                send_queue.push_back((ack_details.addr, ack_details.frame.clone()));
            }
        });

        waiting_multiframe_acks.iter_mut().for_each(|(_, ack_details)| {
            if !ack_details.missing_frames.is_empty() {
                // log_dbg!(
                //     "Resending missing frames {} to {}",
                //     ack_details.msg_id,
                //     ack_details.addr
                // );
                ack_details.missing_frames.iter().for_each(|frame_i| {
                    send_multiframe_queue
                        .push_back((ack_details.addr, ack_details.all_frames[*frame_i as usize].clone()));
                });
                send_multiframe_queue.push_back((
                    ack_details.addr,
                    NetworkFrame::new(ack_details.msg_id, FrameBody::MultiFrameMessageEnd),
                ));
                ack_details.missing_frames.clear();
            }
        });
    }

    fn process_msg_sends(
        rng: &mut Rng,
        msg_out_receiver: &Receiver<(SocketAddr, T, Duration)>,
        waiting_acks: &mut HashMap<u64, SingleFrameAckDetails>,
        waiting_multiframe_acks: &mut HashMap<u64, MultiFrameAckDetails>,
        send_queue: &mut VecDeque<(SocketAddr, NetworkFrame)>,
        send_multiframe_queue: &mut VecDeque<(SocketAddr, NetworkFrame)>,
        address_latencies: Arc<RwLock<Map<IpAddr, AtomicU64>>>,
    ) {
        while let Ok((addr, msg, timeout)) = msg_out_receiver.try_recv() {
            let id = rng.gen_u64();
            // log_dbg!("Sending msg {:?} to {:?} with id {}", &msg, addr, id);

            let data = bincode::encode_to_vec(msg, BINCODE_CONFIG).unwrap();
            let data_len = data.len();

            let is_unicast = match addr.ip() {
                IpAddr::V4(addr) => !addr.is_multicast() && !addr.is_broadcast(),
                IpAddr::V6(addr) => !addr.is_multicast(),
            };

            if data_len < MSG_FRAGMENT_SIZE {
                let frame = NetworkFrame::new(id, FrameBody::SingleFrameMessage { data });

                if is_unicast {
                    let now = Instant::now();
                    let latency = address_latencies
                        .read()
                        .unwrap()
                        .get(&addr.ip())
                        .map(|latency_ms| Duration::from_millis(latency_ms.load(Ordering::Relaxed)))
                        .unwrap_or(Duration::from_millis(100));

                    waiting_acks.insert(
                        id,
                        SingleFrameAckDetails {
                            sent_at_original: now,
                            sent_count: 1,
                            timeout_at: now + timeout,
                            next_resend_at: now + (5 * latency).max(MIN_ACK_TIMEOUT).min(MAX_ACK_TIMEOUT),
                            addr,
                            frame: frame.clone(),
                        },
                    );
                }
                send_queue.push_back((addr, frame));
            } else if data_len < MSG_MAX_TOTAL_SIZE && is_unicast {
                let now = Instant::now();
                let fragment_frames: Vec<_> = data
                    .chunks(MSG_FRAGMENT_SIZE)
                    .enumerate()
                    .map(|(i, data)| {
                        NetworkFrame::new(
                            id,
                            FrameBody::MultiFrameMessageFragment {
                                fragment_id: i,
                                data: data.to_vec(),
                            },
                        )
                    })
                    .collect();

                let start_frame = NetworkFrame::new(
                    id,
                    FrameBody::MultiFrameMessageBegin {
                        total_fragments: fragment_frames.len(),
                        total_size: data_len,
                    },
                );
                let end_frame = NetworkFrame::new(id, FrameBody::MultiFrameMessageEnd);

                waiting_multiframe_acks.insert(
                    id,
                    MultiFrameAckDetails {
                        msg_id: id,
                        timeout_at: now + timeout,
                        addr,
                        missing_frames: Vec::new(),
                        all_frames: fragment_frames.clone(),
                    },
                );

                send_multiframe_queue.push_back((addr, start_frame));
                fragment_frames.into_iter().for_each(|frame| {
                    send_multiframe_queue.push_back((addr, frame));
                });
                send_multiframe_queue.push_back((addr, end_frame));
            } else {
                // log_error!("Message to {:?} too large: {}", addr, data.len());
            }
        }
    }

    fn update_latency_estimate(
        addr: SocketAddr,
        new_latency: Duration,
        address_latencies: Arc<RwLock<Map<IpAddr, AtomicU64>>>,
    ) {
        if !address_latencies.read().unwrap().contains_key(&addr.ip()) {
            address_latencies
                .write()
                .unwrap()
                .insert(addr.ip(), AtomicU64::new(100));
        }

        address_latencies
            .read()
            .unwrap()
            .get(&addr.ip())
            .unwrap()
            .fetch_update(Ordering::Release, Ordering::Acquire, |prev_latency| {
                Some((prev_latency * 9 + new_latency.as_millis() as u64) / 10)
            })
            .unwrap();
    }

    #[allow(dead_code)]
    fn local_ip_mac() -> Option<IpAddr> {
        fn try_interface(interface: &str) -> Result<IpAddr, AddrParseError> {
            let mut command = Command::new("ipconfig");
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
            command.arg("getifaddr");
            command.arg(interface);
            command.status().unwrap();
            let output =
                String::from_utf8(command.output().unwrap().stdout).expect("ipconfig output should be valid utf-8");
            IpAddr::from_str(output.as_str().trim())
        }

        for interface_num in 0..10 {
            let interface_name = format!("en{}", interface_num);
            if let Ok(addr) = try_interface(&interface_name) {
                return Some(addr);
            }
        }

        None
    }

    #[allow(dead_code)]
    fn local_ip_windows() -> Option<IpAddr> {
        Some(IpAddr::from_str("192.168.1.209").unwrap())
    }
}

impl<T> Drop for UdpNetworkSocket<T>
where
    T: 'static + Debug + Send + Encode + Decode<()>,
{
    fn drop(&mut self) {
        // log_info!("Shutting down udp network socket");

        // Wait for socket to send last frames before shutting down
        sleep(Duration::from_millis(2));

        self.socket_on.store(false, Ordering::Relaxed);
        self.socket_handle.take().unwrap().join().ok();
    }
}

// ---------------------------------------------------------- //
// ---------------- Supporting data types ------------------- //
// ---------------------------------------------------------- //

#[derive(Clone, Encode, Decode)]
enum FrameBody {
    SingleFrameMessage { data: Vec<u8> },
    SingleFrameMessageAck,
    MultiFrameMessageBegin { total_fragments: usize, total_size: usize },
    MultiFrameMessageFragment { fragment_id: usize, data: Vec<u8> },
    MultiFrameMessageEnd,
    MultiFrameMessageAck,
    MultiFrameMessageAckFail { missing_fragments: Vec<u32> },
}

impl Debug for FrameBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleFrameMessage { .. } => write!(f, "SingleFrameMessage"),
            Self::SingleFrameMessageAck => write!(f, "SingleFrameMessageAck"),
            Self::MultiFrameMessageBegin { .. } => write!(f, "MultiFrameMessageBegin"),
            Self::MultiFrameMessageFragment { .. } => write!(f, "MultiFrameMessageFragment"),
            Self::MultiFrameMessageEnd => write!(f, "MultiFrameMessageEnd"),
            Self::MultiFrameMessageAck => write!(f, "MultiFrameMessageAck"),
            Self::MultiFrameMessageAckFail { .. } => write!(f, "MultiFrameMessageAckFail"),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
struct NetworkFrame {
    protocol_id: u32,
    frame_id: u64,
    frame_body: FrameBody,
}

impl NetworkFrame {
    fn new(frame_id: u64, frame_body: FrameBody) -> Self {
        Self {
            protocol_id: PROTOCOL_ID,
            frame_id,
            frame_body,
        }
    }
}

impl From<NetworkFrame> for Vec<u8> {
    fn from(frame: NetworkFrame) -> Self {
        let byte_vec = bincode::encode_to_vec(frame.clone(), BINCODE_CONFIG).unwrap();
        assert!(byte_vec.len() <= MAX_UDP_PAYLOAD);
        byte_vec
    }
}

impl TryFrom<&[u8]> for NetworkFrame {
    type Error = DecodeError;
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        bincode::decode_from_slice::<NetworkFrame, Configuration>(data, BINCODE_CONFIG).and_then(
            |(frame, size_used)| {
                if size_used == data.len() {
                    Ok(frame)
                } else {
                    Err(DecodeError::OtherString(
                        "Encoding did not use complete data slice".to_owned(),
                    ))
                }
            },
        )
    }
}

struct SingleFrameAckDetails {
    sent_at_original: Instant,
    sent_count: u32,
    next_resend_at: Instant,
    timeout_at: Instant,
    addr: SocketAddr,
    frame: NetworkFrame,
}

struct MultiFrameAckDetails {
    msg_id: u64,
    timeout_at: Instant,
    addr: SocketAddr,
    missing_frames: Vec<u32>,
    all_frames: Vec<NetworkFrame>,
}

// ---------------------------------------------------------- //
// ------------------------ Tests --------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use std::{
        net::{SocketAddr, UdpSocket},
        panic,
        sync::{
            Arc,
            atomic::{AtomicU32, Ordering},
        },
        thread::{self, sleep},
        time::{Duration, Instant},
    };

    use bincode::{Decode, Encode};

    use crate::math::rand::Rng;
    use crate::net::udp_network_socket::{MAX_UDP_PAYLOAD, UdpNetworkSocket};

    fn catch_unwind_silent<F: FnOnce() -> R + panic::UnwindSafe, R>(f: F) -> thread::Result<R> {
        let prev_hook = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let result = panic::catch_unwind(f);
        panic::set_hook(prev_hook);
        result
    }

    #[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
    enum SimpleMessage {
        NoData,
        SomeData(u32),
        LotsOfBytes(Vec<u8>),
    }

    #[test]
    fn sending_valid_singleframe_messages_succeeds() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3001));
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 3002));

        let socket1: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr1);
        let socket2: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr2);

        for i in 0..64 {
            socket1.send(addr2, SimpleMessage::SomeData(i), Duration::from_secs(1));
        }

        let start_recv = Instant::now();

        let mut resp_count = 0;

        while resp_count != 64 && start_recv + Duration::from_secs(5) > Instant::now() {
            let resp = socket2.try_recv();
            if resp.is_some() {
                resp_count += 1;
            }
        }

        assert_eq!(resp_count, 64);
    }

    #[test]
    fn sending_valid_multiframe_messages_succeeds() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3003));
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 3004));

        let socket1: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr1);
        let socket2: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr2);

        let mut msg_vec: Vec<u8> = vec![0; 21964];
        let mut rng = Rng::new(None);
        rng.fill_random_bytes(msg_vec.as_mut_slice());

        let msg = SimpleMessage::LotsOfBytes(msg_vec);

        socket1.send(addr2, msg.clone(), Duration::from_secs(5));

        let resp = socket2.try_recv_timeout(Duration::from_secs(5));

        assert!(resp.is_some());
        assert_eq!(resp.unwrap().1, msg);
    }

    #[test]
    fn sending_large_message_should_not_block_small_messages_while_sending() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3012));
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 3013));

        let socket1: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr1);
        let socket2: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr2);

        let mut msg_vec: Vec<u8> = vec![0; 51964];
        let mut rng = Rng::new(None);
        rng.fill_random_bytes(msg_vec.as_mut_slice());

        let msg1 = SimpleMessage::LotsOfBytes(msg_vec);
        let msg2 = SimpleMessage::SomeData(672);

        socket1.send(addr2, msg1.clone(), Duration::from_secs(5));
        socket1.send(addr2, msg2.clone(), Duration::from_secs(5));
        socket1.send(addr2, msg2.clone(), Duration::from_secs(5));

        let resp1 = socket2.try_recv_timeout(Duration::from_secs(5));
        let resp2 = socket2.try_recv_timeout(Duration::from_secs(5));
        let resp3 = socket2.try_recv_timeout(Duration::from_secs(5));

        assert!(resp1.is_some());
        assert!(resp2.is_some());
        assert!(resp3.is_some());
        assert_eq!(resp1.unwrap().1, msg2);
        assert_eq!(resp2.unwrap().1, msg2);
        assert_eq!(resp3.unwrap().1, msg1);
    }

    #[test]
    fn receiving_random_data_is_silently_ignored() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3005));
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 3006));

        let socket1: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr1);
        let custom_socket = UdpSocket::bind(addr2).unwrap();

        let mut msg_vec: Vec<u8> = vec![0; 423];
        let mut rng = Rng::new(None);
        rng.fill_random_bytes(msg_vec.as_mut_slice());
        custom_socket.send_to(&msg_vec, addr1).unwrap();

        let resp = socket1.try_recv_timeout(Duration::from_secs(1));
        assert!(resp.is_none());
    }

    #[test]
    fn not_getting_ack_triggers_message_resend_and_timeout() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3007));
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 3008));

        let received_resps = Arc::new(AtomicU32::new(0));
        let received_resp_clones = received_resps.clone();

        let custom_socket = UdpSocket::bind(addr1).unwrap();
        thread::spawn(move || {
            for _ in 0..3 {
                let mut inc_data_buf = [0; MAX_UDP_PAYLOAD];
                let recv = custom_socket.recv_from(&mut inc_data_buf);
                if let Ok((_, addr)) = recv {
                    assert_eq!(addr, addr2);
                    received_resp_clones.fetch_add(1, Ordering::SeqCst);
                } else {
                    panic!("Failed recv message");
                }
            }
        });

        let socket2: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr2);

        socket2.send(addr1, SimpleMessage::SomeData(23), Duration::from_secs(2));

        sleep(Duration::from_secs(3));

        assert_eq!(received_resps.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn latency_estimate_is_updated_correctly() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3009));
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 3010));

        let socket1: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr1);
        let socket2: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr2);

        for i in 0..256 {
            socket1.send(addr2, SimpleMessage::SomeData(i), Duration::from_secs(1));
        }

        let start_recv = Instant::now();

        let mut resp_count = 0;

        while resp_count != 256 && start_recv + Duration::from_secs(5) > Instant::now() {
            let resp = socket2.try_recv();
            if resp.is_some() {
                resp_count += 1;
            }
        }

        while socket1.latency_of(addr2).is_none() {
            sleep(Duration::from_millis(1));
        }

        assert_eq!(resp_count, 256);
        assert!(socket1.latency_of(addr2).unwrap() < Duration::from_millis(10));
    }

    #[test]
    fn dropping_udp_network_socket_frees_the_bound_addr() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3011));
        let socket: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr);

        assert!(
            catch_unwind_silent(|| {
                let _socket: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr);
            })
            .is_err()
        );

        drop(socket);

        let _socket: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr);
    }

    #[test]
    fn getting_local_ip_address_works() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3103));
        let socket1: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr1);
        let ip = socket1.local_ip_addr();
        println!("{:?}", ip);
    }

    #[test]
    fn sending_valid_huge_multiframe_messages_succeeds() {
        let addr1 = SocketAddr::from(([127, 0, 0, 1], 3101));
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 3102));

        let socket1: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr1);
        let socket2: UdpNetworkSocket<SimpleMessage> = UdpNetworkSocket::new(addr2);

        let mut msg_vec: Vec<u8> = vec![0; 249_964_003];
        let mut rng = Rng::new(None);
        rng.fill_random_bytes(msg_vec.as_mut_slice());

        let msg = SimpleMessage::LotsOfBytes(msg_vec);

        socket1.send(addr2, msg.clone(), Duration::from_secs(15));

        let resp = socket2.try_recv_timeout(Duration::from_secs(15));

        assert!(resp.is_some());
        assert_eq!(resp.unwrap().1, msg);
    }
}
