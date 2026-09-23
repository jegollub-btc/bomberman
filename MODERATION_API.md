# Moderation API

How the moderation UI talks to the server. One WebSocket, JSON both ways.

```
ws://127.0.0.1:8080/ws/admin
```

WebSocket runs over TCP, so nothing is lost or reordered here — unlike the
bots' UDP channel. Send a command, get exactly one reply.

---

## Commands (UI → server)

```json
{ "cmd": "start" }
```

| `cmd` | Payload | Effect |
|-------|---------|--------|
| `start` | — | Begin the countdown, then the match. Fails unless at least `min_players` seats are filled. |
| `pause` | — | Freeze the tick loop. Bots keep their sockets; the clock stops. |
| `resume` | — | Unfreeze. |
| `end` | — | End the running match now. Produces a normal `match_end` with `"reason": "aborted"`. |
| `reset` | — | Back to the lobby, ready for the next match. |
| `lock` / `unlock` | — | Stop / allow new bots joining. |
| `kick` | `{"id": 2}` | Free that seat. |
| `rename` | `{"id": 2, "name": "team-rocket"}` | Set a seat's display name. |

`end` and `reset` are deliberately separate: `end` stops play and leaves the
results on screen, `reset` clears them. Collapsing the two would make it
impossible to look at a finished match without dismissing it.

---

## Replies (server → UI)

Every command gets exactly one of these:

```json
{ "type": "ack",   "cmd": "start" }
{ "type": "error", "cmd": "start", "message": "need at least 2 players, have 1" }
```

A command that is not valid right now is an `error`, not a silent no-op — so
the UI can say why the button did nothing.

---

## State pushes (server → UI, unsolicited)

The admin socket also receives everything the spectator socket does
(`lobby`, `match_init`, `state`, `match_end` — see `VISUALIZER_GUIDE.md`).

The one you need for the controls is `lobby`:

```json
{
  "type": "lobby",
  "state": "open",
  "paused": false,
  "can_start": true,
  "min_players": 2,
  "max_players": 4,
  "countdown_ticks": 0,
  "tick_rate": 60,
  "map": { "width": 15, "height": 13, "density": 0.75, "symmetry": "quad", "seed": "0" },
  "slots": [
    { "id": 0, "name": "bot-0", "connected": true, "addr": "10.12.3.44:51234",
      "packets_per_sec": 60, "loss_pct": 0.4, "last_seen_tick": 1233,
      "stale_ms": 16, "stale": false },
    { "id": 1, "name": "bot-1", "connected": false, "addr": null,
      "packets_per_sec": 0, "loss_pct": 0, "last_seen_tick": null,
      "stale_ms": null, "stale": false }
  ]
}
```

Sent on connect, whenever anything changes, **and twice a second regardless**.
That last part matters: `packets_per_sec` and `stale_ms` are measurements, not
events, so a readout that only updated on change would freeze exactly when a
bot goes quiet.

`state` is `open` · `locked` · `countdown` · `running` · `match_over`.

`stale_ms` is **time since that seat last sent anything**, not round-trip time.
RTT needs an echo and a two-byte uplink has no room for a token to echo back;
staleness is measurable and is what actually answers "is it safe to start".
`stale` is the server applying its own threshold to it.

**Drive the buttons from `state` and `can_start`, not from your own bookkeeping.**
`can_start` is the server's own answer to "would `start` succeed right now", so
the UI and the server can never disagree about whether the button should work.
In particular it is `false` in `match_over`: a finished match must be `reset`
before another can begin, so a stray click cannot throw away results somebody is
still reading.

| `state` | `start` | `pause` | `resume` | `end` | `reset` | `kick` |
|---------|:-------:|:-------:|:--------:|:-----:|:-------:|:------:|
| `open` / `locked` | if `can_start` | — | — | — | — | yes |
| `countdown` | — | yes | — | yes | yes | — |
| `running` | — | if not paused | if paused | yes | yes | — |
| `match_over` | — | — | — | — | yes | yes |

---

## Minimal client

```js
const ws = new WebSocket("ws://127.0.0.1:8080/ws/admin");
const send = (cmd, extra = {}) => ws.send(JSON.stringify({ cmd, ...extra }));

ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.type === "lobby") render(msg);            // redraw seats + buttons
  if (msg.type === "error") alert(msg.message);
};

startButton.onclick  = () => send("start");
pauseButton.onclick  = () => send("pause");
endButton.onclick    = () => send("end");
kickButton.onclick   = () => send("kick", { id: 2 });
```

---

## Notes

- **Bots cannot send names.** Their uplink is two bytes wide, with no room for
  one. Names exist only here, via `rename`, and default to `bot-<id>`.
- **Nothing starts on its own.** The server never auto-starts a full lobby;
  "looks full, go" is exactly how a tournament round begins without one of the
  competitors.
- An empty seat is still listed with `"connected": false`, so the UI can draw a
  fixed set of places instead of a list that shifts as bots connect. It keeps a
  `name` (back to the `bot-<id>` default); only `addr` goes `null`.
- An unparseable command still gets an `error` reply, with an empty `cmd`.
- A bot that has stopped sending keeps its seat but goes stale — watch
  `last_seen_tick` and `packets_per_sec`, and make that obvious before someone
  presses Start.
