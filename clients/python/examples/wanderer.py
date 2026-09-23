#!/usr/bin/env python3
"""A complete Bomberman arena bot, in the standard library alone.

It does the whole job described in BOT_GUIDE.md: the hello handshake, the
sequence counter, and keyframe/delta reconstruction with the base-tick check
that is the entire loss-recovery story. The strategy is deliberately plain --
do not stand in fire, break crates, go find more crates -- so the networking
stays the part worth reading.

    python3 clients/python/examples/wanderer.py [host] [port]
"""

import random
import socket
import struct
import sys
import time

HOST = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 47800
SERVER = (HOST, PORT)

# Frame types.
ASSIGNED, LOBBY_STATUS, MATCH_INIT, KEYFRAME, DELTA, MATCH_END = range(6)

# Actions. Codes 6..9 are move+bomb, the only way to drop and leave on one tick.
IDLE, UP, DOWN, LEFT, RIGHT, BOMB = 0, 1, 2, 3, 4, 5
BOMB_AND = {UP: 6, DOWN: 7, LEFT: 8, RIGHT: 9}

EMPTY, SOLID, SOFT = 0, 1, 2
STEP = {UP: (0, -1), DOWN: (0, 1), LEFT: (-1, 0), RIGHT: (1, 0)}
NO_PLAYER = 0xFF


