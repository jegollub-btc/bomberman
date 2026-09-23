# Bomberman Arena — Bot Author's Guide

Everything you need to write a bot. You need a UDP socket and this document;
you do not need to read the server's source.

- **Server port (UDP):** `47800` by default
- **Tick rate:** 60 ticks per second, fixed
- **Protocol version:** 1
- **Byte order:** every multi-byte field is **little-endian**
- **Coordinates:** always **cell indices**, never pixels. `(0,0)` is top-left, `y` grows downward.

---

## 1. Quick start

This is a complete, runnable bot. It ignores the world and wanders at random —
copy it, then replace the last few lines with something that actually thinks.

```python
import random, socket, time

SERVER = ("127.0.0.1", 47800)
UP, DOWN, LEFT, RIGHT, BOMB = 1, 2, 3, 4, 5

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.sendto(bytes([0xFF, 0xFF]), SERVER)          # hello: "give me a slot"

player_id = None
while player_id is None:                          # wait for ASSIGNED (frame type 0x00)
    data, _ = sock.recvfrom(2048)
    if data[0] == 0x00:
        player_id = data[6]
print("I am player", player_id)

sock.setblocking(False)
seq = 0
while True:
    try:
        while True:
            sock.recvfrom(4096)                   # drain the downlink; this bot ignores it
    except BlockingIOError:
        pass

    action = BOMB if random.random() < 0.1 else random.choice([UP, DOWN, LEFT, RIGHT])
    sock.sendto(bytes([player_id, (seq << 4) | action]), SERVER)
    seq = (seq + 1) & 0x0F
    time.sleep(1 / 60)
```

Run the server, run two copies of this, press **Start** in the moderation UI,
and you have a match. Everything below is reference.

---

## 2. How a match runs

```
   you send hello  ──►  ASSIGNED (your player id)
                        LOBBY_STATUS (repeating, while waiting)
   moderator presses Start
                   ──►  MATCH_INIT   (the full static picture, sent 5x)
                   ──►  KEYFRAME + DELTA, 60 per second
                   ──►  MATCH_END
                        back to LOBBY_STATUS
```

You may sit in the lobby for a long time. Keep your socket open and keep
reading; `LOBBY_STATUS` tells you how many slots are filled and whether the
match is about to start.

---

## 3. Connecting

Send the two bytes `0xFF 0xFF` to the server's UDP port.

The server replies with an `ASSIGNED` frame containing your **player id**
(`0`–`3`).

> **Your id is bound to the source address of that packet.** Send everything
> from the same socket, from the same port. Packets whose `player_id` does not
> match the address they came from are dropped without a reply — this is what
> stops one bot from driving another bot's character.

If the lobby is full or locked, you get no `ASSIGNED`. Retry the hello every
half second until you get one.

Sending hello again at any time is safe and is the only way to recover if you
missed `MATCH_INIT`: it re-sends the frame you need.

---

## 4. Sending actions — the two-byte packet

```
byte 0:  player_id          0..=3
byte 1:  (seq << 4) | action_code
```

That is the entire uplink. It is two bytes per tick, forever.

### Action codes (low nibble of byte 1)

| Code | Action | Meaning |
|-----:|--------|---------|
| `0` | `IDLE` | liveness heartbeat -- see below. Identical in effect to sending nothing. |
| `1` | `UP` | step up (y − 1) |
| `2` | `DOWN` | step down (y + 1) |
| `3` | `LEFT` | step left (x − 1) |
| `4` | `RIGHT` | step right (x + 1) |
| `5` | `BOMB` | drop a bomb on your current cell |
| `6` | `UP+BOMB` | drop a bomb, then step up |
| `7` | `DOWN+BOMB` | drop a bomb, then step down |
| `8` | `LEFT+BOMB` | drop a bomb, then step left |
| `9` | `RIGHT+BOMB` | drop a bomb, then step right |
| `10`–`14` | — | reserved; the packet is rejected |
| `15` | `HELLO` | request a slot, or re-request `MATCH_INIT` |

