# Bomberman Arena

A Bomberman server for bot competitions. Bots play over UDP at 60 ticks per
second; a web interface shows the match and lets a moderator run it.

```
bots ──udp/47800──►  server  ──ws──►  visualizer  (spectate)
                        │     ──ws──►  moderation UI  (start / pause / end)
```

## Try it

```sh
just arena          # server + two example bots
```

Then open <http://127.0.0.1:8080> and press Start.

Or piece by piece:

```sh
just server         # bots on udp/47800, web on :8080
just pybot          # one example bot -- run it twice to fill a lobby
```

There is no visualizer yet, so `/` serves a placeholder page listing the
endpoints. That page means the server is up, not that something is broken.

## Documentation

| File | For |
|------|-----|
| `BOT_GUIDE.md` | Anyone writing a bot. Complete byte layouts; no need to read this source. |
| `VISUALIZER_GUIDE.md` | Whoever builds the front end. The WebSocket JSON contract and the 64×64 asset pack. |
| `MODERATION_API.md` | The start / pause / end controls, on their own short page. |

## How it is put together

```
crates/bomber-domain       the rules, and nothing else -- no I/O, no wire format
crates/bomber-protocol     how the domain is spelled in bytes
crates/bomber-application  the session: admission, the tick, delivery, moderation
crates/bomber-server       the composition root and its two adapters
clients/python             a working reference bot, standard library only
```

The dependency points inward. The domain is pure and deterministic: the same
seed and the same inputs produce the same match, which is what makes replays,
regression tests, and settling an argument about a match possible at all.

One constraint shapes the whole protocol: **the bot uplink is exactly two
bytes**. There is no room for an acknowledgement, so the server cannot delta
against a last-acked tick and instead interleaves complete keyframes with
deltas that name the tick they build on. There is no room for a credential
either, so a player id is only believed when it arrives from the address that
claimed the seat.

```sh
just test           # 92 tests
just lint           # clippy, warnings denied
```

## Hosting it

The flake exposes the server and a NixOS module.

```nix
{
  inputs.bomberman.url = "git+ssh://…/bomberman";

  # in your system configuration
  imports = [ bomberman.nixosModules.default ];

  services.bomberman-arena = {
    enable = true;
    openFirewall = true;          # opens the bot UDP port only
    settings = {
      lobby.min_players = 4;
      rules.round_time_secs = 300;
      map = { width = 21; height = 17; symmetry = "quad"; };
    };
  };
}
```

`nix build .#bomber-server` gives you the binary alone; the annotated default
config lands in `share/bomber-server/server.toml`.

> The moderation WebSocket has **no authentication**. Anyone who can reach the
> web port can start, pause and end matches and kick players. It binds to
> loopback by default; put it behind a reverse proxy with auth rather than
> changing `webAddress` to `0.0.0.0`.