class World:
    """Everything the bot knows, rebuilt from keyframes and nudged by deltas."""

    def __init__(self):
        self.me = None
        self.width = self.height = 0
        self.tiles = []
        self.rules = {}
        self.at_tick = None          # which tick `self` represents
        self.synced = False          # is that tick still the server's?
        self.players = {}            # player id  -> dict
        self.bombs = {}              # bomb id    -> dict
        self.powerups = {}           # powerup id -> dict
        self.flames = {}             # (x, y)     -> ticks remaining

    # -- geometry ------------------------------------------------------------
    def tile(self, x, y):
        if 0 <= x < self.width and 0 <= y < self.height:
            return self.tiles[y * self.width + x]
        return SOLID

    def walkable(self, x, y):
        if self.tile(x, y) != EMPTY:
            return False
        return not any(b["x"] == x and b["y"] == y for b in self.bombs.values())

    def blast_cells(self, x, y, radius):
        """Where a bomb at (x, y) reaches. A crate stops it and burns with it."""
        cells = {(x, y)}
        for dx, dy in STEP.values():
            for step in range(1, radius + 1):
                cx, cy = x + dx * step, y + dy * step
                tile = self.tile(cx, cy)
                if tile == SOLID:
                    break
                cells.add((cx, cy))
                if tile == SOFT:
                    break
        return cells

    def danger(self):
        """Cells burning now, plus everything a live bomb is about to cover."""
        doomed = set(self.flames)
        for bomb in self.bombs.values():
            owner = self.players.get(bomb["owner"], {})
            radius = max(1, owner.get("flame", self.rules.get("start_flame", 1)))
            doomed |= self.blast_cells(bomb["x"], bomb["y"], radius)
        return doomed

    # -- frames --------------------------------------------------------------
    def on_match_init(self, data):
        self.me = data[18]
        count = data[19]
        self.width, self.height = data[20], data[21]
        packed_len = (self.width * self.height + 3) // 4
        self.tiles = self._unpack(data[22:22 + packed_len], self.width * self.height)

        at = 22 + packed_len + count * 2
        fields = struct.unpack_from("<HHBBBBBBBII", data, at)
        self.rules = dict(zip((
            "bomb_fuse_ticks", "flame_duration_ticks", "ticks_per_cell",
            "speed_step_ticks", "start_bombs", "start_flame", "max_flame",
            "max_speed", "powerup_chance_pct", "round_time_ticks",
            "sudden_death_tick"), fields))
        self.at_tick = None
        self.synced = False
        print(f"match: {self.width}x{self.height}, I am player {self.me}")

    def on_keyframe(self, data):
        """Always trustworthy: replace everything."""
        tick = struct.unpack_from("<I", data, 1)[0]
        self.width, self.height = data[5], data[6]
        packed_len = (self.width * self.height + 3) // 4
        at = 7
        self.tiles = self._unpack(data[at:at + packed_len], self.width * self.height)
        at += packed_len

        player_count = data[at]
        at += 1
        self.players.clear()
        for _ in range(player_count):
            pid, flags, x, y, facing, progress, bombs, flame, speed, score = \
                struct.unpack_from("<BBBBBBBBBH", data, at)
            self.players[pid] = dict(
                alive=bool(flags & 1), moving=bool(flags & 2), x=x, y=y,
                dir=facing, progress=progress, bombs_max=bombs, flame=flame,
                speed=speed, score=score)
            at += 11

        self.bombs.clear()
        count, = struct.unpack_from("<H", data, at)
        at += 2
        for _ in range(count):
            bid, owner, x, y, fuse = struct.unpack_from("<HBBBH", data, at)
            self.bombs[bid] = dict(owner=owner, x=x, y=y, fuse=fuse)
            at += 7

        self.flames.clear()
        count, = struct.unpack_from("<H", data, at)
        at += 2
        for _ in range(count):
            x, y, ticks = struct.unpack_from("<BBB", data, at)
            self.flames[(x, y)] = ticks
            at += 3

        self.powerups.clear()
        count, = struct.unpack_from("<H", data, at)
        at += 2
        for _ in range(count):
            uid, x, y, kind = struct.unpack_from("<HBBB", data, at)
            self.powerups[uid] = dict(x=x, y=y, kind=kind)
            at += 5

        self.at_tick = tick
        self.synced = True

    def on_delta(self, data):
        """Apply only against exactly the tick this was built from.

        This check is the whole of loss recovery. There is no way to ask for a
        resend, so a delta we cannot chain onto is dropped and we wait for the
        next keyframe -- at most half a second away.
        """
        tick, base = struct.unpack_from("<II", data, 1)
        if self.at_tick is None or base != self.at_tick:
            # We are now holding a picture of the world that has moved on
            # without us. Acting on it is worse than doing nothing: the fire we
            # can see is old, and the fire that kills us is not in it yet.
            self.synced = False
            return False
        count, = struct.unpack_from("<H", data, 9)
        at = 11
        for _ in range(count):
            tag = data[at]
            at += 1
            if tag == 0x01:       # PLAYER_STATE
                pid, flags, x, y, facing, progress = struct.unpack_from("<BBBBBB", data, at)
                self.players.setdefault(pid, {}).update(
                    alive=bool(flags & 1), moving=bool(flags & 2),
                    x=x, y=y, dir=facing, progress=progress)
                at += 6
            elif tag == 0x02:     # PLAYER_STATS
                pid, bombs, flame, speed, score = struct.unpack_from("<BBBBH", data, at)
                self.players.setdefault(pid, {}).update(
                    bombs_max=bombs, flame=flame, speed=speed, score=score)
                at += 6
            elif tag == 0x03:     # BOMB_ADD
                bid, owner, x, y, fuse = struct.unpack_from("<HBBBH", data, at)
                self.bombs[bid] = dict(owner=owner, x=x, y=y, fuse=fuse)
                at += 7
            elif tag == 0x04:     # BOMB_REMOVE
                bid, = struct.unpack_from("<H", data, at)
                self.bombs.pop(bid, None)
                at += 2
            elif tag == 0x05:     # EXPLOSION -- shape only, for animation
                at += 6
            elif tag == 0x06:     # TILE_SET
                x, y, tile = struct.unpack_from("<BBB", data, at)
                self.tiles[y * self.width + x] = tile
                at += 3
            elif tag == 0x07:     # POWERUP_ADD
                uid, x, y, kind = struct.unpack_from("<HBBB", data, at)
                self.powerups[uid] = dict(x=x, y=y, kind=kind)
                at += 5
            elif tag == 0x08:     # POWERUP_REMOVE
                uid, = struct.unpack_from("<H", data, at)
                self.powerups.pop(uid, None)
                at += 3
            elif tag == 0x09:     # PLAYER_DEATH
                self.players.setdefault(data[at], {})["alive"] = False
                at += 2
            elif tag == 0x0A:     # FLAME_ADD
                x, y, ticks = struct.unpack_from("<BBB", data, at)
                self.flames[(x, y)] = ticks
                at += 3
            elif tag == 0x0B:     # FLAME_REMOVE
                x, y = struct.unpack_from("<BB", data, at)
                self.flames.pop((x, y), None)
                at += 2
            elif tag == 0x0C:     # TIMER
                at += 4
            elif tag == 0x0D:     # WALL_CLOSED
                at += 2
            else:
                self.synced = False   # unknown tag: the rest is unreadable
                return False
        self.at_tick = tick
        return True

    @staticmethod
    def _unpack(packed, count):
        """Two bits per cell, row-major, low bits first."""
        return [(packed[i // 4] >> ((i % 4) * 2)) & 0b11 for i in range(count)]


def bfs(world, start, goal, blocked=()):
    """First step of a shortest path from `start` to any cell `goal` accepts.

    Returns (direction, distance) or (None, None). Plain breadth-first search:
    a board is a few hundred cells, so there is nothing to be clever about.

    `blocked` matters more than it looks. Checking only the *destination* for
    danger is not enough -- a route to a safe cell will happily step through a
    burning one on the way, and that is fatal on arrival, not on departure.
    """
    seen = {start}
    queue = [(start, None, 0)]
    while queue:
        (cx, cy), first, dist = queue.pop(0)
        if (cx, cy) != start and goal(cx, cy):
            return first, dist
        for direction, (dx, dy) in STEP.items():
            nxt = (cx + dx, cy + dy)
            if nxt in seen or nxt in blocked or not world.walkable(*nxt):
                continue
            seen.add(nxt)
            queue.append((nxt, direction if first is None else first, dist + 1))
    return None, None


def escape_route(world, here, doomed):
    """The first step of a route out of `doomed`, and how many steps it takes.

    Two things this has to get right, both learned the hard way:

    * Checking only the four adjacent cells can never succeed at blast radius 1
      -- every neighbour of a bomb is inside its own blast -- so a bot that asks
      that question never places one.
    * Knowing that *an* escape exists is not enough. Stepping off in a random
      direction walks into the dead-end branch about as often as not, and the
      bot dies to its own bomb. Take the first step of the route you verified.

    `here` is blocked because once we step off it, our own bomb seals it; so is
    every burning cell, because a route out must not run through fire.
    """
    return bfs(world, here, lambda x, y: (x, y) not in doomed,
               blocked={here} | set(world.flames))


def steps_available(world):
    """How many cells we can cross before a bomb placed now goes off."""
    per_cell = max(1, world.rules.get("ticks_per_cell", 8))
    return world.rules.get("bomb_fuse_ticks", 120) // per_cell


def decide(world):
    """Do not stand in fire; break crates; otherwise go and find one."""
    me = world.players.get(world.me)
    if not me or not me.get("alive") or me.get("moving"):
        return None               # a step commits: nothing to say until it ends

    x, y = me["x"], me["y"]
    doomed = world.danger()

    # Standing somewhere lethal is the only emergency.
    if (x, y) in doomed:
        # Cells that are on fire *now* are impassable. Cells merely due to
        # explode are not: getting out may mean crossing one, and a fuse is
        # many steps long.
        step, _ = bfs(world, (x, y), lambda cx, cy: (cx, cy) not in doomed,
                      blocked=set(world.flames))
        if step is not None:
            return step
        anywhere = [d for d, (dx, dy) in STEP.items() if world.walkable(x + dx, y + dy)]
        return random.choice(anywhere) if anywhere else None

    # A power-up within easy reach beats breaking another crate: it is about to
    # be destroyed by somebody's next blast, and the crate will still be there.
    # This is checked before bombing, because our own blast is usually what ends
    # up blocking the route to it.
    wanted = {(u["x"], u["y"]) for u in world.powerups.values()}
    if wanted:
        step, dist = bfs(world, (x, y), lambda cx, cy: (cx, cy) in wanted,
                         blocked=doomed)
        if step is not None and dist <= 6:
            return step

    # Next to a crate, with a bomb spare and a way out: light it up and leave.
    beside_crate = any(world.tile(x + dx, y + dy) == SOFT for dx, dy in STEP.values())
    mine = sum(1 for b in world.bombs.values() if b["owner"] == world.me)
    if beside_crate and mine < me.get("bombs_max", 1):
        blast = world.blast_cells(x, y, me.get("flame", 1))
        step, dist = escape_route(world, (x, y), doomed | blast)
        if step is not None and dist <= steps_available(world) - 1:
            return BOMB_AND[step]

    # Otherwise head for the nearest crate worth breaking.
    step, _ = bfs(
        world, (x, y),
        lambda cx, cy: any(world.tile(cx + dx, cy + dy) == SOFT
                           for dx, dy in STEP.values()),
        blocked=doomed,   # nothing is worth walking through a blast for
    )
    if step is not None:
        return step

    # Nothing left to break: keep moving, but not into a fire.
    options = [d for d, (dx, dy) in STEP.items()
               if world.walkable(x + dx, y + dy) and (x + dx, y + dy) not in doomed]
    return random.choice(options) if options else None


def main():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.sendto(bytes([0xFF, 0xFF]), SERVER)
    sock.settimeout(1.0)

    world = World()
    seq = 0
    print(f"saying hello to {HOST}:{PORT}")

    while world.me is None:
        try:
            data, _ = sock.recvfrom(4096)
        except socket.timeout:
            sock.sendto(bytes([0xFF, 0xFF]), SERVER)   # lobby full, or packet lost
            continue
        if data[0] == ASSIGNED:
            world.me = data[6]
            print(f"seated as player {world.me}; waiting for the moderator to start")

    sock.setblocking(False)

    while True:
        # Drain everything that arrived this tick.
        while True:
            try:
                data, _ = sock.recvfrom(4096)
            except BlockingIOError:
                break
            kind = data[0]
            if kind == MATCH_INIT:
                world.on_match_init(data)
            elif kind == KEYFRAME:
                world.on_keyframe(data)
            elif kind == DELTA:
                world.on_delta(data)
            elif kind == MATCH_END:
                winner = data[6]
                who = "nobody (draw)" if winner == NO_PLAYER else f"player {winner}"
                print(f"match over, winner: {who}")
                world.at_tick = None
                world.synced = False

        # Only act on a picture we know is current. Between losing a datagram
        # and the next keyframe we are blind, and standing still is the right
        # move -- a keyframe is at most half a second away.
        if world.synced:
            action = decide(world)
            if action is not None:
                sock.sendto(bytes([world.me, (seq << 4) | action]), SERVER)
                seq = (seq + 1) & 0x0F

        time.sleep(1 / 60)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print()
