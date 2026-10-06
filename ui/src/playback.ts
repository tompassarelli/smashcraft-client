// The viewer's controls over an opened replay: play and pause, a frame
// forward or back, and seeking. Playing runs the match's 60 frames a second.
import type { ReplayViewer } from "./replays";

const FRAMES_PER_SECOND = 60;
/** The most frames one animation tick catches up, so a stalled window doesn't run away. */
const MOST_FRAMES_PER_TICK = 6;

export class Playback {
  playing = false;
  private owed = 0;

  constructor(readonly viewer: ReplayViewer) {}

  /** Plays from the start again after the last frame. */
  toggle(): void {
    if (!this.playing && this.viewer.frame >= this.viewer.last) this.viewer.seek(this.viewer.first);
    this.playing = !this.playing;
    this.owed = 0;
  }

  pause(): void {
    this.playing = false;
  }

  stepForward(): void {
    this.pause();
    this.viewer.step();
  }

  stepBack(): void {
    this.pause();
    this.viewer.seek(this.viewer.frame - 1);
  }

  seek(frame: number): void {
    this.viewer.seek(frame);
  }

  /** Runs the frames `elapsedMs` of playing owes; whether the frame changed. Stops at the last frame. */
  tick(elapsedMs: number): boolean {
    if (!this.playing) return false;
    this.owed = Math.min(MOST_FRAMES_PER_TICK, this.owed + (elapsedMs * FRAMES_PER_SECOND) / 1000);
    let changed = false;
    while (this.owed >= 1) {
      this.owed -= 1;
      if (!this.viewer.step()) {
        this.playing = false;
        break;
      }
      changed = true;
    }
    return changed;
  }
}

/** m:ss of a frame count. */
export function clock(frames: number): string {
  const seconds = Math.floor(Math.max(0, frames) / FRAMES_PER_SECOND);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