Codes `6`–`9` are not a shortcut — they are the **only** way to bomb and leave
on the same tick, because bomb placement resolves for every player before
anyone moves.

### The sequence nibble (high nibble of byte 1)

Increment it on every packet you send and let it wrap at 16:

```python
seq = (seq + 1) & 0x0F
```

UDP reorders and duplicates datagrams. The counter is what lets the server tell
"this is a new intent" from "this is the same intent arriving twice", and it is
how per-bot packet loss gets measured. **A bot that always sends `seq = 0` will
have most of its packets discarded as duplicates.**

### Pacing

**Send a packet only on ticks where you actually want to do something.**
Silence means "carry on", costs nothing, and is the normal state for a bot that
is mid-step or waiting for a fuse.

- At most **one packet per tick** (≈ every 16.6 ms). Only the **newest packet
  received inside a tick's window** is used; the rest are discarded, so bursting
  wastes bandwidth and nothing else.
- The server never waits for you. Send nothing and your player does nothing that
  tick.
- `IDLE` exists **only** so the moderation UI can tell a thinking bot from a
  crashed one. Send it a couple of times a second while you are idle -- never
  every tick. A bot transmitting 60 "I am doing nothing" packets per second is
  pure noise, and the server treats it identically to silence anyway.

---

## 5. Receiving state

Every server → client frame starts with the same 5-byte header:

| Offset | Type | Field |
|-------:|------|-------|
| 0 | `u8` | frame type |
| 1 | `u32` | tick |

| Type | Name | When |
|-----:|------|------|
| `0x00` | `ASSIGNED` | reply to your hello |
| `0x01` | `LOBBY_STATUS` | every 30 ticks while waiting |
| `0x02` | `MATCH_INIT` | at match start, sent on 5 consecutive ticks |
| `0x03` | `KEYFRAME` | at match start, then every 30 ticks (0.5 s) |
| `0x04` | `DELTA` | every tick that is not a keyframe |
| `0x05` | `MATCH_END` | when the match ends |

### 5.1 Entity ids — which number means what

Four different things on the wire are small integers, and mixing them up is the
easiest mistake to make. They live in **separate namespaces**: bomb `3` and
power-up `3` have nothing to do with each other.

| Namespace | Type | Range | Assigned by | Lives from → to | Reused? |
|-----------|------|-------|-------------|-----------------|---------|
| **player id** | `u8` | `0`–`3` | server, on your hello | you join → you are kicked | Yes. A freed seat is handed to the next bot that says hello. Stable for the whole session, across matches. |
| **bomb id** | `u16` | `1`–`65535` | server, on placement | `BOMB_ADD` → `BOMB_REMOVE` | Wraps after 65535, never `0`. Unique among *live* bombs, which is all you need. |
| **power-up id** | `u16` | `1`–`65535` | server, on drop | `POWERUP_ADD` → `POWERUP_REMOVE` | Same as bombs. |
| **tick** | `u32` | from `0` | server | resets to `0` at each `MATCH_INIT` | Per match, monotonic. |
| **match id** | `u32` | any | server | one match | For logs; you never need it. |

Two things that deliberately have **no id**:

- **Flame cells** — addressed by `(x, y)`. A cell is either burning or it is not;
  there is nothing to track across ticks.
- **Tiles** — addressed by `(x, y)`.

**`0xFF` is the "no player" sentinel.** It appears in `PLAYER_DEATH.killer`
(nobody gets credit), `POWERUP_REMOVE.taken_by` (fire destroyed it, nobody
picked it up), and `MATCH_END.winner` (a draw). It is never a real player id.

Keep three maps and you have the whole world:

