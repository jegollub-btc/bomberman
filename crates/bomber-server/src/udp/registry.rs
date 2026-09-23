use std::collections::HashMap;
use std::net::SocketAddr;

use bomber_domain::shared::PlayerId;

/// Which address owns which seat.
///
/// This is the whole of authentication. A two-byte uplink has no room for a
/// token, so a player id is only ever believed when it arrives from the address
/// that claimed it at the door. Without this check any bot could drive any
/// other bot's character by putting a different number in byte 0.
#[derive(Debug, Default)]
pub struct Registry {
    by_addr: HashMap<SocketAddr, PlayerId>,
    by_player: HashMap<PlayerId, SocketAddr>,
}

impl Registry {
    pub fn bind(&mut self, addr: SocketAddr, player: PlayerId) {
        // A seat can only have one address, and an address only one seat.
        if let Some(previous) = self.by_player.insert(player, addr) {
            self.by_addr.remove(&previous);
        }
        self.by_addr.insert(addr, player);
    }

    pub fn release(&mut self, player: PlayerId) {
        if let Some(addr) = self.by_player.remove(&player) {
            self.by_addr.remove(&addr);
        }
    }

    pub fn player_of(&self, addr: &SocketAddr) -> Option<PlayerId> {
        self.by_addr.get(addr).copied()
    }

    pub fn addr_of(&self, player: PlayerId) -> Option<SocketAddr> {
        self.by_player.get(&player).copied()
    }

    /// Every bound seat, for broadcasting.
    pub fn endpoints(&self) -> Vec<(PlayerId, SocketAddr)> {
        self.by_player.iter().map(|(p, a)| (*p, *a)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(port: u16) -> SocketAddr {
        format!("127.0.0.1:{port}").parse().unwrap()
    }

    #[test]
    fn binding_is_two_way_and_releasing_clears_both_sides() {
        let mut registry = Registry::default();
        let player = PlayerId::new(1);
        registry.bind(addr(1000), player);

        assert_eq!(registry.player_of(&addr(1000)), Some(player));
        assert_eq!(registry.addr_of(player), Some(addr(1000)));

        registry.release(player);
        assert_eq!(registry.player_of(&addr(1000)), None);
        assert_eq!(registry.addr_of(player), None);
    }

    /// A bot that reconnects from a new source port must not leave its old
    /// address able to drive the seat.
    #[test]
    fn rebinding_a_seat_forgets_the_old_address() {
        let mut registry = Registry::default();
        let player = PlayerId::new(0);
        registry.bind(addr(1000), player);
        registry.bind(addr(2000), player);

        assert_eq!(registry.player_of(&addr(1000)), None);
        assert_eq!(registry.player_of(&addr(2000)), Some(player));
        assert_eq!(registry.endpoints().len(), 1);
    }
}
