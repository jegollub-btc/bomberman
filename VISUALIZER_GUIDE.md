# Bomberman Arena — Visualizer & Moderation UI Guide

Everything needed to build the front end. The server is the only thing you talk
to, over two WebSocket endpoints. You never touch UDP — that is the bots' side.

You are building **one app with two jobs**:

1. **Spectator view** — render the live match at 60 ticks/s.
2. **Moderation console** — see who has connected, and start/reset/kick.

- **Server:** `http://127.0.0.1:8080` by default (`web_bind` in `config/server.toml`)
- **Spectate:** `ws://127.0.0.1:8080/ws/spectate` — read-only
- **Admin:** `ws://127.0.0.1:8080/ws/admin` — everything spectate sends, plus commands
- **Wire format:** JSON, both directions
- **Coordinates:** cell indices. `(0,0)` is top-left, `y` grows downward.

Tech is your call. The reference shape is Vite + TypeScript with the board on a
`<canvas>` and the surrounding UI in whatever framework you like — but nothing
in this document depends on that choice.

---

## 1. The protocol in one picture

```
  connect /ws/spectate
        ──►  { "type": "lobby",      ... }    who is connected, what state we're in
        ──►  { "type": "match_init", ... }    only if a match is already running
   moderator presses Start
        ──►  { "type": "match_init", ... }    the board, the spawns, the rules
        ──►  { "type": "state",      ... }    x60 per second
        ──►  { "type": "match_end",  ... }
        ──►  { "type": "lobby",      ... }    back to waiting
```

Every message is a JSON object with a `type` field. Ignore types you do not
recognise — new ones may be added.

TCP underneath, so unlike the bots you get **no packet loss and no reordering**.
There is no delta reconstruction on your side: `state` is complete every tick.

---

## 2. Messages from the server

### 2.1 `lobby`

Sent on connect and whenever anything about the roster or state changes.

```json
{
  "type": "lobby",
  "state": "open",
  "max_players": 4,
  "countdown_ticks": 0,
  "can_start": true,
  "map": { "width": 15, "height": 13, "density": 0.75, "symmetry": "quad", "seed": 0 },
  "slots": [
    {
      "id": 0,
      "name": "bot-0",
      "addr": "10.12.3.44:51234",
      "connected": true,
      "rtt_ms": 3,
      "packets_per_sec": 60,
      "loss_pct": 0.4,
      "last_seen_tick": 1233
    },
    { "id": 1, "name": null, "addr": null, "connected": false,
      "rtt_ms": null, "packets_per_sec": 0, "loss_pct": 0, "last_seen_tick": null }
  ]
}
```

`state` is one of `open`, `locked`, `countdown`, `running`, `match_over`.
Empty slots are still listed, with `connected: false`, so the UI can render a
fixed grid of seats rather than a growing list.

`can_start` is the server's own answer to "would a Start command succeed right
now" — use it to enable/disable the button instead of re-deriving the rule.

### 2.2 `match_init`

The static picture. Sent once when a match starts, and immediately on connect
if a match is already in progress (so a late-joining spectator is not stuck
waiting).

```json
{
  "type": "match_init",
  "match_id": 42,
  "seed": "10873452098734512",
  "tick_rate": 60,
  "width": 15,
  "height": 13,
  "tiles": [1,1,1,1,"…", 1],
  "spawns": [[1,1],[13,1],[1,11],[13,11]],
  "players": [ { "id": 0, "name": "bot-0" }, { "id": 1, "name": "bot-1" } ],
  "rules": {
    "bomb_fuse_ticks": 120,
    "flame_duration_ticks": 30,
    "ticks_per_cell": 8,
    "speed_step_ticks": 1,
    "start_bombs": 1,
    "start_flame": 1,
    "max_flame": 6,
    "max_speed": 3,
    "powerup_chance_pct": 30,
    "round_time_ticks": 10800,
    "sudden_death_tick": 7200
  }
}
```

- `tiles` is a flat row-major array of length `width * height`. Index is
  `y * width + x`. Values: **`0` empty, `1` solid wall, `2` soft block (crate)**.
- `seed` is **a string**, not a number. It is a 64-bit value and would lose
  precision as a JSON number. You only need it for display.

### 2.3 `state` — one per tick

```json
{
  "type": "state",
  "tick": 1234,
  "ticks_remaining": 9566,
  "players": [
    {
      "id": 0, "alive": true, "moving": true,
      "x": 3, "y": 5, "dir": "left",
      "move_progress": 4, "move_total": 8,
      "bombs_max": 2, "flame": 3, "speed": 1, "score": 120
    }
  ],
  "bombs":    [ { "id": 9, "owner": 0, "x": 3, "y": 6, "fuse": 77 } ],
  "flames":   [ { "x": 5, "y": 5, "ticks": 12 } ],
  "powerups": [ { "id": 4, "x": 7, "y": 7, "kind": "speed" } ],
  "tile_changes": [ { "x": 4, "y": 6, "tile": "empty" } ],
  "events": []
}
```