```python
players  = {}   # player id  -> position, stats, alive
bombs    = {}   # bomb id    -> cell, fuse, owner
powerups = {}   # powerup id -> cell, kind
flames   = {}   # (x, y)     -> ticks remaining
grid     = []   # [y * width + x] -> tile
```

Every record in §5.6 tells you to insert into, update, or delete from exactly
one of these.

### 5.2 `ASSIGNED` (0x00) — 9 bytes

| Offset | Type | Field |
|-------:|------|-------|
| 5 | `u8` | protocol version (currently `1`) |
| 6 | `u8` | **your player id** |
| 7 | `u8` | tick rate (`60`) |
| 8 | `u8` | max players |

### 5.3 `LOBBY_STATUS` (0x01) — 11 bytes

| Offset | Type | Field |
|-------:|------|-------|
| 5 | `u8` | lobby state (below) |
| 6 | `u8` | players connected |
| 7 | `u8` | max players |
| 8 | `u8` | slot mask — bit *i* set means slot *i* is taken |
| 9 | `u16` | countdown ticks remaining (0 outside `COUNTDOWN`) |

Lobby states: `0` open · `1` locked · `2` countdown · `3` running · `4` match over.

### 5.4 `MATCH_INIT` (0x02) — the initial data

Everything static about the match. Read it once and keep it.

| Offset | Type | Field |
|-------:|------|-------|
| 5 | `u8` | protocol version |
| 6 | `u32` | match id |
| 10 | `u64` | RNG seed (the match is reproducible from this) |
| 18 | `u8` | **your player id** |
| 19 | `u8` | player count *N* |
| 20 | `u8` | board width *W* |
| 21 | `u8` | board height *H* |
| 22 | `u8[ceil(W*H/4)]` | packed tile grid (§5.7) |
| … | `(u8,u8) × N` | spawn cell per player, indexed by player id |
| … | rules block | §5.8 |

### 5.5 `KEYFRAME` (0x03) — complete state

| Offset | Type | Field |
|-------:|------|-------|
| 5 | `u8` | width *W* |
| 6 | `u8` | height *H* |
| 7 | `u8[ceil(W*H/4)]` | packed tile grid — the **current** grid, blocks already destroyed |
| … | `u8` | player count, then that many **player records** (11 bytes each) |
| … | `u16` | bomb count, then that many **bomb records** (7 bytes each) |
| … | `u16` | flame count, then that many **flame records** (3 bytes each) |
| … | `u16` | power-up count, then that many **power-up records** (5 bytes each) |
| … | `u32` | ticks remaining in the match |

**Player record (11 bytes)**

| Type | Field |
|------|-------|
| `u8` | player id |
| `u8` | flags — bit 0 `alive`, bit 1 `moving` |
| `u8` | x |
| `u8` | y |
| `u8` | facing (`0` down, `1` up, `2` left, `3` right) |
| `u8` | move progress (0 = settled) |
| `u8` | max concurrent bombs |
| `u8` | blast radius |
| `u8` | speed level |
| `u16` | score |

**Bomb record (7 bytes):** `u16` id, `u8` owner, `u8` x, `u8` y, `u16` fuse ticks remaining.

**Flame record (3 bytes):** `u8` x, `u8` y, `u8` ticks remaining. **Every listed cell is lethal.**

**Power-up record (5 bytes):** `u16` id, `u8` x, `u8` y, `u8` kind
(`0` extra bomb, `1` bigger flame, `2` speed).

### 5.6 `DELTA` (0x04) — changes since a named tick

| Offset | Type | Field |
|-------:|------|-------|
| 5 | `u32` | **base tick** — the tick this delta builds on |
| 9 | `u16` | record count, then that many records |

Each record is a `u8` tag followed by a fixed payload:

