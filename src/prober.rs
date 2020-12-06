extern crate pnet;
use pnet::packet::{ip::IpNextHeaderProtocols, ipv4::*, udp::*, Packet};
use std::net::Ipv4Addr;

#[derive(Default)]
pub struct ProbeDebugResult {}

pub struct ProbeResult {
    destination: Ipv4Addr,
    responder: Ipv4Addr,
    distance: u8,
    from_destination: bool,
    debug: ProbeDebugResult,
}

#[derive(Copy, Clone)]
pub enum ProbePhase {
    Pre = 0,
    Main = 1,
}

pub type ProbeCallback = fn(result: ProbeResult);

pub struct Prober {
    callback: ProbeCallback,
    phase: ProbePhase,
    dst_port: u16,
    payload_msg: String,
    encode_timestamp: bool,
}

impl Prober {
    const IPV4_HEADER_LENGTH: u16 = 20;

    pub fn new(
        callback: ProbeCallback,
        phase: ProbePhase,
        dst_port: u16,
        payload_msg: String,
        encode_timestamp: bool,
    ) -> Self {
        Self {
            callback,
            phase,
            dst_port,
            payload_msg,
            encode_timestamp,
        }
    }
}

pub type ProbeUnit = (Ipv4Addr, u8);

impl Prober {
    pub fn pack(&self, destination: ProbeUnit, source_ip: Ipv4Addr) -> Option<Ipv4Packet> {
        let (dst_ip, ttl) = destination;
        let timestamp = crate::utils::get_timestamp_ms_u16();
        let expect_total_size = {
            let mut size = 128;
            if self.encode_timestamp {
                size |= ((timestamp >> 10) & 0x3F) << 1;
            }
            size
        };
        let expect_udp_size = expect_total_size - Self::IPV4_HEADER_LENGTH;

        let mut udp_packet = MutableUdpPacket::owned(vec![0u8; expect_udp_size as usize])?;
        udp_packet.set_source(8888); // TODO: what's the port?
        udp_packet.set_destination(self.dst_port);
        udp_packet.set_length(expect_udp_size);
        udp_packet.set_payload(self.payload_msg.as_bytes());

        let ip_id = {
            let mut id = (ttl as u16 & 0x1F) | ((self.phase as u16 & 0x1) << 5);
            if self.encode_timestamp {
                id |= (timestamp & 0x3FF) << 6;
            }
            id
        };

        let mut ip_packet = MutableIpv4Packet::owned(vec![0u8; expect_total_size as usize])?;
        ip_packet.set_version(4);
        ip_packet.set_header_length((Self::IPV4_HEADER_LENGTH >> 2) as u8);
        ip_packet.set_destination(dst_ip);
        ip_packet.set_source(source_ip);
        ip_packet.set_next_level_protocol(IpNextHeaderProtocols::Udp);
        ip_packet.set_ttl(ttl);
        ip_packet.set_identification(ip_id);
        ip_packet.set_total_length(expect_total_size);

        ip_packet.set_payload(udp_packet.packet());

        return Some(ip_packet.consume_to_immutable());
    }
}

mod test {
    use super::*;

    #[test]
    fn test_pack() {
        let prober = Prober::new(|_| {}, ProbePhase::Pre, 33434, "hello".to_owned(), true);
        let packet = prober
            .pack(("1.2.3.4".parse().unwrap(), 32), "4.3.2.1".parse().unwrap())
            .unwrap();
        println!("{:#?}", packet);
    }
}