- `players`, `bombs`, `flames` and `powerups` are **complete each tick** —
  replace your copies wholesale, do not merge.
- `tiles` is *not* resent. Apply `tile_changes` to the grid you got from
  `match_init`. It is almost always empty.
- `dir` is `"up" | "down" | "left" | "right"`.
- `kind` is `"extra_bomb" | "flame" | "speed"`.
- `tile` is `"empty" | "solid" | "soft"`.

**`move_progress` / `move_total` are what you interpolate with.** See §4.2 —
and note that a moving player's `x`/`y` are already the *destination*.

### 2.4 Entity ids

Three different things are numbered, in **separate namespaces**. Bomb `3` and
power-up `3` are unrelated.

| Field | Namespace | Range | Lives from → to |
|-------|-----------|-------|-----------------|
| `player` / `players[].id` | player id | `0`–`3` | joins the lobby → is kicked. Stable across matches. |
| `bomb` / `bombs[].id` | bomb id | `1`+ | `bomb_placed` → `explosion` |
| `powerup` / `powerups[].id` | power-up id | `1`+ | `powerup_spawned` → `powerup_taken` / `powerup_burned` |

Flames and tiles have no id — they are addressed by `(x, y)`, because a cell is
either burning or it is not and there is nothing to follow across ticks.

Events always name the namespace in the field itself (`player`, `bomb`,
`powerup`) rather than a bare `id`, precisely so this cannot be got wrong.

### 2.5 `events`

Discrete things that happened on this tick, carried inside `state`. They exist
because some things are invisible in a state diff — a bomb that explodes and
clears within one tick, for instance — and because animations and sounds need a
trigger, not a level.

```json
{ "type": "bomb_placed",     "bomb": 9, "player": 0, "x": 3, "y": 6 }
{ "type": "explosion",       "bomb": 9, "x": 3, "y": 6,
                             "up": 1, "down": 2, "left": 0, "right": 3 }
{ "type": "block_destroyed", "x": 4, "y": 6 }
{ "type": "powerup_spawned", "powerup": 5, "x": 4, "y": 6, "kind": "extra_bomb" }
{ "type": "powerup_taken",   "powerup": 4, "player": 0, "kind": "speed" }
{ "type": "powerup_burned",  "powerup": 4, "x": 7, "y": 7 }
{ "type": "death",           "player": 3 }
{ "type": "wall_closed",     "x": 1, "y": 1 }
```

| Event | Draw / play |
|-------|-------------|
| `bomb_placed` | Start the `bomb_tick` loop on that cell. |
| `explosion` | Compose the flame sprites from the arm lengths (§4.4) and start the 5-frame animation. **This is the one that drives the explosion art.** |
| `block_destroyed` | Play `crate_break_0…3` on that cell, once. |
| `powerup_spawned` | Item appears; start its `hover` loop. |
| `powerup_taken` | Pickup chime, brief flash on the collecting player. |
| `powerup_burned` | Item destroyed by fire — a different, sadder effect than being taken. |
| `death` | Death animation and sound for that player. |
| `wall_closed` | Sudden death sealed a cell; worth a thud and a shake. |

`explosion` gives you the blast **shape** — arm lengths in each direction from
the centre. The `flames` array tells you which cells are *currently* lethal. Use
the event to start an animation, the array to know what to keep drawing.

### 2.6 `match_end`

```json
{
  "type": "match_end",
  "tick": 5000,
  "reason": "last_standing",
  "winner": 0,
  "results": [
    { "id": 0, "placement": 1, "score": 1300 },
    { "id": 1, "placement": 2, "score": 400 }
  ]
}
```

`reason` is `"last_standing" | "timeout" | "aborted"`. `winner` is `null` on a
draw. Tied players share a `placement`.

The server stays on `match_over` until the moderator resets, so the results
screen can sit there as long as needed.

### 2.7 `ack` / `error` (admin only)

Every command gets exactly one reply:

```json
{ "type": "ack",   "cmd": "start" }
{ "type": "error", "cmd": "start", "message": "need at least 2 players" }
```

### 2.8 `map_preview` (admin only)

Reply to `preview_map`. Same shape as the board part of `match_init`, so you
can render it with the same code:

```json
{ "type": "map_preview", "width": 15, "height": 13,
  "tiles": [], "spawns": [[1,1]], "seed": "123456789" }
```

---

## 3. Commands (admin socket)

Send JSON objects with a `cmd` field. `MODERATION_API.md` covers this channel on
its own if you only need the controls.