| Tag | Record | Payload | What it means |
|----:|--------|---------|---------------|
| `0x01` | `PLAYER_STATE` | `u8` **player id**, `u8` flags, `u8` x, `u8` y, `u8` dir, `u8` progress | A player moved, turned, or died. Sent every tick for every player whose state changed. |
| `0x02` | `PLAYER_STATS` | `u8` **player id**, `u8` bombs_max, `u8` flame, `u8` speed, `u16` score | Inventory or score changed. |
| `0x03` | `BOMB_ADD` | `u16` **bomb id**, `u8` **player id** (owner), `u8` x, `u8` y, `u16` fuse | A bomb was placed. Remember the id. |
| `0x04` | `BOMB_REMOVE` | `u16` **bomb id** | That bomb is gone. Always paired with an `EXPLOSION` at the same cell. |
| `0x05` | `EXPLOSION` | `u8` x, `u8` y, `u8` up, `u8` down, `u8` left, `u8` right | The blast *shape* — arm lengths from the centre. For animation. The cells that kill arrive as `FLAME_ADD`. |
| `0x06` | `TILE_SET` | `u8` x, `u8` y, `u8` tile | A cell changed: a crate was destroyed (`→ empty`) or sudden death walled it off (`→ solid`). |
| `0x07` | `POWERUP_ADD` | `u16` **power-up id**, `u8` x, `u8` y, `u8` kind | An item dropped from a destroyed crate. |
| `0x08` | `POWERUP_REMOVE` | `u16` **power-up id**, `u8` **player id** or `0xFF` | Item left the board. A player id means collected; `0xFF` means fire destroyed it. |
| `0x09` | `PLAYER_DEATH` | `u8` **player id**, `u8` killer (always `0xFF`) | That player is out for the rest of the match. |
| `0x0A` | `FLAME_ADD` | `u8` x, `u8` y, `u8` ticks | **This cell is now lethal.** The record that matters for staying alive. |
| `0x0B` | `FLAME_REMOVE` | `u8` x, `u8` y | The fire on that cell went out. |
| `0x0C` | `TIMER` | `u32` ticks remaining | Round clock. |
| `0x0D` | `WALL_CLOSED` | `u8` x, `u8` y | Sudden death sealed this cell. A `TILE_SET → solid` for the same cell arrives with it. |

`EXPLOSION` is the blast *shape*, useful for animation. The cells that actually
kill you arrive as `FLAME_ADD` records — trust those.

### 5.7 The packed tile grid

Two bits per cell, row-major, low bits first.

```
index = y * width + x
byte  = packed[index // 4]
tile  = (byte >> ((index % 4) * 2)) & 0b11
```

Tiles: `0` empty (walkable) · `1` solid wall (indestructible) · `2` soft block
(destructible, blocks movement and stops a blast).

A 15×13 board is 49 bytes; a 31×31 board is 241.

### 5.8 The rules block (17 bytes)

Read these rather than hardcoding them — the moderator can retune the server
between matches and your bot will follow along.

| Type | Field | Default |
|------|-------|--------:|
| `u16` | bomb fuse ticks | 120 |
| `u16` | flame duration ticks | 30 |
| `u8` | ticks per cell at speed 0 | 8 |
| `u8` | ticks saved per speed level | 1 |
| `u8` | starting bombs | 1 |
| `u8` | starting blast radius | 1 |
| `u8` | max blast radius | 6 |
| `u8` | max speed level | 3 |
| `u8` | power-up drop chance, percent | 30 |
| `u32` | round length in ticks | 10800 |
| `u32` | sudden-death start tick | 7200 |

Ticks to cross one cell at speed *s*:

```
max(1, ticks_per_cell - ticks_saved_per_level * s)
```

### 5.9 `MATCH_END` (0x05)

| Offset | Type | Field |
|-------:|------|-------|
| 5 | `u8` | reason — `0` last standing, `1` timeout, `2` aborted |
| 6 | `u8` | winner id, or `0xFF` for a draw |
| 7 | `u8` | result count, then that many records |

**Result record (4 bytes):** `u8` id, `u8` placement (`1` = winner), `u16` score.

---

## 6. Handling packet loss — read this one

