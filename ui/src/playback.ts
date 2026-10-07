// The viewer's controls over a watched replay: play and pause, a frame
// forward or back, and seeking. Playing runs the match's 60 frames a second;
// a replay played in its map's simulation answers asynchronously, so one run
// of frames finishes before the next starts.
import type { Watch } from "./replays";

const FRAMES_PER_SECOND = 60;
/** The most frames one animation tick catches up, so a stalled window doesn't run away. */
const MOST_FRAMES_PER_TICK = 6;

export class Playback {
  playing = false;
  private owed = 0;
  private busy = false;

  constructor(readonly watch: Watch) {}

  /** Plays from the start again after the last frame. */
  async toggle(): Promise<void> {
    if (!this.playing && this.watch.frame >= this.watch.last) await this.watch.seek(this.watch.first);
    this.playing = !this.playing;
    this.owed = 0;
  }

  pause(): void {
    this.playing = false;
  }

  async stepForward(): Promise<void> {
    this.pause();
    await this.watch.advance(1);
  }

  async stepBack(): Promise<void> {
    this.pause();
    await this.watch.seek(this.watch.frame - 1);
  }

  async seek(frame: number): Promise<void> {
    await this.watch.seek(frame);
  }

  /** Runs the frames `elapsedMs` of playing owes; whether the frame changed. Stops at the last frame. */
  async tick(elapsedMs: number): Promise<boolean> {
    if (!this.playing || this.busy) return false;
    this.owed = Math.min(MOST_FRAMES_PER_TICK, this.owed + (elapsedMs * FRAMES_PER_SECOND) / 1000);
    const frames = Math.floor(this.owed);
    if (frames === 0) return false;
    this.owed -= frames;
    const before = this.watch.frame;
    this.busy = true;
    try {
      if (!(await this.watch.advance(frames))) this.playing = false;
    } finally {
      this.busy = false;
    }
    return this.watch.frame !== before;
  }
}

/** m:ss of a frame count. */
export function clock(frames: number): string {
  const seconds = Math.floor(Math.max(0, frames) / FRAMES_PER_SECOND);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