| Command | Payload | Effect |
|---------|---------|--------|
| `start` | — | Begin the countdown. Fails unless at least 2 slots are filled. |
| `pause` / `resume` | — | Freeze/unfreeze the tick loop. Bots keep their sockets; the clock stops. |
| `end` | — | End the running match now. Emits `match_end` with reason `aborted`, and leaves the results up. |
| `reset` | — | Clear the results and return to the lobby. |
| `lock` / `unlock` | — | Stop/allow new bots joining. |
| `kick` | `{"id": 2}` | Free that slot. The bot must send hello again to rejoin. |
| `rename` | `{"id": 2, "name": "team-rocket"}` | Set the display name for a slot. |
| `configure_map` | `{"width":15,"height":13,"density":0.75,"symmetry":"quad","seed":0}` | Settings for the *next* match. All fields optional. |
| `preview_map` | — | Generate a board with the current settings and return it, without starting anything. |

Notes that will save you time:

- **Bots cannot send their own names.** Their uplink is two bytes wide — there
  is no room for a name, ever. Names come from `rename` and are purely a UI
  convenience. Default is `bot-<id>`.
- `width`/`height` are forced odd and clamped to 7…63 by the server. Echo back
  what the server reports, not what the user typed.
- `symmetry` is `"none" | "mirror_x" | "quad"`. Default `quad`, and it is worth
  a tooltip: it mirrors the crate layout so no player is dealt a more open
  corner, which is what keeps results comparable between bots.
- `seed: 0` means "random per match". The actual seed used is reported in
  `match_init`.

---

## 4. Rendering

### 4.1 The asset pack

`bomberman-assets.zip` contains:

```
spritesheet.png   512x640, 8 columns x 10 rows, RGBA
atlas.json        frame rectangles + 22 animation groups with recommended fps
sprites/          the same 73 frames as individual PNGs
preview.html      standalone viewer, open it by double-clicking
```

Every frame is **64x64**, transparent background, origin top-left.

| What | Frames |
|------|--------|
| Floor | `floor_0`, `floor_1` — seamlessly tileable |
| Solid wall | `wall_solid` |
| Crate (soft block) | `crate`, plus `crate_break_0…3` for the destruction animation |
| Player | `player_{down,up,left,right}_{0…3}` — walk cycle is `0 → 1 → 2 → 3`, frames 0 and 2 are the standing poses |
| Bomb | `bomb_0…3`, a visibly burning fuse, looping at 8 fps |
| Explosion | `expl_{center,arm_h,arm_v,tip_up,tip_down,tip_left,tip_right}_{0…4}` |
| Power-ups | `item_{bomb_up,fire_up,speed_up,kick,remote}_{0,1}` |

Two things to know up front:

- **`item_kick` and `item_remote` are unused.** The simulation ships three
  power-ups (`extra_bomb` → `item_bomb_up`, `flame` → `item_fire_up`, `speed` →
  `item_speed_up`). The other two sprites are there for later.
- **There is one player character, not four.** All four players share the same
  sprites, so you need to distinguish them yourself — a per-player tint
  (draw the sprite to an offscreen canvas, composite a colour with
  `source-atop`), or a coloured ring/nameplate under the feet. Decide early;
  it affects your draw loop. Recolouring by hue alone tends to muddy pixel art,
  so a marker under the player is usually the safer call.

### 4.2 Interpolation — the one formula that matters

A moving player's `x`/`y` are **already the destination cell**. The step
commits the instant it starts and cannot be cancelled. So to draw the player
between cells you walk *backwards* from the destination:

```js
const CELL = 64;
const DELTA = { up: [0,-1], down: [0,1], left: [-1,0], right: [1,0] };

function pixelPos(p) {
  if (!p.moving) return [p.x * CELL, p.y * CELL];
  const [dx, dy] = DELTA[p.dir];
  const behind = 1 - p.move_progress / p.move_total;   // 1 at the start, 0 at the end
  return [(p.x - dx * behind) * CELL, (p.y - dy * behind) * CELL];
}
```

At `move_progress = move_total` the player settles, `moving` flips to false and
`x`/`y` are unchanged — so the two branches meet exactly and there is no visible
snap.

Pick the walk-cycle frame from `move_progress`, not from wall-clock time, so
the animation stays locked to the simulation:

```js
const frame = p.moving ? Math.floor(p.move_progress / p.move_total * 4) % 4 : 0;
```

### 4.3 Board size and the canvas

`CELL = 64` is **yours alone** — the server speaks only in cells and knows
nothing about pixels. A 15x13 board is 960x832 px, which fits a 1080p screen
with room for the HUD.

For bigger boards, scale by an **integer** factor (2x, or 1/2, 1/3) rather than
a fractional one; fractional scaling of pixel art produces uneven pixel sizes
that look like a rendering bug.