UDP drops packets. Your uplink is two bytes, so **you cannot acknowledge
anything and you cannot ask for a resend.** Everything in this section follows
from that.

The rule is short:

> Apply a `DELTA` **only** if you currently hold state at exactly its
> `base_tick`. Otherwise throw it away and wait for the next `KEYFRAME`.

A keyframe arrives every 30 ticks, so the worst case is half a second of stale
state. That is the entire recovery mechanism. Do not try to be clever and
apply deltas out of order — you will silently desync and your bot will start
walking into fires it thinks are not there.

```python
static   = None   # from MATCH_INIT
state    = None   # everything dynamic
at_tick  = None   # which tick `state` represents

def on_frame(frame):
    global static, state, at_tick
    if   frame.type == MATCH_INIT:
        static, state, at_tick = parse_match_init(frame), None, None
    elif frame.type == KEYFRAME:
        state, at_tick = parse_keyframe(frame), frame.tick     # always trustworthy
    elif frame.type == DELTA:
        if at_tick is None or frame.base_tick != at_tick:
            return                                            # we missed something
        apply_records(state, frame.records)
        at_tick = frame.tick
    elif frame.type == MATCH_END:
        state, at_tick = None, None
```

Other loss-related facts:

- `MATCH_INIT` is sent on **5 consecutive ticks**. If you still miss it, send
  hello (`0xFF 0xFF`) and it will be sent again.
- Your *outgoing* packets can be lost too. That tick your player just repeats
  nothing — there is no retry and you should not build one.
- Frames can arrive **out of order**. Ignore any frame whose tick is older than
  what you already hold.

---

## 7. The rules

### Movement

- A step **commits the moment it starts.** The instant you send `LEFT`, your
  `x`/`y` become the *destination* cell and stay there; `move_progress` counts
  up to `move_total`.
- You **cannot be redirected or stopped mid-step.** Actions sent during a step
  are ignored. Plan a cell ahead.
- Walking into a wall does not move you but **does turn you**, so you can aim
  without needing an open cell.
- A cell holds **one player**. If two players go for the same cell on the same
  tick, the **lower player id gets it** and the other is blocked.

### Bombs

- `BOMB` drops a bomb on your current cell, if you have one spare
  (`bombs_active < bombs_max`) and there is no bomb there already.
- **Bomb placement resolves for every player before anyone moves**, so
  `LEFT+BOMB` reliably drops and leaves.
- You can **walk off your own bomb**. You can never walk back onto it — bombs
  block movement for everyone, and the only reason you were ever on one is that
  you placed it.
- The fuse is fixed; there is no remote detonation.
- Your bomb count is refunded when the bomb detonates, not when it is placed.

### Blast

- The blast is a cross: `flame` cells in each of the four directions from the
  bomb.
- A **solid wall** stops it dead and is unharmed.
- A **soft block** stops it *and* is destroyed. The cell behind a soft block is
  shielded.
- **Chain reactions resolve inside a single tick.** A bomb caught in a blast
  detonates immediately, and so does anything *its* blast reaches. A ten-bomb
  chain happens in one tick, not ten.
- A power-up caught in a blast is **destroyed**, not collected.
- Flame cells stay lethal for `flame_duration_ticks`.

### Death

- You die if you are standing on a flame cell at the end of a tick. That covers
  both "a bomb went off under you" and "you walked into an existing fire".
- Deaths carry no kill credit (`killer` is `0xFF`). Chain reactions make the
  question genuinely ambiguous, so nobody is blamed.
- Death is permanent for the match.

### Power-ups

Dropped by destroyed soft blocks with `powerup_chance_pct` probability, then
equally likely to be one of:

| Kind | Effect |
|------|--------|
| `0` extra bomb | +1 concurrent bomb, uncapped |
| `1` bigger flame | +1 blast radius, capped at `max_flame` |
| `2` speed | +1 speed level (fewer ticks per cell), capped at `max_speed` |

