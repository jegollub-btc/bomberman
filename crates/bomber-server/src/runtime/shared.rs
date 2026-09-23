use std::sync::{Arc, Mutex};

use bomber_application::ArenaSession;
use tokio::sync::broadcast;

use crate::udp::Registry;
use crate::web::dto::{lobby_dto, match_init_dto, state_dto, Outbound};

/// How many messages a slow watcher may fall behind before it starts missing
/// them. State is complete every tick, so a lagging client catches up on the
/// next one rather than desyncing -- which is why this can be modest.
const FEED_CAPACITY: usize = 256;

/// State shared between the tick loop, the UDP socket and the web handlers.
///
/// Two locks, never held at the same time. Everything below takes one, finishes
/// with it, and only then takes the other -- so there is no lock ordering to
/// get wrong later.
#[derive(Clone)]
pub struct Shared {
    pub session: Arc<Mutex<ArenaSession>>,
    pub registry: Arc<Mutex<Registry>>,
    feed: broadcast::Sender<Arc<str>>,
}

impl Shared {
    pub fn new(session: ArenaSession) -> Self {
        let (feed, _) = broadcast::channel(FEED_CAPACITY);
        Shared {
            session: Arc::new(Mutex::new(session)),
            registry: Arc::new(Mutex::new(Registry::default())),
            feed,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<str>> {
        self.feed.subscribe()
    }

    pub fn publish(&self, outbound: &Outbound) {
        // An error here only means nobody is watching.
        let _ = self.feed.send(Arc::from(outbound.to_json()));
    }

    pub fn publish_json(&self, json: Arc<str>) {
        let _ = self.feed.send(json);
    }

    /// The current lobby, as the UI sees it.
    pub fn lobby_message(&self) -> Outbound {
        let snapshot = self.session.lock().unwrap().snapshot();
        let registry = self.registry.lock().unwrap();
        Outbound::Lobby(lobby_dto(&snapshot, &registry))
    }

    pub fn notify_lobby_changed(&self) {
        let message = self.lobby_message();
        self.publish(&message);
    }

    /// Everything a freshly connected watcher needs to draw the current
    /// situation, including one already in progress.
    ///
    /// A spectator that connects mid-match must not be left staring at nothing
    /// until the next one starts, so the match frame is replayed on connect.
    pub fn welcome_messages(&self) -> Vec<Outbound> {
        let mut messages = vec![self.lobby_message()];

        let session = self.session.lock().unwrap();
        let snapshot = session.snapshot();
        if let Some(init) = match_init_dto(&session, &snapshot) {
            messages.push(Outbound::MatchInit(init));
        }
        if let Some(game) = session.game() {
            messages.push(Outbound::State(state_dto(game, &[])));
        }
        messages
    }
}