```js
ctx.imageSmoothingEnabled = false;                  // or everything turns to mush
canvas.width  = boardW * CELL * devicePixelRatio;   // backing store
canvas.height = boardH * CELL * devicePixelRatio;
canvas.style.width  = `${boardW * CELL}px`;         // CSS size
canvas.style.height = `${boardH * CELL}px`;
ctx.scale(devicePixelRatio, devicePixelRatio);
```

### 4.4 Composing an explosion

The explosion is deliberately split into pieces so a blast of any length is
built from the same tiles. For an `explosion` event with `right: 2`:

```
  centre cell      →  expl_center_<f>
  one cell right   →  expl_arm_h_<f>
  two cells right  →  expl_tip_right_<f>
```

Vertical arms use `expl_arm_v`, and each direction has its own tip. **All
pieces of one frame must use the same frame index `<f>`** or the flame will
visibly tear. `expl_center` already contains both bars at full thickness plus a
larger fireball, so the crossing never pinches.

Drive `<f>` from the explosion's age at 14 fps over 5 frames, and let it play
out even if the underlying `flames` entries expire first.

### 4.5 Draw order

1. Floor (alternate `floor_0`/`floor_1` — a checker or a hash of `x,y`, your call)
2. Crates (`crate`, or `crate_break_*` while the destruction animation plays)
3. Solid walls
4. Power-ups (`item_*_hover`, 3 fps)
5. Bombs (`bomb_tick`, 8 fps)
6. Flames
7. Players — **sorted by y**, so a player on a lower row overlaps one above
8. Nameplates / HUD overlay

If any sprite is taller than 64 px, anchor it **bottom-centre** on the cell, not
top-left. Getting this wrong shows up as a half-cell offset that is
surprisingly hard to spot later.

### 4.6 Frame pacing

The server sends 60 states per second, which on a 60 Hz display means one state
per frame. Do not drive rendering from the socket directly:

- Keep the two most recent `state` messages.
- Render on `requestAnimationFrame` and interpolate between them by wall-clock
  time.

On a 144 Hz display this is the difference between smooth and juddery. On a slow
or hitching connection it also degrades gracefully instead of freezing.

---

## 5. What the UI needs to show

### Lobby

- Four seats, always visible, filled or empty.
- Per seat: name (editable — sends `rename`), source address, RTT, packets/s,
  loss %, and a clear connected/stale indicator. A bot that stops sending
  should look obviously dead before someone presses Start.
- Map controls: width, height, density, symmetry, seed, with a **preview board**
  rendered from `preview_map`.
- Buttons: Start (disabled unless `can_start`), Lock/Unlock, Kick per seat.

### Match

- The board.
- Per-player HUD: name, score, bombs, flame, speed, alive/dead.
- Round timer from `ticks_remaining` (divide by `tick_rate`).
- A visible warning when `tick >= sudden_death_tick` — the board starting to
  close in is confusing if it is unannounced.
- A scrolling event log is cheap to build from `events` and very useful when
  someone asks "what just killed me".

### Post-match

- Results table from `match_end`, sorted by placement.
- Reset button back to the lobby.

---

## 6. Running it locally

```sh
just server     # arena on udp/47800, web on :8080
just pybot      # an example bot, run it twice to fill the lobby
```

The server serves the built front end from `visualizer/dist` if it exists, so a
production build is reachable at `http://127.0.0.1:8080` with no extra process.

For development, run your dev server separately and proxy the sockets. With
Vite:

```js
// vite.config.ts
export default {
  server: {
    proxy: { "/ws": { target: "ws://127.0.0.1:8080", ws: true } }
  }
}
```

Then connect to `/ws/spectate` and `/ws/admin` relative to your own origin, and
the same code works in dev and in production.

---

## 7. Things that will bite you

| Symptom | Cause |
|---------|-------|
| Players teleport a cell ahead when they start moving | You drew `x`/`y` directly. They are the destination from the first tick of a step — interpolate backwards (§4.2). |
| Board is blank after a reconnect mid-match | You waited for the next `match_init`. It is sent on connect if a match is running; handle it at any time, not just at match start. |
| Crates reappear after a while | You rebuilt the grid from `match_init` and ignored `tile_changes`. |
| The seed shown is wrong in the last digits | It came through as a JSON number. It is a string for exactly this reason. |
| Explosions tear or flicker at the joins | Different frame indices across the centre/arm/tip pieces of the same blast. |
| Sprites look blurry or unevenly scaled | `imageSmoothingEnabled` left on, or a fractional canvas scale. |
| All four players look identical | They are — one character in the asset pack. Tint or badge them yourself (§4.1). |
| Animation speeds up and slows down | Driven by wall-clock instead of `move_progress`. |