Picked up by occupying the cell.

### Sudden death and the timer

- From `sudden_death_tick`, one cell turns into a solid wall every **6 ticks**,
  spiralling inward from the outer ring. Anything on that cell — player, bomb,
  power-up — is destroyed.
- At `round_time_ticks` the match ends regardless. If several players are still
  alive, the highest score wins; a tie is a draw.

### Scoring

| Event | Points |
|-------|-------:|
| Destroy a soft block | +10 |
| Kill another player | +100 |
| Be alive when the match ends | +500 |

### Tick order

The order is fixed and it is part of the rules:

1. Existing flames age; expired ones clear.
2. Fuses tick down; detonations and their full chain resolve; blocks are
   destroyed, power-ups spawn, new flames appear.
3. **Bombs are placed** (all players), then **players move** (in id order).
4. Power-ups are collected.
5. Anyone standing in a flame dies.
6. Sudden death walls off its next cell.
7. Win condition is evaluated.

The practical consequence: a flame that appears in step 2 kills you in step 5
of the *same* tick, even if you moved in step 3.

---

## 8. Your tick budget

You have **under 16.6 ms** per tick.

A slow bot does not stall anyone. The server never waits: it takes whatever
arrived in the tick window and moves on. What a slow bot loses is its own
turns — miss a window and your player does nothing that tick.

The one non-negotiable rule: **never block your receive loop.** Reading the
socket and deciding should not be serialised behind each other. Drain
everything available, then decide, then send.

Bandwidth is small: roughly 5–8 KB/s down and 120 B/s up per bot.

---

## 9. Reference implementations

| Language | Path | What it handles for you |
|----------|------|-------------------------|
| Python | `clients/python/` | hello handshake, sequence counter, keyframe/delta reconstruction; stdlib `socket` only, zero dependencies |
| Rust | `crates/bomber-bot/` | the same, plus the shared `bomber-proto` types |

Both expose the same shape: implement `on_tick(state) -> Action` and the SDK
does the networking.

---

## 10. Testing your bot

```sh
just server          # starts the arena on udp/47800 and the web UI on :8080
just pybot           # the example Python bot
just bot             # the example Rust bot
```

Open <http://127.0.0.1:8080>, wait for your bot to appear in the lobby, press
**Start**, and watch it play.

To run against a tuned ruleset, edit `config/server.toml` and restart — bots
pick up the new constants from `MATCH_INIT` with no rebuild.

---

## 11. Troubleshooting

| Symptom | Almost certainly |
|---------|------------------|
| Never receive `ASSIGNED` | Lobby is full or locked, or you are sending to the wrong port. Retry hello every 500 ms. |
| `ASSIGNED` arrives, then nothing | You are reading with a different socket than you sent hello from. Identity is bound to the source address. |
| Actions are ignored | Byte 0 is not your assigned id, or you used a reserved action code (`10`–`14`). Both are dropped silently. |
| Moderation UI shows your bot as stale while it is thinking | You are sending nothing at all. Send `IDLE` (`0`) a couple of times a second so the UI can see you are alive. |
| You track a bomb that never disappears | You matched a `BOMB_REMOVE` against a power-up or player id. The three id namespaces are separate (§5.1). |
| Only some actions register | You are not incrementing the sequence nibble, so repeats look like duplicates. |
| Player drifts from where you think it is | You applied a `DELTA` whose `base_tick` you did not hold. Discard and wait for a `KEYFRAME`. |
| State freezes for half a second, then jumps | Normal after packet loss — that is the keyframe arriving. If it happens constantly, check for a blocked receive loop. |
| You bomb and die to your own blast | Use `LEFT+BOMB` (`8`) and friends. A bare `BOMB` leaves you standing on it. |
| You walk into fire you thought had expired | Flames last `flame_duration_ticks`, and step 5 of the tick order kills you in the same tick you move. |
